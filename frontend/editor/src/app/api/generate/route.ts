import { openai } from '@ai-sdk/openai'
import { streamText } from 'ai'

import { env } from '@/constants/env'

export const runtime = 'edge'

export async function POST(req: Request): Promise<Response> {
  try {
    const { prompt, option, command } = await req.json()

    if (!env.OPENAI_API_KEY || env.OPENAI_API_KEY === '') {
      return new Response('Missing OPENAI_API_KEY - make sure to add it to your .env.local file.', {
        status: 400
      })
    }

    let messages: Array<{ role: 'system' | 'user'; content: string }> = []

    switch (option) {
      case 'continue':
        messages = [
          {
            role: 'system',
            content:
              'You are an AI writing assistant that continues existing text based on context from prior text. ' +
              'Give more weight/priority to the later characters than the beginning ones. ' +
              'Limit your response to no more than 200 characters, but make sure to construct complete sentences. ' +
              'Use Markdown formatting when appropriate.'
          },
          {
            role: 'user',
            content: prompt
          }
        ]
        break

      case 'zap':
        messages = [
          {
            role: 'system',
            content:
              'You are an AI writing assistant that generates text based on a prompt. ' +
              'You take an input from the user and a command for manipulating the text. ' +
              'Use Markdown formatting when appropriate.'
          },
          {
            role: 'user',
            content: `For this text: ${prompt}. You have to respect the command: ${command}`
          }
        ]
        break

      default:
        return new Response(
          JSON.stringify({ 
            error: `Unknown option: ${option}. This API only supports 'continue' and 'zap'. For text refinement (improve/fix/longer/shorter), use backend API directly.` 
          }),
          {
            status: 400,
            headers: { 'Content-Type': 'application/json' }
          }
        )
    }

    const result = streamText({
      model: openai('gpt-4o-mini'),
      messages: messages
    })

    return result.toUIMessageStreamResponse()
  } catch (error) {
    console.error('Error in generate API:', error)
    return new Response(
      JSON.stringify({ 
        error: error instanceof Error ? error.message : 'Unknown error' 
      }), 
      {
        status: 500,
        headers: { 'Content-Type': 'application/json' }
      }
    )
  }
}
