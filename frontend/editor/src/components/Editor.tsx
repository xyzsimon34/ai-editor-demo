'use client'

import * as React from 'react'
import { useCallback, useEffect, useState } from 'react'
import type { Extension } from '@tiptap/core'
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

import { AIGhostExtension } from '@/lib/aiGhostExtension'
import { getExtensions } from '@/lib/extensions'
import { uploadFn } from '@/lib/imageUpload'
import { createYjsExtension } from '@/lib/yjsExtension'
import { useAsyncGuard } from '@/hooks/useAsyncGuard'
import { useAutoAITrigger } from '@/hooks/useAutoAITrigger'
import { useCollaboration, type BackseaterComment } from '@/hooks/useCollaboration'
import { useYjsPersistence } from '@/hooks/useYjsPersistence'

import { AIStatusBubble } from './AIStatusBubble'
import { Separator } from './base/Separator'
import { TextButtons } from './base/TextButtons'
import { CommentToast } from './CommentToast'
import GenerativeMenuSwitch from './generative/GenerativeMenuSwitch'
import { slashCommand, suggestionItems } from './SlashCommand'

// Constants
const DOC_ID = 'ai-editor-doc'
const DEFAULT_EDITOR_CONTENT: JSONContent = { type: 'doc', content: [] }
const AI_HIGHLIGHT_DURATION_MS = 3000
const FOCUS_DELAY_MS = 100

