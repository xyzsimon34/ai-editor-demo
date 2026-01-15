import { useState, useCallback } from 'react'

import type { Agent, PulseRequest, PulseResponse } from '@/types/ai'

import { getPulseSuggestions } from '@/services'

interface UsePulseOptions {
  onSuccess?: (data: PulseResponse) => void
  onError?: (error: Error) => void
}

interface UsePulseReturn {
  suggestions: PulseResponse | null
  isLoading: boolean
  error: Error | null
  getPulse: (text: string, agents: Agent[]) => Promise<void>
  reset: () => void
}

/**
 * React hook for using the Pulse API (intelligent suggestion system)
 * 
 * @example
 * ```tsx
 * const { suggestions, isLoading, error, getPulse } = usePulse()
 * 
 * // Get suggestions from both agents
 * await getPulse('Your text here', ['researcher', 'refiner'])
 * 
 * // Access suggestions
 * if (suggestions) {
 *   console.log(suggestions.suggestions.researcher)
 *   console.log(suggestions.suggestions.refiner)
 * }
 * ```
 */
export function usePulse(options?: UsePulseOptions): UsePulseReturn {
  const [suggestions, setSuggestions] = useState<PulseResponse | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  const getPulse = useCallback(
    async (text: string, agents: Agent[]) => {
      if (!text.trim()) {
        setError(new Error('Text cannot be empty'))
        return
      }

      if (agents.length === 0) {
        setError(new Error('At least one agent must be selected'))
        return
      }

      setIsLoading(true)
      setError(null)

      try {
        const request: PulseRequest = { text, agents }
        const response = await getPulseSuggestions(request)
        
        setSuggestions(response)
        options?.onSuccess?.(response)
      } catch (err) {
        const error = err instanceof Error ? err : new Error('Unknown error')
        setError(error)
        options?.onError?.(error)
      } finally {
        setIsLoading(false)
      }
    },
    [options]
  )

  const reset = useCallback(() => {
    setSuggestions(null)
    setError(null)
    setIsLoading(false)
  }, [])

  return {
    suggestions,
    isLoading,
    error,
    getPulse,
    reset
  }
}
