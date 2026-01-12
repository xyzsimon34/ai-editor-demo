import { BoldIcon, CodeIcon, ItalicIcon, StrikethroughIcon, UnderlineIcon } from 'lucide-react'
import { EditorBubbleItem, useEditor, type EditorInstance } from 'novel'

import { cn } from '@/lib/utils'
import { Button } from './Button'

type SelectorItem = {
  name: string
  isActive: (editor: EditorInstance) => boolean
  command: (editor: EditorInstance) => void
  icon: React.ComponentType<{ className?: string }>
}

export const TextButtons = () => {
  const { editor } = useEditor()
  if (!editor) return null
  const items: SelectorItem[] = [
    {
      name: 'bold',
      isActive: (editor) => editor.isActive('bold'),
      command: (editor) => editor.chain().focus().toggleBold().run(),
      icon: BoldIcon
    },
    {
      name: 'italic',
      isActive: (editor) => editor.isActive('italic'),
      command: (editor) => editor.chain().focus().toggleItalic().run(),
      icon: ItalicIcon
    },
    {
      name: 'underline',
      isActive: (editor) => editor.isActive('underline'),
      command: (editor) => editor.chain().focus().toggleUnderline().run(),
      icon: UnderlineIcon
    },
    {
      name: 'strike',
      isActive: (editor) => editor.isActive('strike'),
      command: (editor) => editor.chain().focus().toggleStrike().run(),
      icon: StrikethroughIcon
    },
    {
      name: 'code',
      isActive: (editor) => editor.isActive('code'),
      command: (editor) => editor.chain().focus().toggleCode().run(),
      icon: CodeIcon
    }
  ]
  return (
    <div className={'flex'}>
      {items.map((item) => (
        <EditorBubbleItem
          key={item.name}
          onSelect={(editor) => {
            item.command(editor)
          }}
        >
          <Button size={'sm'} className={'rounded-none'} variant={'ghost'} type={'button'}>
            <item.icon
              className={cn('size-4', {
                'text-blue-500': item.isActive(editor)
              })}
            />
          </Button>
        </EditorBubbleItem>
      ))}
    </div>
  )
}
