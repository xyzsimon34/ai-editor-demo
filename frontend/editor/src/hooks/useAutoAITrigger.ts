import { useCallback, useEffect, useRef, useState } from 'react'
import type { EditorInstance } from 'novel'

export interface AutoAITriggerOptions {
  enabled: boolean
  debounceMs?: number
  minCharacters?: number
  minChangeThreshold?: number
  onTrigger: () => void
}

interface TriggerState {
  lastContentLength: number
  lastContent: string
}

interface ContentSnapshot {
  content: string
  charCount: number
}

export function useAutoAITrigger(editor: EditorInstance | null, options: AutoAITriggerOptions) {
  const { enabled, debounceMs = 3000, minCharacters = 3, minChangeThreshold = 50, onTrigger } = options

  const [isPending, setIsPending] = useState(false)
  const [remainingTime, setRemainingTime] = useState<number | null>(null)

  const timerRef = useRef<NodeJS.Timeout | null>(null)
  const countdownRef = useRef<NodeJS.Timeout | null>(null)
  const isExecutingRef = useRef(false)
  const lastTriggerTimeRef = useRef<number>(0)
  const stateRef = useRef<TriggerState>({
    lastContentLength: 0,
    lastContent: ''
  })

  // Keep callback ref stable
  const onTriggerRef = useRef(onTrigger)

  useEffect(() => {
    onTriggerRef.current = onTrigger
  }, [onTrigger])

  const clearTimers = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current)
      timerRef.current = null
    }
    if (countdownRef.current) {
      clearInterval(countdownRef.current)
      countdownRef.current = null
    }
    setIsPending(false)
    setRemainingTime(null)
  }, [])

  const shouldTriggerAI = useCallback(
    (snapshot: ContentSnapshot): boolean => {
      const { content, charCount } = snapshot
      const { lastContentLength, lastContent } = stateRef.current

      const delta = charCount - lastContentLength
      const magnitude = Math.abs(delta)

      const hasEnoughChars = charCount >= minCharacters || delta < 0

      if (!hasEnoughChars) {
        return false
      }

      if (magnitude < minChangeThreshold) {
        return false
      }

      if (content === lastContent) {
        return false
      }

      return true
    },
    [minCharacters, minChangeThreshold]
  )

  const scheduleAITrigger = useCallback(() => {
    if (!editor || !enabled) {
      return
    }

    if (isExecutingRef.current) {
      return
    }

    const now = Date.now()
    const timeSinceLastTrigger = now - lastTriggerTimeRef.current
    if (timeSinceLastTrigger < 2000) {
      return
    }

    const scheduleSnapshot = getContentSnapshot(editor)

    if (scheduleSnapshot.content === stateRef.current.lastContent) {
      return
    }

    const shouldSchedule = shouldTriggerAI(scheduleSnapshot)

    if (!shouldSchedule) {
      return
    }

    clearTimers()
    setIsPending(true)

    let remaining = Math.ceil(debounceMs / 1000)
    setRemainingTime(remaining)

    countdownRef.current = setInterval(() => {
      remaining -= 1
      setRemainingTime(remaining > 0 ? remaining : null)
      if (remaining <= 0 && countdownRef.current) {
        clearInterval(countdownRef.current)
        countdownRef.current = null
      }
    }, 1000)

    timerRef.current = setTimeout(() => {
      const triggerSnapshot = getContentSnapshot(editor)

      if (triggerSnapshot.content !== scheduleSnapshot.content) {
        clearTimers()
        return
      }

      stateRef.current = {
        lastContentLength: triggerSnapshot.charCount,
        lastContent: triggerSnapshot.content
      }
      isExecutingRef.current = true
      lastTriggerTimeRef.current = Date.now()

      clearTimers()

      try {
        onTriggerRef.current()
      } finally {
        setTimeout(() => {
          isExecutingRef.current = false
        }, 1500)
      }
    }, debounceMs)
  }, [editor, enabled, debounceMs, shouldTriggerAI, clearTimers])

  const cancelScheduled = useCallback(() => {
    clearTimers()
  }, [clearTimers])

  useEffect(() => {
    return () => {
      clearTimers()
    }
  }, [clearTimers])

  return {
    scheduleAITrigger,
    cancelScheduled,
    isPending,
    remainingTime
  }
}

function getContentSnapshot(editor: EditorInstance): ContentSnapshot {
  // Extract text excluding pending AI suggestions (to prevent loop)
  const state = editor.state
  let content = ''
  let charCount = 0

  state.doc.descendants((node, pos) => {
    if (node.isText && node.text) {
      // Check if this text node has a pending aisuggestion mark
      const hasPendingAISuggestion = node.marks.some(
        (mark) => mark.type.name === 'aisuggestion' && mark.attrs?.status === 'pending'
      )
      
      // Only include text that doesn't have pending AI suggestions
      if (!hasPendingAISuggestion) {
        content += node.text
        charCount += node.text.length
      }
    }
  })

  return { content, charCount }
}
