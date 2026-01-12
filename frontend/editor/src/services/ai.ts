import type { GenerateRequest } from '@/types/ai'

/**
 *
 *
 * @param request
 * @returns
 */
export async function generateText(request: GenerateRequest): Promise<Response> {
  // TODO: `${env.API_ENDPOINT}/generate`
  const endpoint = '/api/generate'

  const response = await fetch(endpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  })

  if (!response.ok) {
    throw new Error(`AI generation failed: ${response.statusText}`)
  }

  return response
}

/**
 *
 *
 * export async function generateText(request: GenerateRequest): Promise<Response> {
 *   const endpoint = `${env.API_ENDPOINT}/generate`
 *
 *   const response = await fetch(endpoint, {
 *     method: 'POST',
 *     headers: {
 *       'Content-Type': 'application/json',
 *       'Authorization': `Bearer ${getAuthToken()}`
 *     },
 *     body: JSON.stringify(request)
 *   })
 *
 *   return response
 * }
 */
