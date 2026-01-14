import { useEffect, useState, useRef } from 'react'
import { env } from '@/constants/env'
import type { EditorInstance } from 'novel'

export function useEditorSync(editor: EditorInstance | null) {
  const [status, setStatus] = useState<'disconnected' | 'connected' | 'connecting'>('disconnected')
  const wsRef = useRef<WebSocket | null>(null)
  const isApplyingUpdateRef = useRef(false)

  useEffect(() => {
    if (!editor) return

    const wsUrl = env.BACKEND_URL.replace('http://', 'ws://').replace('https://', 'wss://') + '/ws'
    const ws = new WebSocket(wsUrl)
    wsRef.current = ws

    ws.onopen = () => {
      setStatus('connected')
      // Request initial state
      ws.send(JSON.stringify({ type: 'get-state' }))
    }

    ws.onclose = () => {
      setStatus('disconnected')
    }

    ws.onerror = () => {
      setStatus('disconnected')
    }

    ws.onmessage = (event) => {
      const data = event.data

      // Handle binary data (Yjs updates from backend)
      if (data instanceof ArrayBuffer) {
        // For now, we'll ignore Yjs updates since we're doing JSON sync
        // In the future, you could decode and apply Yjs updates here
        console.log('Received Yjs update (not applying in JSON sync mode)')
      }
      // Handle JSON messages
      else if (typeof data === 'string') {
        try {
          const message = JSON.parse(data)
          if (message.type === 'content-update' && message.content) {
            isApplyingUpdateRef.current = true
            editor.commands.setContent(message.content)
            setTimeout(() => {
              isApplyingUpdateRef.current = false
            }, 100)
          }
        } catch (e) {
          console.error('Failed to parse WebSocket message:', e)
        }
      }
    }

    // Send editor updates to server
    const handleUpdate = () => {
      if (ws.readyState === WebSocket.OPEN && !isApplyingUpdateRef.current) {
        const content = editor.getJSON()
        ws.send(JSON.stringify({ type: 'content-update', content }))
      }
    }

    // Debounce updates to avoid spam
    let updateTimeout: NodeJS.Timeout
    editor.on('update', () => {
      clearTimeout(updateTimeout)
      updateTimeout = setTimeout(handleUpdate, 300)
    })

    return () => {
      clearTimeout(updateTimeout)
      ws.close()
      editor.off('update', handleUpdate)
    }
  }, [editor])

  return { status }
}

