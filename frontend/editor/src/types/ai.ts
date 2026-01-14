export type AIOption = 'continue' | 'improve' | 'shorter' | 'longer' | 'fix' | 'zap'

export interface GenerateRequest {
  prompt: string
  option: AIOption
  command?: string
}

export interface GenerateResponse {
  text: string
  error?: string
}

export interface AIError {
  message: string
  code?: string
  details?: unknown
}

export type Agent = 'researcher' | 'refiner'

export interface PulseRequest {
  text: string
  agents: Agent[]
}

export interface PulseSuggestion {
  agent: Agent
  content: string
}

export interface PulseResponse {
  suggestions: Record<Agent, string>
}
