'use client'

import { type HTMLAttributes, type ReactNode } from 'react'
import { FileText } from 'lucide-react'

import { cn } from '@/lib/utils'

export const Header = ({
  className,
  children,
  saveStatus = 'Saved'
}: { children?: ReactNode; saveStatus?: string } & HTMLAttributes<HTMLElement>) => {
  const isSaved = saveStatus === 'Saved'

  return (
    <header
      className={cn(
        'sticky top-0 z-50 h-16 border-b border-zinc-800 bg-zinc-950/80 backdrop-blur-md',
        className
      )}
    >
      <nav className={'flex h-full items-center justify-between px-6 lg:px-8'}>
        <div className={'flex items-center gap-3'}>
          <FileText className={'size-6 text-blue-500'} />
          <h1 className={'text-xl font-semibold text-zinc-100'}>{'AI Editor'}</h1>
        </div>

        <div className={'flex items-center gap-4'}>
          <div className={'flex items-center gap-2 text-sm text-zinc-400'}>
            <div className={cn('size-2 rounded-full', isSaved ? 'bg-green-500' : 'bg-amber-500')} />
            <span className={'hidden sm:inline'}>{saveStatus}</span>
          </div>
          {children}
        </div>
      </nav>
    </header>
  )
}
