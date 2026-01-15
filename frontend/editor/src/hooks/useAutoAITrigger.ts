import { useCallback, useEffect, useRef, useState } from 'react'
import type { EditorInstance } from 'novel'

export interface AutoAITriggerOptions {
  enabled: boolean
  debounceMs?: number
  minCharacters?: number
  minChangeThreshold?: number
  cooldownMs?: number
  onTrigger: () => void
}

interface TriggerState {
  lastTriggerTime: number
  lastContentLength: number
  lastContent: string
}

export function useAutoAITrigger(editor: EditorInstance | null, options: AutoAITriggerOptions) {
  const {
    enabled,
    debounceMs = 3000,
    minCharacters = 3,
    minChangeThreshold = 50,
    cooldownMs = 30000,
    onTrigger
  } = options

  const [isPending, setIsPending] = useState(false)
  const [remainingTime, setRemainingTime] = useState<number | null>(null)

  const timerRef = useRef<NodeJS.Timeout | null>(null)
  const countdownRef = useRef<NodeJS.Timeout | null>(null)
  const stateRef = useRef<TriggerState>({
    lastTriggerTime: 0,
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
    (content: string, charCount: number): boolean => {
      if (charCount < minCharacters) {
        return false
      }

      return true
    },
    [minCharacters]
  )

  const scheduleAITrigger = useCallback(() => {
    if (!editor || !enabled) {
      return
    }

    const content = editor.getText()
    const charCount = editor.storage.characterCount?.characters() || content.length

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
      const shouldTrigger = shouldTriggerAI(content, charCount)

      if (!shouldTrigger) {
        clearTimers()
        return
      }

      stateRef.current = {
        lastTriggerTime: Date.now(),
        lastContentLength: charCount,
        lastContent: content
      }

      clearTimers()
      onTriggerRef.current()
    }, debounceMs)
  }, [editor, enabled, debounceMs, shouldTriggerAI, clearTimers, minCharacters, minChangeThreshold])

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
