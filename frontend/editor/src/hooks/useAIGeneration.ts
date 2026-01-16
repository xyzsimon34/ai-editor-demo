import { useCallback, useState } from 'react'
import { refineText, type RefineAction } from '@/services/backend'
import type { AIOption } from '@/types/ai'
import { useCompletion } from '@ai-sdk/react'
import { useAsyncGuard } from './useAsyncGuard'

export interface UseAIGenerationOptions {
  onFinish?: () => void
  onError?: (error: Error) => void
}

export interface UseAIGenerationReturn {
  completion: string
  isLoading: boolean
  generate: (prompt: string, options: { option: AIOption; command?: string }) => Promise<void>
}

export function useAIGeneration(options?: UseAIGenerationOptions): UseAIGenerationReturn {
  const [backendCompletion, setBackendCompletion] = useState('')
  const [backendLoading, setBackendLoading] = useState(false)
  const asyncGuard = useAsyncGuard()

  const {
    completion: openaiCompletion,
    complete,
    isLoading: openaiLoading
  } = useCompletion({
    api: '/api/generate',
    onFinish: options?.onFinish,
    onError: options?.onError
  })

  const generate = useCallback(
    async (prompt: string, generateOptions: { option: AIOption; command?: string }): Promise<void> => {
      const { option, command } = generateOptions

      if (option === 'improve' || option === 'fix' || option === 'longer' || option === 'shorter') {
        setBackendLoading(true)
        setBackendCompletion('')

        await asyncGuard.run(async (isStale) => {
          try {
            const result = await refineText(prompt, option as RefineAction)

            let currentText = ''
            for (let i = 0; i < result.length; i++) {
              if (isStale()) return

              currentText += result[i]
              setBackendCompletion(currentText)
              await new Promise((resolve) => setTimeout(resolve, 10))
            }

            if (!isStale()) {
              options?.onFinish?.()
            }
          } catch (error) {
            if (!isStale()) {
              const err = error instanceof Error ? error : new Error('Unknown error')
              options?.onError?.(err)
              throw err
            }
          } finally {
            if (!isStale()) {
              setBackendLoading(false)
            }
          }
        })
      } else if (option === 'continue' || option === 'zap') {
        setBackendCompletion('')
        await complete(prompt, {
          body: {
            option,
            command
          }
        })
      } else {
        const error = new Error(`Unknown option: ${option}`)
        options?.onError?.(error)
        throw error
      }
    },
    [complete, options]
  )

  return {
    completion: backendCompletion || openaiCompletion,
    isLoading: backendLoading || openaiLoading,
    generate
  }
}
