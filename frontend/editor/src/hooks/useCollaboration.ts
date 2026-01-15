import { useCallback, useEffect, useRef, useState } from 'react'
import * as Y from 'yjs'

import { env } from '@/constants/env'

// Interfaces
interface AiCommandPayload {
  type: 'AI_COMMAND'
  action: string
  payload?: Record<string, unknown>
}

interface UseCollaborationReturn {
  status: ConnectionStatus
  runAiCommand: (action: string, payload?: Record<string, unknown>) => void
}

type ConnectionStatus = 'disconnected' | 'connected' | 'connecting'

// Constants
const RECONNECT_DELAY_MS = 3000
const CLEAN_CLOSE_CODE = 1000

export function useCollaboration(ydoc: Y.Doc): UseCollaborationReturn {
  const [status, setStatus] = useState<ConnectionStatus>('disconnected')
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  const runAiCommand = useCallback((action: string, payload?: Record<string, unknown>) => {
    const ws = wsRef.current
    const isWebSocketOpen = ws?.readyState === WebSocket.OPEN

    if (!isWebSocketOpen || !ws) {
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
      const isWebSocketOpen = ws?.readyState === WebSocket.OPEN
      const isLocalUpdate = origin !== 'websocket'

      if (isWebSocketOpen && isLocalUpdate && ws) {
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
          // eslint-disable-next-line no-console
          console.log('Received AI Command (string):', data)
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

  return { status, runAiCommand }
}

// Helpers
function buildWebSocketUrl(backendUrl: string): string {
  return backendUrl.replace('http://', 'ws://').replace('https://', 'wss://') + '/ws'
}
