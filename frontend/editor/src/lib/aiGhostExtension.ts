import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { createRoot } from 'react-dom/client'
import React from 'react'
import Check from '@/components/icons/Check'
import Close from '@/components/icons/Close'

const pluginKey = new PluginKey('aiGhostExtension')

export interface AIGhostStorage {
  suggestion: string | null
  agentType: 'composer' | 'linter' | 'backseater' | null
}

export const AIGhostExtension = Extension.create<never, AIGhostStorage>({
  name: 'aiGhost',

  addStorage() {
    return {
      suggestion: null,
      agentType: null
    }
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: pluginKey,
        state: {
          init() {
            return DecorationSet.empty
          },
          apply: (tr, decorationSet) => {
            const meta = tr.getMeta(pluginKey)
            if (meta?.action === 'set') {
              const { text, pos, agentType } = meta

              let colorClass = 'text-zinc-500'
              if (agentType === 'linter') colorClass = 'text-red-500'
              if (agentType === 'backseater') colorClass = 'text-yellow-500'

              const widget = Decoration.widget(
                pos,
                (_view) => {
                  const container = document.createElement('span')
                  container.className = 'inline-flex items-center ml-1'
                  container.style.pointerEvents = 'auto'

                  const textSpan = document.createElement('span')
                  textSpan.textContent = text
                  textSpan.className = `${colorClass} opacity-60 mr-2`
                  container.appendChild(textSpan)

                  const btnGroup = document.createElement('span')
                  btnGroup.className = 'inline-flex gap-1 select-none items-center'

                  const acceptBtn = document.createElement('button')
                  acceptBtn.className =
                    'flex items-center justify-center w-4 h-4 rounded-full bg-green-500/20 text-green-500 hover:bg-green-500/30 transition-colors cursor-pointer border border-green-500/30'
                  acceptBtn.title = 'Accept (Tab)'
                  acceptBtn.onmousedown = (e) => {
                    e.preventDefault()
                    e.stopPropagation()

                    if (this.editor) {
                      this.editor.commands.acceptAISuggestion()
                      this.editor.commands.focus()
                    }
                  }
                  const acceptRoot = createRoot(acceptBtn)
                  acceptRoot.render(React.createElement(Check, { className: 'size-4' }))

                  const rejectBtn = document.createElement('button')
                  rejectBtn.className =
                    'flex items-center justify-center w-4 h-4 rounded-full bg-red-500/20 text-red-500 hover:bg-red-500/30 transition-colors cursor-pointer border border-red-500/30'
                  rejectBtn.title = 'Reject (Esc)'
                  rejectBtn.onmousedown = (e) => {
                    e.preventDefault()
                    e.stopPropagation()
                    if (this.editor) {
                      this.editor.commands.rejectAISuggestion()
                      this.editor.commands.focus()
                    }
                  }
                  const rejectRoot = createRoot(rejectBtn)
                  rejectRoot.render(React.createElement(Close, { className: 'size-4' }))

                  btnGroup.appendChild(acceptBtn)
                  btnGroup.appendChild(rejectBtn)
                  container.appendChild(btnGroup)

                  return container
                },
                { side: 1 }
              )
              return DecorationSet.create(tr.doc, [widget])
            }

            if (meta?.action === 'clear') {
              return DecorationSet.empty
            }

            decorationSet = decorationSet.map(tr.mapping, tr.doc)
            return decorationSet
          }
        },
        props: {
          decorations(state) {
            return this.getState(state)
          }
        }
      })
    ]
  },

  addCommands() {
    return {
      setAISuggestion:
        (text: string, agentType: 'composer' | 'linter' | 'backseater' = 'composer') =>
        ({ tr, dispatch, editor }) => {
          this.storage.suggestion = text
          this.storage.agentType = agentType

          if (dispatch) {
            const pos = editor.state.selection.to
            tr.setMeta(pluginKey, { action: 'set', text, pos, agentType })
            dispatch(tr)
          }
          return true
        },

      clearAISuggestion:
        () =>
        ({ tr, dispatch }) => {
          this.storage.suggestion = null
          this.storage.agentType = null
          if (dispatch) {
            tr.setMeta(pluginKey, { action: 'clear' })
            dispatch(tr)
          }
          return true
        },

      acceptAISuggestion:
        () =>
        ({ commands }) => {
          const suggestion = this.storage.suggestion
          if (suggestion) {
            commands.insertContent(suggestion)
            commands.clearAISuggestion()
            return true
          }
          return false
        },

      rejectAISuggestion:
        () =>
        ({ commands }) => {
          if (this.storage.suggestion) {
            commands.clearAISuggestion()
            return true
          }
          return false
        }
    }
  },

  addKeyboardShortcuts() {
    return {
      Tab: () => {
        if (this.storage.suggestion) {
          return this.editor.commands.acceptAISuggestion()
        }
        return false
      },
      Escape: () => {
        if (this.storage.suggestion) {
          return this.editor.commands.rejectAISuggestion()
        }
        return false
      }
    }
  }
})

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    aiGhost: {
      setAISuggestion: (text: string, agentType?: 'composer' | 'linter' | 'backseater') => ReturnType
      clearAISuggestion: () => ReturnType
      acceptAISuggestion: () => ReturnType
      rejectAISuggestion: () => ReturnType
    }
  }
}
