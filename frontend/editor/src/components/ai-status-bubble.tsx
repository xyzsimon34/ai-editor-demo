'use client'

import React, { useEffect, useState } from 'react'

const THINKING_MESSAGES = ['🤔 讓我想想...', '☕ 先喝口咖啡...', '🔥 腦細胞燃燒中...', '👀 偷看你的文字中...']

const DONE_MESSAGES = ['✨ 搞定！不用謝', '🎉 完成！我真棒', '💪 寫完了，快誇我']

function getRandomMessage(messages: string[]): string {
  return messages[Math.floor(Math.random() * messages.length)]
}

interface AIStatusBubbleProps {
  status: 'idle' | 'thinking' | 'done' | 'error' | 'complete'
  className?: string
}

export function AIStatusBubble({ status, className = '' }: AIStatusBubbleProps) {
  const [message, setMessage] = useState('')
  const [isVisible, setIsVisible] = useState(false)

  useEffect(() => {
    if (status === 'thinking') {
      setMessage(getRandomMessage(THINKING_MESSAGES))
      setIsVisible(true)
    } else if (status === 'done' || status === 'complete') {
      setMessage(getRandomMessage(DONE_MESSAGES))
      // Auto-hide after 2 seconds
      const timer = setTimeout(() => setIsVisible(false), 2000)
      return () => clearTimeout(timer)
    } else if (status === 'error') {
      setMessage('😱 出錯了，稍後再試試')
      const timer = setTimeout(() => setIsVisible(false), 3000)
      return () => clearTimeout(timer)
    } else {
      setIsVisible(false)
    }
  }, [status])

  if (!isVisible) return null

  return (
    <div
      className={`fixed right-6 top-20 z-50 animate-bounce rounded-2xl border border-zinc-700 bg-zinc-800/95 px-4 py-3 shadow-xl backdrop-blur-sm ${status === 'done' ? 'border-green-500/50' : 'border-blue-500/50'} ${className} `}
    >
      <div className={'flex items-center gap-2'}>
        <span className={'text-base'}>{message}</span>
        {status === 'thinking' && <span className={'animate-pulse text-zinc-400'}>{'...'}</span>}
      </div>
    </div>
  )
}