// Types
interface EditorProps {
  onSaveStatusChange?: (status: string) => void
  onSidebarPropsChange?: (props: {
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
  }) => void
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

// Helpers
function requestPersistentStorage() {
  navigator.storage?.persist?.()
}

// Main Component
export default function Editor({ onSaveStatusChange, onSidebarPropsChange }: EditorProps) {
  const [ydoc] = useState(() => new Y.Doc({ gc: false }))
  const [yXmlFragment] = useState(() => ydoc.getXmlFragment('content'))
  const [editorInstance, setEditorInstance] = useState<EditorInstance | null>(null)

  const handleAiSuggestion = (text: string) => {
    editorInstance?.commands.setAISuggestion(text)
  }

  const [currentComment, setCurrentComment] = useState<BackseaterComment | null>(null)

  const handleComment = (comment: BackseaterComment) => {
    setCurrentComment(comment)
  }

  const { isLocalSynced } = useYjsPersistence({ docId: DOC_ID, ydoc })

  const handleToggleStateChange = useCallback(
    (toggleType: 'LINTER' | 'BACKSEATER' | 'EMOJI_REPLACER', enabled: boolean) => {
      if (toggleType === 'LINTER') {
        setIsLinterEnabled(enabled)
      } else if (toggleType === 'BACKSEATER') {
        setIsBackseaterEnabled(enabled)
      } else if (toggleType === 'EMOJI_REPLACER') {
        setIsEmojiReplacerEnabled(enabled)
      }
    },
    []
  )

  const {
    status: collaborationStatus,
    aiStatus,
    aiStatusMessage,
    isServerSynced,
    runAiCommand
  } = useCollaboration(ydoc, isLocalSynced, handleAiSuggestion, handleComment, handleToggleStateChange)

  const [initialContent, setInitialContent] = useState<JSONContent | null>(null)
  const [saveStatus, setSaveStatus] = useState('Saved')
  const [characterCount, setCharacterCount] = useState<number>()
  const [isGenerativeMenuOpen, setIsGenerativeMenuOpen] = useState(false)
  const [yjsExtension, setYjsExtension] = useState<Extension | null>(null)
  const [isAutoModeEnabled, setIsAutoModeEnabled] = useState(false)
  const [isLinterEnabled, setIsLinterEnabled] = useState(false)
  const [isBackseaterEnabled, setIsBackseaterEnabled] = useState(false)
  const [isEmojiReplacerEnabled, setIsEmojiReplacerEnabled] = useState(false)
  const [isAIGenerating, setIsAIGenerating] = useState(false)
  const asyncGuard = useAsyncGuard()

  const isConnected = collaborationStatus === 'connected'

  useEffect(() => {
    createYjsExtension(yXmlFragment).then(setYjsExtension)
  }, [yXmlFragment])

  useEffect(() => {
    requestPersistentStorage()
  }, [])

  const extensions = [...getExtensions(), ...(yjsExtension ? [yjsExtension] : []), AIGhostExtension, slashCommand]

  const handleAITrigger = useCallback(() => {
    if (runAiCommand && isConnected) {
      asyncGuard.nextId()
      setIsAIGenerating(true)
      runAiCommand('AGENT', { role: 'researcher', mode: 'preview' })
    }
  }, [runAiCommand, isConnected, asyncGuard])

  const handleLinterToggle = useCallback(() => {
    if (!runAiCommand || !isConnected) return
    runAiCommand('TOGGLE', 'LINTER')
  }, [runAiCommand, isConnected])

  const handleBackseaterToggle = useCallback(() => {
    if (!runAiCommand || !isConnected) return
    runAiCommand('TOGGLE', 'BACKSEATER')
  }, [runAiCommand, isConnected])

  const handleEmojiReplacerToggle = useCallback(() => {
    if (!runAiCommand || !isConnected) return
    runAiCommand('TOGGLE', 'EMOJI_REPLACER')
  }, [runAiCommand, isConnected])

  const { scheduleAITrigger, cancelScheduled, isPending, remainingTime } = useAutoAITrigger(editorInstance, {
    enabled: isAutoModeEnabled,
    debounceMs: 3000,
    minCharacters: 10,
    minChangeThreshold: 1,
    onTrigger: handleAITrigger
  })

  const handleAutoModeToggle = useCallback(() => {
    setIsAutoModeEnabled((prev) => {
      if (prev) cancelScheduled()
      return !prev
    })
  }, [cancelScheduled])

  const debouncedUpdates = useDebouncedCallback((editor: EditorInstance) => {
    const charCount = editor.storage.characterCount.characters()
    setCharacterCount(charCount > 0 ? charCount : undefined)

    setSaveStatus('Saved')
    onSaveStatusChange?.('Saved')

    if (isAutoModeEnabled) {
      if (isAIGenerating) {
        asyncGuard.cancel()
        setIsAIGenerating(false)
        editor.commands.clearAISuggestion()
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

      setTimeout(() => {
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
  }, [ydoc, editorInstance, yjsExtension, asyncGuard])

  useEffect(() => {
    if (editorInstance && yjsExtension) {
      setTimeout(() => editorInstance.commands.focus('end'), FOCUS_DELAY_MS)
    }
  }, [editorInstance, yjsExtension])

  // Notify parent component of sidebar props changes
  const onSidebarPropsChangeRef = React.useRef(onSidebarPropsChange)
  useEffect(() => {
    onSidebarPropsChangeRef.current = onSidebarPropsChange
  }, [onSidebarPropsChange])

  useEffect(() => {
    if (onSidebarPropsChangeRef.current) {
      onSidebarPropsChangeRef.current({
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
        onAutoModeToggle: handleAutoModeToggle,
        onLinterToggle: handleLinterToggle,
        onBackseaterToggle: handleBackseaterToggle,
        onEmojiReplacerToggle: handleEmojiReplacerToggle
      })
    }
  }, [
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
    handleAutoModeToggle,
    handleLinterToggle,
    handleBackseaterToggle,
    handleEmojiReplacerToggle
  ])

  if (!initialContent || !yjsExtension || !isLocalSynced) {
    return <LoadingState isLocalSynced={isLocalSynced} />
  }

  return (
    <div className={'relative min-h-screen w-full bg-zinc-900'}>
      <AIStatusBubble status={aiStatus} message={aiStatusMessage} />
      <CommentToast comment={currentComment} />

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
          onUpdate={({ editor, transaction }) => {
            setEditorInstance(editor)

            const aiMeta = transaction.getMeta('aiGhost')
            const isAIOperation = aiMeta?.action === 'set' || aiMeta?.action === 'clear'

            if (!isAIOperation) {
              debouncedUpdates(editor)
            }

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
