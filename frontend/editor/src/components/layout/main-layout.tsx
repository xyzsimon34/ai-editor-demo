'use client'

import { ReactNode } from 'react'

import Header from './header'

interface MainLayoutProps {
  children: ReactNode
  saveStatus?: string
  onSave?: () => void
}

export default function MainLayout({ children, saveStatus, onSave: _onSave }: MainLayoutProps) {
  return (
    <div className={'flex min-h-screen flex-col bg-background'}>
      <Header saveStatus={saveStatus} _onSave={_onSave} />
      <main className={'flex-1'}>
        <div className={'mx-auto w-full max-w-4xl px-4 py-12 sm:px-6 lg:px-8'}>{children}</div>
      </main>
    </div>
  )
}
