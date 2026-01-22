'use client'

import { MessageSquare, Smile, Sparkles, Zap } from 'lucide-react'

import { cn } from '@/lib/utils'

import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator
} from './base/Sidebar'

function ConnectionIndicator({ isConnected, isServerSynced }: { isConnected: boolean; isServerSynced: boolean }) {
  const getClassName = () => {
    if (isConnected && isServerSynced) return 'text-green-500'
    if (isConnected) return 'text-blue-500'
    return 'text-amber-500'
  }

  const getTitle = () => {
    if (isConnected && isServerSynced) return 'Synced with server'
    if (isConnected) return 'Connected, syncing...'
    return 'Disconnected'
  }

  const getSymbol = () => {
    if (isConnected && isServerSynced) return '●'
    if (isConnected) return '◐'
    return '○'
  }

  return (
    <span className={getClassName()} title={getTitle()}>
      {getSymbol()}
    </span>
  )
}

interface EditorSidebarProps {
  // Status bar props
  isConnected: boolean
  isServerSynced: boolean
  saveStatus: string
  characterCount?: number
  // Toggle buttons props
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

export function EditorSidebar({
  isConnected,
  isServerSynced,
  saveStatus,
  characterCount,
  isAutoModeEnabled,
  isLinterEnabled,
  isBackseaterEnabled,
  isEmojiReplacerEnabled,
  isPending,
  remainingTime,
  onAutoModeToggle,
  onLinterToggle,
  onBackseaterToggle,
  onEmojiReplacerToggle
}: EditorSidebarProps) {
  return (
    <Sidebar side={'left'} variant={'sidebar'} collapsible={'icon'}>
      <SidebarHeader>
        <div className={'flex items-center gap-2.5 px-2 py-3'}>
          <div className={'flex size-8 items-center justify-center rounded-md'}></div>
          <div className={'flex flex-col'}></div>
        </div>
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>{'AI Features'}</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton
                  onClick={onAutoModeToggle}
                  isActive={isAutoModeEnabled}
                  tooltip={isAutoModeEnabled ? 'Auto AI' : 'Manual'}
                  className={isAutoModeEnabled ? 'bg-blue-500/10 text-blue-400 hover:bg-blue-500/15' : ''}
                >
                  <Zap className={cn('size-4', isAutoModeEnabled && 'text-blue-400')} />
                  <span>{isAutoModeEnabled ? 'Auto AI' : 'Manual'}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton
                  onClick={onLinterToggle}
                  isActive={isLinterEnabled}
                  disabled={!isConnected}
                  tooltip={isLinterEnabled ? 'Linter On' : 'Linter Off'}
                  className={isLinterEnabled ? 'bg-purple-500/10 text-purple-400 hover:bg-purple-500/15' : ''}
                >
                  <Sparkles className={cn('size-4', isLinterEnabled && 'text-purple-400')} />
                  <span>{isLinterEnabled ? 'Linter On' : 'Linter Off'}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton
                  onClick={onBackseaterToggle}
                  isActive={isBackseaterEnabled}
                  disabled={!isConnected}
                  tooltip={isBackseaterEnabled ? 'Backseater On' : 'Backseater Off'}
                  className={isBackseaterEnabled ? 'bg-green-500/10 text-green-400 hover:bg-green-500/15' : ''}
                >
                  <MessageSquare className={cn('size-4', isBackseaterEnabled && 'text-green-400')} />
                  <span>{isBackseaterEnabled ? 'Backseater On' : 'Backseater Off'}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton
                  onClick={onEmojiReplacerToggle}
                  isActive={isEmojiReplacerEnabled}
                  disabled={!isConnected}
                  tooltip={isEmojiReplacerEnabled ? 'Emoji Replacer On' : 'Emoji Replacer Off'}
                  className={isEmojiReplacerEnabled ? 'bg-yellow-500/10 text-yellow-400 hover:bg-yellow-500/15' : ''}
                >
                  <Smile className={cn('size-4', isEmojiReplacerEnabled && 'text-yellow-400')} />
                  <span>{isEmojiReplacerEnabled ? 'Emoji Replacer On' : 'Emoji Replacer Off'}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        {isAutoModeEnabled && isPending && remainingTime !== null && (
          <SidebarGroup>
            <SidebarGroupContent>
              <div className={'px-2 py-1.5'}>
                <div
                  className={
                    'flex min-w-0 items-center gap-2 rounded-md border border-blue-500/20 bg-blue-500/10 px-3 py-2'
                  }
                >
                  <div className={'size-1.5 shrink-0 animate-pulse rounded-full bg-blue-400'} />
                  <span
                    className={'min-w-0 truncate text-xs font-medium text-blue-400'}
                  >{`AI processing in ${remainingTime}s...`}</span>
                </div>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>
        )}

        <SidebarSeparator />

        <SidebarGroup>
          <SidebarGroupLabel>{'Status'}</SidebarGroupLabel>
          <SidebarGroupContent>
            <div className={'space-y-2.5 px-2 py-1.5'}>
              <div className={'flex min-w-0 items-center gap-2.5 rounded-md bg-zinc-800/30 px-2.5 py-2'}>
                <ConnectionIndicator isConnected={isConnected} isServerSynced={isServerSynced} />
                <div className={'flex min-w-0 flex-1 flex-col'}>
                  <span className={'truncate text-xs font-medium text-zinc-300'}>{saveStatus}</span>
                  <span className={'truncate text-[10px] text-zinc-500'}>
                    {isConnected && isServerSynced ? 'All synced' : isConnected ? 'Syncing...' : 'Offline'}
                  </span>
                </div>
              </div>
              {characterCount !== undefined && characterCount > 0 && (
                <div className={'min-w-0 rounded-md bg-zinc-800/30 px-2.5 py-2'}>
                  <div className={'truncate text-xs font-medium text-zinc-300'}>{characterCount.toLocaleString()}</div>
                  <div className={'truncate text-[10px] text-zinc-500'}>{'characters'}</div>
                </div>
              )}
            </div>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </Sidebar>
  )
}
