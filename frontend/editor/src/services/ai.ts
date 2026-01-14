import type { GenerateRequest, PulseRequest, PulseResponse } from '@/types/ai'

import { env } from '@/constants/env'

export async function refineText(text: string, action: 'improve' | 'fix' | 'longer' | 'shorter'): Promise<string> {
  const endpoint = `${env.BACKEND_API_URL}/${action}`

  const response = await fetch(endpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ text })
  })

  if (!response.ok) {
    const errorText = await response.text().catch(() => response.statusText)
    throw new Error(`Backend refine API failed: ${errorText}`)
  }

  const data = await response.json()
  return data.text
}

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
 * Get AI-powered suggestions from multiple specialized agents
 * @param request - Pulse request containing text and selected agents
 * @returns Suggestions from each agent
 */
export async function getPulseSuggestions(request: PulseRequest): Promise<PulseResponse> {
  const endpoint = '/api/pulse'

  const response = await fetch(endpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  })

  if (!response.ok) {
    const errorText = await response.text().catch(() => response.statusText)
    throw new Error(`Pulse API failed: ${errorText}`)
  }

  const data: PulseResponse = await response.json()
  return data
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
