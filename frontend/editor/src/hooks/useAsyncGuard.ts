import { useRef, useCallback } from 'react'

interface AsyncGuard {
  /**
   * Runs an async action and provides a check to see if it's still the latest request.
   * @param action A function that receives an `isStale` check.
   */
  run: <T>(action: (isStale: () => boolean) => Promise<T>) => Promise<T | undefined>
  /**
   * Manually marks the current request as stale.
   */
  cancel: () => void
  /**
   * Checks if a given ID is still the latest one.
   */
  isLatest: (id: number) => boolean
  /**
   * Gets the current request ID.
   */
  currentId: () => number
  /**
   * Generates a new request ID.
   */
  nextId: () => number
}

/**
 * A hook to prevent race conditions in async operations.
 * Following the "Guard" pattern to ensure only the latest request's side effects are executed.
 */
export function useAsyncGuard(): AsyncGuard {
  const lastIdRef = useRef<number>(0)

  const currentId = useCallback(() => lastIdRef.current, [])

  const nextId = useCallback(() => {
    lastIdRef.current = Date.now()
    return lastIdRef.current
  }, [])

  const isLatest = useCallback((id: number) => id === lastIdRef.current, [])

  const cancel = useCallback(() => {
    lastIdRef.current = 0
  }, [])

  const run = useCallback(async <T>(action: (isStale: () => boolean) => Promise<T>): Promise<T | undefined> => {
    const executionId = ++lastIdRef.current
    const isStale = () => executionId !== lastIdRef.current

    try {
      return await action(isStale)
    } catch (error) {
      if (!isStale()) throw error
      return undefined
    }
  }, [])

  return { run, cancel, isLatest, nextId, currentId }
}
