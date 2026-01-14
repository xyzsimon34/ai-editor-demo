import { useEffect, useState, useRef } from 'react'
import * as Y from 'yjs'
import { env } from '@/constants/env'

export function useYjsCollaboration(ydoc: Y.Doc) {
  const [status, setStatus] = useState<'disconnected' | 'connected' | 'connecting'>('disconnected')
  const wsRef = useRef<WebSocket | null>(null)

  useEffect(() => {
    const wsUrl = env.BACKEND_URL.replace('http://', 'ws://').replace('https://', 'wss://') + '/ws'
    const ws = new WebSocket(wsUrl)
    ws.binaryType = 'arraybuffer'
    wsRef.current = ws

    ws.onopen = () => {
      setStatus('connected')
    }

    ws.onclose = () => {
      setStatus('disconnected')
    }

    ws.onerror = () => {
      setStatus('disconnected')
    }

    ws.onmessage = (event) => {
      const data = event.data

      // Binary Sync (Y.js Update)
      if (data instanceof ArrayBuffer) {
        Y.applyUpdate(ydoc, new Uint8Array(data), 'websocket')
      }
      // Future AI Commands (JSON)
      else if (typeof data === 'string') {
        console.log('Received AI Command:', data)
      }
    }

    // Capture User Typing -> Send to Backend
    const updateHandler = (update: Uint8Array, origin: any) => {
      if (ws.readyState === WebSocket.OPEN && origin !== 'websocket') {
        ws.send(update)
      }
    }

    ydoc.on('update', updateHandler)

    return () => {
      ws.close()
      ydoc.off('update', updateHandler)
    }
  }, [ydoc])

  return { status }
}

