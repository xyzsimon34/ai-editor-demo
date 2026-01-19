'use client'

import { useEffect, useRef, useState } from 'react'
import type { Extension } from '@tiptap/core'
import { Sparkles, Zap } from 'lucide-react'
import {
  EditorCommand,
  EditorCommandEmpty,
  EditorCommandItem,
  EditorCommandList,
  EditorContent,
  EditorRoot,
  handleCommandNavigation,
  handleImageDrop,
  handleImagePaste,
  ImageResizer,
  type EditorInstance,
  type JSONContent
} from 'novel'
import { useDebouncedCallback } from 'use-debounce'
import * as Y from 'yjs'

import { AIHighlightDecorationExtension } from '@/lib/aiHighlightDecoration'
import { getExtensions } from '@/lib/extensions'
import { uploadFn } from '@/lib/image-upload'
import { createYjsExtension } from '@/lib/yjsExtension'
import { useAutoAITrigger } from '@/hooks/useAutoAITrigger'
import { useCollaboration } from '@/hooks/useCollaboration'
import { useYjsPersistence } from '@/hooks/useYjsPersistence'
import { useAsyncGuard } from '@/hooks/useAsyncGuard'

import { AIStatusBubble } from './ai-status-bubble'
import { Button } from './base/Button'
import { Separator } from './base/Separator'
import { TextButtons } from './base/TextButtons'
import GenerativeMenuSwitch from './generative/generative-menu-switch'
import { slashCommand, suggestionItems } from './slash-command'

// Constants
const DOC_ID = 'ai-editor-doc'
const DEFAULT_EDITOR_CONTENT: JSONContent = { type: 'doc', content: [] }
const AI_HIGHLIGHT_DURATION_MS = 3000
const FOCUS_DELAY_MS = 100

// Types
interface EditorProps {
  onSaveStatusChange?: (status: string) => void
}

// Subcomponents
function LoadingState({ isLocalSynced }: { isLocalSynced: boolean }) {
  return (
    <div className={'flex min-h-screen items-center justify-center bg-zinc-900'}>
      <div className={'flex flex-col items-center gap-3'}>
        <div className={'size-8 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-500'} />
        <span className={'text-sm text-zinc-400'}>
          {isLocalSynced ? 'Initializing editor...' : 'Loading local data...'}
        </span>
      </div>
    </div>
  )
}

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

function StatusBar({
  isConnected,
  isServerSynced,
  saveStatus,
  characterCount
}: {
  isConnected: boolean
  isServerSynced: boolean
  saveStatus: string
  characterCount?: number
}) {
  return (
    <div
      className={
        'fixed right-4 top-4 z-50 flex items-center gap-3 rounded-lg bg-zinc-800/90 px-3 py-2 text-xs backdrop-blur-sm'
      }
    >
      <ConnectionIndicator isConnected={isConnected} isServerSynced={isServerSynced} />
      <span className={'text-zinc-400'}>{saveStatus}</span>
      {characterCount !== undefined && characterCount > 0 && (
        <span className={'text-zinc-500'}>{`${characterCount} characters`}</span>
      )}
    </div>
  )
}

// Helpers
function requestPersistentStorage() {
  navigator.storage?.persist?.()
}

