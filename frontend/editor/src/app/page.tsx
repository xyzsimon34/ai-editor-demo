'use client'

import { useState } from 'react'

import Editor from '@/components/editor'
import { Header } from '@/components/Layout'

export default function Home() {
  const [saveStatus, setSaveStatus] = useState('Saved')

  return (
    <div className={'flex min-h-screen flex-col bg-zinc-950'}>
      <Header saveStatus={saveStatus} />
      <main className={'flex-1'}>
        <Editor onSaveStatusChange={setSaveStatus} />
      </main>
    </div>
  )
}
