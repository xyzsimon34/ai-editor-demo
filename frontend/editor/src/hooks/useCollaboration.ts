import { useCallback, useEffect, useRef, useState } from 'react'
import * as Y from 'yjs'

import { env } from '@/constants/env'

interface AiCommandPayload {
  type: 'AI_COMMAND'
  action: string
  payload?: AiPayload
}

interface AIStatusMessage {
  type: 'AI_STATUS'
  status: 'thinking' | 'done'
  message: string
}

interface SyncCompleteMessage {
  type: 'SYNC_COMPLETE'
}

interface UseCollaborationReturn {
  status: ConnectionStatus
  aiStatus: AIStatus
  isServerSynced: boolean
  runAiCommand: (action: string, payload?: AiPayload) => void
}

interface BackseaterComment {
  type: 'COMMENT'
  comment_on: string
  comment: string
  color_hex: string
}

type AiPayload = Record<string, unknown> | string | number | boolean | null
type AIStatus = 'idle' | 'thinking' | 'done'
type ConnectionStatus = 'disconnected' | 'connected' | 'connecting'
type WebSocketMessage = AIStatusMessage | SyncCompleteMessage

const RECONNECT_DELAY_MS = 3000
const CLEAN_CLOSE_CODE = 1000

function buildWebSocketUrl(backendUrl: string): string {
  return backendUrl.replace('http://', 'ws://').replace('https://', 'wss://') + '/ws'
}

function isWebSocketOpen(socket: WebSocket | null): boolean {
  return socket?.readyState === WebSocket.OPEN
}

export function useCollaboration(ydoc: Y.Doc, isLocalSynced: boolean): UseCollaborationReturn {
  const [status, setStatus] = useState<ConnectionStatus>('disconnected')
  const [aiStatus, setAiStatus] = useState<AIStatus>('idle')
  const [isServerSynced, setIsServerSynced] = useState(false)
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const hasReceivedFirstUpdate = useRef(false)

  const runAiCommand = useCallback((action: string, payload?: AiPayload) => {
    const ws = wsRef.current
    if (!ws || !isWebSocketOpen(ws)) return

    const message: AiCommandPayload = { type: 'AI_COMMAND', action, payload }
    ws.send(JSON.stringify(message))
  }, [])

  useEffect(() => {
    if (!isLocalSynced) return

    const clearReconnectTimeout = () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
        reconnectTimeoutRef.current = null
      }
    }

    const scheduleReconnect = (connectFn: () => void) => {
      reconnectTimeoutRef.current = setTimeout(() => {
        reconnectTimeoutRef.current = null
        connectFn()
      }, RECONNECT_DELAY_MS)
    }

    const handleYjsUpdate = (update: Uint8Array, origin: unknown) => {
      const ws = wsRef.current
      if (ws && isWebSocketOpen(ws) && origin !== 'websocket') {
        ws.send(update)
      }
    }

    const handleBinaryMessage = (data: ArrayBuffer) => {
      Y.applyUpdate(ydoc, new Uint8Array(data), 'websocket')
      if (!hasReceivedFirstUpdate.current) {
        hasReceivedFirstUpdate.current = true
        setIsServerSynced(true)
      }
    }

    const handleJsonMessage = (data: string) => {
      try {
        const parsed = JSON.parse(data) as WebSocketMessage
        if (parsed.type === 'AI_STATUS') setAiStatus(parsed.status)
        else if (parsed.type === 'SYNC_COMPLETE') setIsServerSynced(true)
      } catch {
        // Ignore non-JSON messages
      }
    }

    const connect = () => {
      setStatus('connecting')
      const ws = new WebSocket(buildWebSocketUrl(env.BACKEND_URL))
      ws.binaryType = 'arraybuffer'
      wsRef.current = ws

      ws.onopen = () => {
        setStatus('connected')
        clearReconnectTimeout()
        ws.send(Y.encodeStateAsUpdate(ydoc))
      }

      ws.onclose = (event) => {
        setStatus('disconnected')
        if (event.code !== CLEAN_CLOSE_CODE && !reconnectTimeoutRef.current) {
          scheduleReconnect(connect)
        }
      }

      ws.onerror = () => setStatus('disconnected')

      ws.onmessage = (event) => {
        if (event.data instanceof ArrayBuffer) handleBinaryMessage(event.data)
        else if (typeof event.data === 'string') handleJsonMessage(event.data)
      }
    }

    ydoc.on('update', handleYjsUpdate)
    connect()

    return () => {
      clearReconnectTimeout()
      wsRef.current?.close()
      ydoc.off('update', handleYjsUpdate)
      hasReceivedFirstUpdate.current = false
      setIsServerSynced(false)
    }
  }, [ydoc, isLocalSynced])

  return { status, aiStatus, isServerSynced, runAiCommand }
}