// Main Component
export default function Editor({ onSaveStatusChange }: EditorProps) {
  const [ydoc] = useState(() => new Y.Doc({ gc: false }))
  const [yXmlFragment] = useState(() => ydoc.getXmlFragment('content'))

  const { isLocalSynced } = useYjsPersistence({ docId: DOC_ID, ydoc })
  const { status: collaborationStatus, aiStatus, isServerSynced, runAiCommand } = useCollaboration(ydoc, isLocalSynced)

  const [initialContent, setInitialContent] = useState<JSONContent | null>(null)
  const [saveStatus, setSaveStatus] = useState('Saved')
  const [characterCount, setCharacterCount] = useState<number>()
  const [editorInstance, setEditorInstance] = useState<EditorInstance | null>(null)
  const [isGenerativeMenuOpen, setIsGenerativeMenuOpen] = useState(false)
  const [yjsExtension, setYjsExtension] = useState<Extension | null>(null)
  const [isAutoModeEnabled, setIsAutoModeEnabled] = useState(false)
  const [isLinterEnabled, setIsLinterEnabled] = useState(false)
  const [isAIGenerating, setIsAIGenerating] = useState(false)
  const asyncGuard = useAsyncGuard()

  const isConnected = collaborationStatus === 'connected'

  useEffect(() => {
    createYjsExtension(yXmlFragment).then(setYjsExtension)
  }, [yXmlFragment])

  useEffect(() => {
    requestPersistentStorage()
  }, [])

  const extensions = [
    ...getExtensions(),
    ...(yjsExtension ? [yjsExtension] : []),
    AIHighlightDecorationExtension,
    slashCommand
  ]

  const handleAITrigger = () => {
    if (runAiCommand && isConnected) {
      asyncGuard.nextId()
      setIsAIGenerating(true)
      runAiCommand('AGENT', { role: 'researcher' })
    }
  }

  const handleLinterToggle = () => {
    if (!runAiCommand || !isConnected) return
    setIsLinterEnabled((prev) => !prev)
    runAiCommand('TOGGLE', 'LINTER')
  }

  const handleAutoModeToggle = () => {
    setIsAutoModeEnabled((prev) => {
      if (prev) cancelScheduled()
      return !prev
    })
  }

  const { scheduleAITrigger, cancelScheduled, isPending, remainingTime } = useAutoAITrigger(editorInstance, {
    enabled: isAutoModeEnabled,
    debounceMs: 3000,
    minCharacters: 10,
    minChangeThreshold: 10,
    onTrigger: handleAITrigger
  })

  const debouncedUpdates = useDebouncedCallback((editor: EditorInstance) => {
    const charCount = editor.storage.characterCount.characters()
    setCharacterCount(charCount > 0 ? charCount : undefined)

    window.localStorage.setItem('novel-content', JSON.stringify(editor.getJSON()))
    window.localStorage.setItem('markdown', editor.storage.markdown.getMarkdown())

    setSaveStatus('Saved')
    onSaveStatusChange?.('Saved')

    if (isAutoModeEnabled) {
      if (isAIGenerating) {
        asyncGuard.cancel()
        setIsAIGenerating(false)
      }
      scheduleAITrigger()
    }
  }, 500)

  useEffect(() => {
    if (yjsExtension && isLocalSynced) setInitialContent(DEFAULT_EDITOR_CONTENT)
  }, [yjsExtension, isLocalSynced])

  useEffect(() => {
    if (!editorInstance || !yjsExtension) return

    const handleYjsUpdate = (_update: Uint8Array, origin: unknown) => {
      if (origin !== 'websocket') return

      const requestIdAtResponse = asyncGuard.currentId()

      if (requestIdAtResponse === 0) return

      editorInstance.commands.highlightAIText('[AI was here]')
      setTimeout(() => {
        editorInstance.commands.clearAIHighlight()
        if (asyncGuard.isLatest(requestIdAtResponse)) {
          setIsAIGenerating(false)
          asyncGuard.cancel()
        }
      }, AI_HIGHLIGHT_DURATION_MS)
    }

    ydoc.on('update', handleYjsUpdate)
    return () => {
      ydoc.off('update', handleYjsUpdate)
    }
  }, [ydoc, editorInstance, yjsExtension])

  useEffect(() => {
    if (editorInstance && yjsExtension) {
      setTimeout(() => editorInstance.commands.focus('end'), FOCUS_DELAY_MS)
    }
  }, [editorInstance, yjsExtension])

  if (!initialContent || !yjsExtension || !isLocalSynced) {
    return <LoadingState isLocalSynced={isLocalSynced} />
  }

  return (
    <div className={'relative min-h-screen w-full bg-zinc-900'}>
      <AIStatusBubble status={aiStatus} />

      <StatusBar
        isConnected={isConnected}
        isServerSynced={isServerSynced}
        saveStatus={saveStatus}
        characterCount={characterCount}
      />

      <div className={'fixed bottom-6 left-6 z-50 flex items-center gap-3'}>
        <Button
          onClick={handleAutoModeToggle}
          size={'sm'}
          variant={isAutoModeEnabled ? 'default' : 'outline'}
          className={
            isAutoModeEnabled
              ? 'gap-2 bg-blue-600 text-white hover:bg-blue-700'
              : 'gap-2 border-zinc-700 bg-zinc-800 hover:bg-zinc-700'
          }
        >
          <Zap className={'size-4'} />
          {isAutoModeEnabled ? 'Auto AI' : 'Manual'}
        </Button>

        <Button
          onClick={handleLinterToggle}
          size={'sm'}
          variant={isLinterEnabled ? 'default' : 'outline'}
          className={
            isLinterEnabled
              ? 'gap-2 bg-emerald-600 text-white hover:bg-emerald-700'
              : 'gap-2 border-zinc-700 bg-zinc-800 hover:bg-zinc-700'
          }
          disabled={!isConnected}
        >
          <Sparkles className={'size-4'} />
          {isLinterEnabled ? 'Linter On' : 'Linter Off'}
        </Button>

        {isAutoModeEnabled && isPending && remainingTime !== null && (
          <span className={'animate-pulse rounded-md bg-blue-600/20 px-3 py-1.5 text-xs text-blue-400'}>
            {`AI in ${remainingTime}s...`}
          </span>
        )}
      </div>

      <EditorRoot>
        <EditorContent
          initialContent={initialContent}
          extensions={extensions}
          className={'relative min-h-screen w-full border border-zinc-800 bg-zinc-900 shadow-xl'}
          editorProps={{
            handleDOMEvents: {
              keydown: (_view, event) => handleCommandNavigation(event)
            },
            handlePaste: (view, event) => handleImagePaste(view, event, uploadFn),
            handleDrop: (view, event, _slice, moved) => handleImageDrop(view, event, moved, uploadFn),
            attributes: {
              class:
                'prose prose-lg prose-invert prose-headings:font-title font-default focus:outline-none max-w-3xl mx-auto px-8 py-16 text-zinc-200'
            }
          }}
          onUpdate={({ editor }) => {
            setEditorInstance(editor)
            debouncedUpdates(editor)
            setSaveStatus('Unsaved')
            onSaveStatusChange?.('Unsaved')
          }}
          slotAfter={<ImageResizer />}
        >
          <EditorCommand
            className={
              'z-50 h-auto max-h-[330px] overflow-y-auto rounded-lg border border-zinc-700 bg-zinc-800 px-1 py-2 shadow-xl backdrop-blur-sm transition-all'
            }
          >
            <EditorCommandEmpty className={'px-2 text-zinc-500'}>{'No results'}</EditorCommandEmpty>
            <EditorCommandList>
              {suggestionItems.map((item) => (
                <EditorCommandItem
                  value={item.title}
                  onCommand={(val) => item.command?.(val)}
                  className={
                    'flex w-full items-center space-x-2 rounded-md px-2 py-1 text-left text-sm text-zinc-300 hover:bg-zinc-700 aria-selected:bg-zinc-700'
                  }
                  key={item.title}
                >
                  <div
                    className={
                      'flex size-10 items-center justify-center rounded-md border border-zinc-700 bg-zinc-900 text-zinc-400'
                    }
                  >
                    {item.icon}
                  </div>
                  <div>
                    <p className={'font-medium text-zinc-200'}>{item.title}</p>
                    <p className={'text-xs text-zinc-500'}>{item.description}</p>
                  </div>
                </EditorCommandItem>
              ))}
            </EditorCommandList>
          </EditorCommand>

          <GenerativeMenuSwitch open={isGenerativeMenuOpen} onOpenChange={setIsGenerativeMenuOpen}>
            <Separator orientation={'vertical'} />
            <TextButtons />
            <Separator orientation={'vertical'} />
          </GenerativeMenuSwitch>
        </EditorContent>
      </EditorRoot>
    </div>
  )
}
