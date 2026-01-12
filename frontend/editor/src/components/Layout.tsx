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
        'sticky top-0 z-50 h-14 border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60',
        className
      )}
    >
      <nav className={'flex h-full items-center justify-between px-4 sm:px-6 lg:px-8'}>
        <div className={'flex items-center gap-2'}>
          <FileText className={'size-5 text-primary'} />
          <h1 className={'text-lg font-semibold'}>{'AI Editor'}</h1>
        </div>

        <div className={'flex items-center gap-4'}>
          <div className={'flex items-center gap-2 text-xs text-muted-foreground'}>
            <div className={cn('size-2 rounded-full', isSaved ? 'bg-green-500' : 'bg-yellow-500')} />
            <span className={'hidden sm:inline'}>{saveStatus}</span>
          </div>
          {children}
        </div>
      </nav>
    </header>
  )
}
