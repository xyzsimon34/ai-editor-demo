export type AIGenerationOption = 'continue' | 'zap'

export interface GenerateTextRequest {
  prompt: string
  option: AIGenerationOption
  command?: string
}

export async function generateText(request: GenerateTextRequest): Promise<Response> {
  const endpoint = '/api/generate'

  const response = await fetch(endpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  })

  if (!response.ok) {
    const errorText = await response.text().catch(() => response.statusText)
    throw new Error(`AI generation failed: ${errorText}`)
  }

  return response
}
