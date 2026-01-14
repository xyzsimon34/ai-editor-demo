import { useEffect, useState, useRef } from 'react'
import * as Y from 'yjs'
import { env } from '@/constants/env'

export function useCollaboration(ydoc: Y.Doc) {
  const [status, setStatus] = useState<'disconnected' | 'connected' | 'connecting'>('disconnected')
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  useEffect(() => {
    // Capture User Typing -> Send to Backend
    const updateHandler = (update: Uint8Array, origin: any) => {
      if (wsRef.current?.readyState === WebSocket.OPEN && origin !== 'websocket') {
        wsRef.current.send(update)
      }
    }

    const connect = () => {
      const wsUrl = env.BACKEND_URL.replace('http://', 'ws://').replace('https://', 'wss://') + '/ws'
      console.log('Connecting to WebSocket:', wsUrl)
      setStatus('connecting')
      
      const ws = new WebSocket(wsUrl)
      ws.binaryType = 'arraybuffer'
      wsRef.current = ws

      ws.onopen = () => {
        console.log('WebSocket connected successfully')
        setStatus('connected')
        // Clear any pending reconnect
        if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current)
          reconnectTimeoutRef.current = null
        }
      }

      ws.onclose = (event) => {
        console.log('WebSocket closed:', event.code, event.reason || 'No reason')
        setStatus('disconnected')
        
        // Only reconnect if it wasn't a clean close (code 1000)
        if (event.code !== 1000 && !reconnectTimeoutRef.current) {
          console.log('Attempting to reconnect in 3 seconds...')
          reconnectTimeoutRef.current = setTimeout(() => {
            reconnectTimeoutRef.current = null
            connect()
          }, 3000)
        }
      }

      ws.onerror = (error) => {
        console.error('WebSocket error:', error)
        // Error details are usually in the Event, not the error parameter
        console.error('WebSocket failed to connect. Is the backend running on', env.BACKEND_URL, '?')
        setStatus('disconnected')
      }

      ws.onmessage = (event) => {
        const data = event.data

        // LANE A: Binary Sync (Y.js Update)
        if (data instanceof ArrayBuffer) {
          // Apply the update from Backend to our local Yjs Doc
          Y.applyUpdate(ydoc, new Uint8Array(data), 'websocket')
        }
        // LANE B: AI Commands (Text/JSON)
        else if (typeof data === 'string') {
          console.log('Received AI Command (string):', data)
        }
        else {
          // Debug: log what we actually received
          console.log('Received unknown message type:', typeof data, data)
        }
      }
    }

    // Subscribe to local Yjs changes
    ydoc.on('update', updateHandler)

    connect()

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
      }
      if (wsRef.current) {
        wsRef.current.close()
      }
      ydoc.off('update', updateHandler)
    }
  }, [ydoc])

  return { status }
}

