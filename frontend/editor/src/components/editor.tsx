'use client'

import { useEffect, useState } from 'react'
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

import { AIStatusBubble } from './ai-status-bubble'
import { Button } from './base/Button'
import { Separator } from './base/Separator'
import { TextButtons } from './base/TextButtons'
import GenerativeMenuSwitch from './generative/generative-menu-switch'
import { slashCommand, suggestionItems } from './slash-command'

const defaultEditorContent: JSONContent = {
  type: 'doc',
  content: []
}

interface EditorProps {
  onSaveStatusChange?: (status: string) => void
}

export default function Editor({ onSaveStatusChange }: EditorProps) {
  const [ydoc] = useState(() => new Y.Doc())
  const [yXmlFragment] = useState(() => ydoc.getXmlFragment('content'))

  const { status: collaborationStatus, aiStatus, runAiCommand } = useCollaboration(ydoc)

  const [initialContent, setInitialContent] = useState<null | JSONContent>(null)
  const [saveStatus, setSaveStatus] = useState('Saved')
  const [characterCount, setCharacterCount] = useState<number>()
  const [editorInstance, setEditorInstance] = useState<EditorInstance | null>(null)
  const [isGenerativeMenuOpen, setIsGenerativeMenuOpen] = useState(false)
  const [yjsExtension, setYjsExtension] = useState<Extension | null>(null)
  const [isAutoModeEnabled, setIsAutoModeEnabled] = useState(false)
  const [isLinterEnabled, setIsLinterEnabled] = useState(false)
  const [isAIGenerating, setIsAIGenerating] = useState(false)

  useEffect(() => {
    createYjsExtension(yXmlFragment).then(setYjsExtension)
  }, [yXmlFragment])

  const extensions = [
    ...getExtensions(),
    ...(yjsExtension ? [yjsExtension] : []),
    AIHighlightDecorationExtension,
    slashCommand
  ]

  const handleAITrigger = () => {
    if (runAiCommand && collaborationStatus === 'connected') {
      setIsAIGenerating(true)
      runAiCommand('AGENT', { role: 'researcher' })
    }
  }

  const handleLinterToggle = () => {
    if (!runAiCommand || collaborationStatus !== 'connected') {
      return
    }
    setIsLinterEnabled((prev) => !prev)
    runAiCommand('TOGGLE', 'LINTER')
  }

  const { scheduleAITrigger, cancelScheduled, isPending, remainingTime } = useAutoAITrigger(editorInstance, {
    enabled: isAutoModeEnabled,
    debounceMs: 3000,
    minCharacters: 10,
    minChangeThreshold: 10,
    onTrigger: handleAITrigger
  })

  const debouncedUpdates = useDebouncedCallback(async (editor: EditorInstance) => {
    const json = editor.getJSON()
    const charCount = editor.storage.characterCount.characters()
    setCharacterCount(charCount > 0 ? charCount : undefined)

    window.localStorage.setItem('novel-content', JSON.stringify(json))
    window.localStorage.setItem('markdown', editor.storage.markdown.getMarkdown())
    const newStatus = 'Saved'
    setSaveStatus(newStatus)
    onSaveStatusChange?.(newStatus)

    if (isAutoModeEnabled && !isAIGenerating) {
      scheduleAITrigger()
    }
  }, 500)

  useEffect(() => {
    if (yjsExtension) {
      setInitialContent(defaultEditorContent)
    } else {
      const content = window.localStorage.getItem('novel-content')
      if (content) setInitialContent(JSON.parse(content))
      else setInitialContent(defaultEditorContent)
    }
  }, [yjsExtension])

  useEffect(() => {
    if (!editorInstance || !yjsExtension) return

    const handleYjsUpdate = (update: Uint8Array, origin: unknown) => {
      if (origin !== 'websocket') return

      editorInstance.commands.highlightAIText('[AI was here]')

      setTimeout(() => {
        editorInstance.commands.clearAIHighlight()
        setIsAIGenerating(false)
      }, 3000)
    }

    ydoc.on('update', handleYjsUpdate)

    return () => {
      ydoc.off('update', handleYjsUpdate)
    }
  }, [ydoc, editorInstance, yjsExtension])

  useEffect(() => {
    if (editorInstance && yjsExtension) {
      setTimeout(() => {
        editorInstance.commands.focus('end')
      }, 100)
    }
  }, [editorInstance, yjsExtension])

  if (!initialContent || !yjsExtension) return null

  return (
    <div className={'relative min-h-screen w-full bg-zinc-900'}>
      <AIStatusBubble status={aiStatus} />

      <div
        className={
          'fixed right-4 top-4 z-50 flex items-center gap-3 rounded-lg bg-zinc-800/90 px-3 py-2 text-xs backdrop-blur-sm'
        }
      >
        <span className={collaborationStatus === 'connected' ? 'text-green-500' : 'text-amber-500'}>
          {collaborationStatus === 'connected' ? '●' : '◐'}
        </span>
        <span className={'text-zinc-400'}>{saveStatus}</span>
        {characterCount !== undefined && characterCount > 0 && (
          <span className={'text-zinc-500'}>{`${characterCount} characters`}</span>
        )}
      </div>

      <div className={'fixed bottom-6 left-6 z-50 flex items-center gap-3'}>
        <Button
          onClick={() => {
            setIsAutoModeEnabled(!isAutoModeEnabled)
            if (isAutoModeEnabled) {
              cancelScheduled()
            }
          }}
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
          disabled={collaborationStatus !== 'connected'}
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
            const newStatus = 'Unsaved'
            setSaveStatus(newStatus)
            onSaveStatusChange?.(newStatus)
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

      {/* <PulseSidebar editorText={editorText} isOpen={sidebarOpen} onToggle={() => setSidebarOpen(!sidebarOpen)} /> */}
    </div>
  )
}
