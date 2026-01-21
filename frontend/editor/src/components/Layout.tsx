'use client'

import { type HTMLAttributes, type ReactNode } from 'react'
import { FileText, Sparkles } from 'lucide-react'

import { cn } from '@/lib/utils'

export const Header = ({
  className,
  children
}: { children?: ReactNode } & HTMLAttributes<HTMLElement>) => {
  return (
    <header
      className={cn(
        'sticky top-0 z-10 h-16 shrink-0 border-b border-zinc-800/50 bg-zinc-950/95 backdrop-blur-sm supports-[backdrop-filter]:bg-zinc-950/80',
        className
      )}
    >
      <nav className={'flex h-full items-center justify-between px-4 lg:px-6'}>
        <div className={'flex items-center gap-2.5'}>
          <div className={'relative'}>
            <FileText className={'size-5 text-blue-500'} />
            <Sparkles className={'absolute -right-1 -top-1 size-2.5 text-blue-400'} />
          </div>
          <h1 className={'text-base font-semibold leading-tight text-zinc-100'}>{'AI Editor'}</h1>
        </div>

        {children && <div className={'flex items-center gap-3'}>{children}</div>}
      </nav>
    </header>
  )
}
