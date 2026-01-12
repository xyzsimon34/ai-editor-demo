'use client'

import { useState } from 'react'

import Editor from '@/components/editor'
import MainLayout from '@/components/layout/main-layout'

export default function Home() {
  const [saveStatus, setSaveStatus] = useState('Saved')

  const handleSave = () => {
    setSaveStatus('Saved')
  }

  return (
    <MainLayout saveStatus={saveStatus} onSave={handleSave}>
      <div className={'flex flex-col'}>
        <Editor onSaveStatusChange={setSaveStatus} />
      </div>
    </MainLayout>
  )
}
