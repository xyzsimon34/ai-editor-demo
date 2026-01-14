'use client'

import { useEffect, useState } from 'react'
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

import { getExtensions } from '@/lib/extensions'
import { uploadFn } from '@/lib/image-upload'
import { useCollaboration } from '@/hooks/useCollaboration'
import { createYjsExtension } from '@/lib/yjsExtension'

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
  // 1. Create the Yjs Document for collaboration
  const [ydoc] = useState(() => new Y.Doc())
  // Create Yjs XML fragment for the editor content (must match backend field name)
  const [yXmlFragment] = useState(() => ydoc.getXmlFragment('content'))
  
  // 2. Setup WebSocket collaboration
  const { status: collaborationStatus } = useCollaboration(ydoc)

  const [initialContent, setInitialContent] = useState<null | JSONContent>(null)
  const [_saveStatus, setSaveStatus] = useState('Saved')
  const [charsCount, setCharsCount] = useState<number>()

  const [_openNode, _setOpenNode] = useState(false)
  const [_openColor, _setOpenColor] = useState(false)
  const [_openLink, _setOpenLink] = useState(false)
  const [openAI, setOpenAI] = useState(false)

  // Load Yjs extension asynchronously
  const [yjsExtension, setYjsExtension] = useState<any>(null)

  useEffect(() => {
    createYjsExtension(yXmlFragment).then(setYjsExtension)
  }, [yXmlFragment])

  // Get extensions (include Yjs extension once loaded)
  const extensions = [
    ...getExtensions(),
    ...(yjsExtension ? [yjsExtension] : []),
    slashCommand,
  ]

  const debouncedUpdates = useDebouncedCallback(async (editor: EditorInstance) => {
    const json = editor.getJSON()
    const charCount = editor.storage.characterCount.characters()
    setCharsCount(charCount > 0 ? charCount : undefined)
    window.localStorage.setItem('novel-content', JSON.stringify(json))
    window.localStorage.setItem('markdown', editor.storage.markdown.getMarkdown())
    const newStatus = 'Saved'
    setSaveStatus(newStatus)
    onSaveStatusChange?.(newStatus)
  }, 500)

  useEffect(() => {
    // When Yjs is active, use empty content - ySyncPlugin will populate from Yjs
    if (yjsExtension) {
      setInitialContent(defaultEditorContent)
    } else {
      const content = window.localStorage.getItem('novel-content')
      if (content) setInitialContent(JSON.parse(content))
      else setInitialContent(defaultEditorContent)
    }
  }, [yjsExtension])

  if (!initialContent || !yjsExtension) return null

  return (
    <div className={'relative w-full'}>
      <div className={'mb-4 flex items-center justify-between gap-4 text-sm text-muted-foreground'}>
        <div className={'flex items-center gap-2'}>
          <span className={collaborationStatus === 'connected' ? 'text-green-600' : 'text-red-600'}>
            {collaborationStatus === 'connected' ? '● Connected' : '○ Disconnected'}
          </span>
        </div>
        {charsCount !== undefined && charsCount > 0 && (
          <div className={'flex items-center gap-2'}>
            <span>
              {charsCount}
              {' characters'}
            </span>
          </div>
        )}
      </div>
      <EditorRoot>
        <EditorContent
          initialContent={initialContent}
          extensions={extensions}
          className={
            'relative min-h-[600px] w-full overflow-hidden rounded-lg border border-muted bg-background shadow-sm'
          }
          editorProps={{
            handleDOMEvents: {
              keydown: (_view, event) => handleCommandNavigation(event)
            },
            handlePaste: (view, event) => handleImagePaste(view, event, uploadFn),
            handleDrop: (view, event, _slice, moved) => handleImageDrop(view, event, moved, uploadFn),
            attributes: {
              class:
                'prose prose-lg dark:prose-invert prose-headings:font-title font-default focus:outline-none max-w-full px-4 sm:px-8 py-6'
            }
          }}
          onUpdate={({ editor }) => {
            debouncedUpdates(editor)
            const newStatus = 'Unsaved'
            setSaveStatus(newStatus)
            onSaveStatusChange?.(newStatus)
          }}
          slotAfter={<ImageResizer />}
        >
          <EditorCommand
            className={
              'z-50 h-auto max-h-[330px] overflow-y-auto rounded-md border border-muted bg-background px-1 py-2 shadow-md transition-all'
            }
          >
            <EditorCommandEmpty className={'px-2 text-muted-foreground'}>{'No results'}</EditorCommandEmpty>
            <EditorCommandList>
              {suggestionItems.map((item) => (
                <EditorCommandItem
                  value={item.title}
                  onCommand={(val) => item.command?.(val)}
                  className={
                    'flex w-full items-center space-x-2 rounded-md px-2 py-1 text-left text-sm hover:bg-accent aria-selected:bg-accent'
                  }
                  key={item.title}
                >
                  <div
                    className={'flex size-10 items-center justify-center rounded-md border border-muted bg-background'}
                  >
                    {item.icon}
                  </div>
                  <div>
                    <p className={'font-medium'}>{item.title}</p>
                    <p className={'text-xs text-muted-foreground'}>{item.description}</p>
                  </div>
                </EditorCommandItem>
              ))}
            </EditorCommandList>
          </EditorCommand>

          <GenerativeMenuSwitch open={openAI} onOpenChange={setOpenAI}>
            <Separator orientation={'vertical'} />
            <TextButtons />
            <Separator orientation={'vertical'} />
          </GenerativeMenuSwitch>
        </EditorContent>
      </EditorRoot>
    </div>
  )
}
