'use client'

import { useState } from 'react'

import { Header } from '@/components/Layout'
import Editor from '@/components/Editor'

export default function Home() {
  const [saveStatus, setSaveStatus] = useState('Saved')

  return (
    <div className={'flex min-h-screen flex-col bg-background'}>
      <Header saveStatus={saveStatus} />
      <main className={'flex-1'}>
        <div className={'mx-auto w-full max-w-4xl px-4 py-12 sm:px-6 lg:px-8'}>
          <Editor onSaveStatusChange={setSaveStatus} />
        </div>
      </main>
    </div>
  )
}
