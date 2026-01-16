import { useCallback, useEffect, useRef, useState } from 'react'
import * as Y from 'yjs'

import { env } from '@/constants/env'

// Interfaces
type AiPayload = Record<string, unknown> | string | number | boolean | null

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

type AIStatus = 'idle' | 'thinking' | 'done'

interface UseCollaborationReturn {
  status: ConnectionStatus
  aiStatus: AIStatus
  runAiCommand: (action: string, payload?: AiPayload) => void
}

interface BackseaterComment {
  type: 'COMMENT'
  comment_on: string
  comment: string
  color_hex: string
}

type ConnectionStatus = 'disconnected' | 'connected' | 'connecting'

// Constants
const RECONNECT_DELAY_MS = 3000
const CLEAN_CLOSE_CODE = 1000

export function useCollaboration(ydoc: Y.Doc): UseCollaborationReturn {
  const [status, setStatus] = useState<ConnectionStatus>('disconnected')
  const [aiStatus, setAiStatus] = useState<AIStatus>('idle')
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  const runAiCommand = useCallback((action: string, payload?: AiPayload) => {
    const ws = wsRef.current
    if (!ws || !isWebSocketOpen(ws)) {
      console.warn('Cannot send AI command: Socket not open')
      return
    }

    const message: AiCommandPayload = {
      type: 'AI_COMMAND',
      action,
      payload
    }

    ws.send(JSON.stringify(message))
  }, [])

  useEffect(() => {
    const handleYjsUpdate = (update: Uint8Array, origin: unknown) => {
      const ws = wsRef.current
      const isLocalUpdate = origin !== 'websocket'

      if (ws && isWebSocketOpen(ws) && isLocalUpdate) {
        ws.send(update)
      }
    }

    const clearReconnectTimeout = () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
        reconnectTimeoutRef.current = null
      }
    }

    const scheduleReconnect = (connect: () => void) => {
      // eslint-disable-next-line no-console
      console.log(`Attempting to reconnect in ${RECONNECT_DELAY_MS / 1000} seconds...`)
      reconnectTimeoutRef.current = setTimeout(() => {
        reconnectTimeoutRef.current = null
        connect()
      }, RECONNECT_DELAY_MS)
    }

    const connect = () => {
      const wsUrl = buildWebSocketUrl(env.BACKEND_URL)
      // eslint-disable-next-line no-console
      console.log('Connecting to WebSocket:', wsUrl)
      setStatus('connecting')

      const ws = new WebSocket(wsUrl)
      ws.binaryType = 'arraybuffer'
      wsRef.current = ws

      ws.onopen = () => {
        // eslint-disable-next-line no-console
        console.log('WebSocket connected successfully')
        setStatus('connected')
        clearReconnectTimeout()
      }

      ws.onclose = (event) => {
        // eslint-disable-next-line no-console
        console.log('WebSocket closed:', event.code, event.reason || 'No reason')
        setStatus('disconnected')

        const isCleanClose = event.code === CLEAN_CLOSE_CODE
        const hasNoScheduledReconnect = !reconnectTimeoutRef.current

        if (!isCleanClose && hasNoScheduledReconnect) {
          scheduleReconnect(connect)
        }
      }

      ws.onerror = (error) => {
        // eslint-disable-next-line no-console
        console.error('WebSocket error:', error)
        // eslint-disable-next-line no-console
        console.error('WebSocket failed to connect. Is the backend running on', env.BACKEND_URL, '?')
        setStatus('disconnected')
      }

      ws.onmessage = (event) => {
        const data = event.data

        if (data instanceof ArrayBuffer) {
          Y.applyUpdate(ydoc, new Uint8Array(data), 'websocket')
        } else if (typeof data === 'string') {
          try {
            const parsed = JSON.parse(data)
            if (parsed.type === 'AI_STATUS') {
              setAiStatus((parsed as AIStatusMessage).status)
            } else if (parsed.type === 'COMMENT') {
              const comment = parsed as BackseaterComment
              console.log('Received COMMENT:', comment)
            } else {
              console.log('Received unknown message type:', parsed.type, parsed)
            }
          } catch {
            // eslint-disable-next-line no-console
            console.log('Received non-JSON string:', data)
          }
        } else {
          // eslint-disable-next-line no-console
          console.log('Received unknown message type:', typeof data, data)
        }
      }
    }

    ydoc.on('update', handleYjsUpdate)
    connect()

    return () => {
      clearReconnectTimeout()
      if (wsRef.current) {
        wsRef.current.close()
      }
      ydoc.off('update', handleYjsUpdate)
    }
  }, [ydoc])

  return { status, aiStatus, runAiCommand }
}

// Helpers
function buildWebSocketUrl(backendUrl: string): string {
  return backendUrl.replace('http://', 'ws://').replace('https://', 'wss://') + '/ws'
}

function isWebSocketOpen(socket: WebSocket | null): boolean {
  return socket?.readyState === WebSocket.OPEN
}
