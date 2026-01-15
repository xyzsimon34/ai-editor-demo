import type { PulseRequest, PulseResponse } from '@/types/ai'

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
