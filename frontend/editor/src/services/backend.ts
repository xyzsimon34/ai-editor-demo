import { env } from '@/constants/env'

export type RefineAction = 'improve' | 'fix' | 'longer' | 'shorter'

interface RefineTextRequest {
  text: string
}

interface RefineTextResponse {
  text: string
}

export async function refineText(text: string, action: RefineAction): Promise<string> {
  const endpoint = `${env.BACKEND_URL}/${action}`

  const response = await fetch(endpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ text } satisfies RefineTextRequest)
  })

  if (!response.ok) {
    const errorText = await response.text().catch(() => response.statusText)
    throw new Error(`Backend API failed (${action}): ${errorText}`)
  }

  const data: RefineTextResponse = await response.json()
  return data.text
}
