'use client'

import { useState } from 'react'

import { SidebarInset, SidebarProvider } from '@/components/base/Sidebar'
import { TooltipProvider } from '@/components/base/Tooltip'
import Editor from '@/components/Editor'
import { EditorSidebar } from '@/components/EditorSidebar'
import { Header } from '@/components/Layout'

type SidebarProps = {
  isConnected: boolean
  isServerSynced: boolean
  saveStatus: string
  characterCount?: number
  isAutoModeEnabled: boolean
  isLinterEnabled: boolean
  isBackseaterEnabled: boolean
  isEmojiReplacerEnabled: boolean
  isPending: boolean
  remainingTime: number | null
  onAutoModeToggle: () => void
  onLinterToggle: () => void
  onBackseaterToggle: () => void
  onEmojiReplacerToggle: () => void
}

export default function Home() {
  const [sidebarProps, setSidebarProps] = useState<SidebarProps | null>(null)

  return (
    <TooltipProvider>
      <SidebarProvider>
        <div className={'flex h-screen flex-col bg-zinc-950'}>
          {sidebarProps && <EditorSidebar {...sidebarProps} />}
          <SidebarInset className={'flex flex-col overflow-hidden'}>
            <Header className={'shrink-0'} />
            <div className={'flex-1 overflow-hidden'}>
              <Editor onSidebarPropsChange={setSidebarProps} />
            </div>
          </SidebarInset>
        </div>
      </SidebarProvider>
    </TooltipProvider>
  )
}
