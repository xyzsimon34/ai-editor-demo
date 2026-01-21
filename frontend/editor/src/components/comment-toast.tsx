'use client'

import { useEffect, useState } from 'react'

import type { BackseaterComment } from '@/hooks/useCollaboration'

interface CommentToastProps {
  comment: BackseaterComment | null
}

export function CommentToast({ comment }: CommentToastProps) {
  const [isVisible, setIsVisible] = useState(false)

  useEffect(() => {
    if (comment) {
      setIsVisible(true)
      const timer = setTimeout(() => setIsVisible(false), 5000)
      return () => clearTimeout(timer)
    } else {
      setIsVisible(false)
    }
  }, [comment])

  if (!comment || !isVisible) return null

  return (
    <div
      className={'fixed right-6 top-32 z-50 max-w-sm rounded-lg border bg-zinc-800/95 p-4 shadow-lg backdrop-blur-sm'}
      style={{ borderColor: comment.color_hex || '#fbbf24' }}
    >
      <div className={'space-y-2'}>
        <div className={'text-xs font-semibold text-zinc-400'}>{'💬 Comment on:'}</div>
        <div className={'text-sm italic text-zinc-300'}>{`"${comment.comment_on}"`}</div>
        <div className={'text-sm text-zinc-200'}>{comment.comment}</div>
      </div>
    </div>
  )
}
