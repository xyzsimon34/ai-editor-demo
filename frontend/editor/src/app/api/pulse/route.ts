import { NextRequest, NextResponse } from 'next/server'
import type { PulseRequest, PulseResponse } from '@/types/ai'

import { env } from '@/constants/env'

export async function POST(request: NextRequest) {
  try {
    const body: PulseRequest = await request.json()

    if (!body.text || !body.agents || body.agents.length === 0) {
      return NextResponse.json({ error: 'Invalid request: text and agents are required' }, { status: 400 })
    }

    const backendUrl = `${env.BACKEND_API_URL}/agent/pulse`
    const response = await fetch(backendUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(body)
    })

    if (!response.ok) {
      const errorText = await response.text().catch(() => response.statusText)
      throw new Error(`Backend pulse API failed: ${errorText}`)
    }

    const data: PulseResponse = await response.json()

    return NextResponse.json(data)
  } catch (error) {
    console.error('Pulse API error:', error)
    return NextResponse.json(
      {
        error: error instanceof Error ? error.message : 'Unknown error occurred'
      },
      { status: 500 }
    )
  }
}
