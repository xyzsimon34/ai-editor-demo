import React from 'react'
import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { createRoot } from 'react-dom/client'
import type { Mark, Node as ProseMirrorNode } from '@tiptap/pm/model'

import Check from '@/components/icons/Check'
import Close from '@/components/icons/Close'

const pluginKey = new PluginKey('aiGhostExtension')

type AgentType = 'composer' | 'linter' | 'backseater'

interface ExtensionWithEditor {
  editor?: {
    commands: {
      acceptAISuggestion: () => boolean
      rejectAISuggestion: () => boolean
      focus: (position: 'end') => boolean
    }
    storage: {
      aiGhost?: AIGhostStorage
    }
  }
}

export interface AIGhostStorage {
  suggestion: string | null
  agentType: AgentType | null
  markedSuggestion: {
    from: number
    to: number
    text: string
    agentType: AgentType
  } | null
}

function mapToolToAgentType(tool: string | null | undefined): AgentType {
  if (tool === 'linter') return 'linter'
  if (tool === 'backseater') return 'backseater'
  return 'composer'
}

function getColorClassForAgentType(agentType: AgentType): string {
  if (agentType === 'linter') return 'text-red-500'
  if (agentType === 'backseater') return 'text-yellow-500'
  return 'text-zinc-500'
}

function createActionButtonsWidget(
  position: number,
  extension: ExtensionWithEditor,
  showText?: string,
  textColorClass?: string
) {
  return Decoration.widget(
    position,
    (_view) => {
      const container = document.createElement('span')
      container.className = 'inline-flex items-center ml-1'
      container.style.pointerEvents = 'auto'

      if (showText && textColorClass) {
        const textSpan = document.createElement('span')
        textSpan.textContent = showText
        textSpan.className = `${textColorClass} opacity-60 mr-2`
        container.appendChild(textSpan)
      }

      const btnGroup = document.createElement('span')
      btnGroup.className = 'inline-flex gap-1 select-none items-center'

      const acceptBtn = document.createElement('button')
      acceptBtn.className =
        'flex items-center justify-center w-4 h-4 rounded-full bg-green-500/20 text-green-500 hover:bg-green-500/30 transition-colors cursor-pointer border border-green-500/30'
      acceptBtn.title = 'Accept (Tab)'
      acceptBtn.onmousedown = (e) => {
        e.preventDefault()
        e.stopPropagation()
        if (extension.editor) {
          extension.editor.commands.acceptAISuggestion()
          extension.editor.commands.focus('end')
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
        if (extension.editor) {
          extension.editor.commands.rejectAISuggestion()
          extension.editor.commands.focus('end')
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
}

function createMarkedSuggestionDecorations(
  markedSuggestion: { from: number; to: number; agentType: AgentType },
  extension: ExtensionWithEditor
) {
  const colorClass = getColorClassForAgentType(markedSuggestion.agentType)

  const inlineDecoration = Decoration.inline(markedSuggestion.from, markedSuggestion.to, {
    class: `${colorClass} opacity-60`,
    style: 'opacity: 0.6;',
  })

  const widgetDecoration = createActionButtonsWidget(markedSuggestion.to, extension)

  return [inlineDecoration, widgetDecoration]
}

function findPendingAISuggestion(doc: ProseMirrorNode): { from: number; to: number; text: string; agentType: AgentType } | null {
  let startPos: number | null = null
  let endPos: number | null = null
  let agentType: AgentType = 'composer'
  const textParts: string[] = []
  let inMarkedRegion = false

  doc.descendants((node: ProseMirrorNode, pos: number) => {
    if (node.isText) {
      const aiMark = node.marks?.find((mark: Mark) => mark.type.name === 'aisuggestion')

      if (aiMark) {
        const status = aiMark.attrs?.status
        if (status === 'pending') {
          const tool = aiMark.attrs?.tool
          const currentAgentType = mapToolToAgentType(tool)

          if (!inMarkedRegion) {
            startPos = pos
            agentType = currentAgentType
            inMarkedRegion = true
          }

          textParts.push(node.textContent)
          endPos = pos + node.textContent.length
        } else {
          if (inMarkedRegion && startPos !== null && endPos !== null) {
            return false
          }
        }
      } else {
        if (inMarkedRegion) {
          return false
        }
      }
    } else {
      if (inMarkedRegion && startPos !== null && endPos !== null) {
        return false
      }
    }
  })

  if (startPos !== null && endPos !== null && textParts.length > 0) {
    return {
      from: startPos,
      to: endPos,
      text: textParts.join(''),
      agentType
    }
  }

  return null
}

export const AIGhostExtension = Extension.create<never, AIGhostStorage>({
  name: 'aiGhost',

  addStorage() {
    return {
      suggestion: null,
      agentType: null,
      markedSuggestion: null
    }
  },

  addProseMirrorPlugins() {
    const extension = this

    return [
      new Plugin({
        key: pluginKey,
        state: {
          init() {
            return DecorationSet.empty
          },
          apply: (tr, decorationSet, oldState, newState) => {
            const meta = tr.getMeta(pluginKey)

            if (meta?.action === 'clear') {
              return DecorationSet.empty
            }

            if (meta?.action === 'set') {
              const markedSuggestion = findPendingAISuggestion(newState.doc)
              if (!markedSuggestion) {
                const { text, pos, agentType } = meta
                const colorClass = getColorClassForAgentType(agentType)
                const widget = createActionButtonsWidget(pos, extension, text, colorClass)
                return DecorationSet.create(tr.doc, [widget])
              }
            }

            const markedSuggestion = findPendingAISuggestion(newState.doc)

            if (extension.editor) {
              extension.editor.storage.aiGhost.markedSuggestion = markedSuggestion
            }

            if (markedSuggestion) {
              const decorations = createMarkedSuggestionDecorations(markedSuggestion, extension)
              return DecorationSet.create(newState.doc, decorations)
            }

            decorationSet = decorationSet.map(tr.mapping, tr.doc)

            return decorationSet
          }
        },
        props: {
          decorations(state) {
            const pluginState = this.getState(state)

            if (pluginState && pluginState.find().length > 0) {
              return pluginState
            }

            const markedSuggestion = findPendingAISuggestion(state.doc)
            if (markedSuggestion) {
              const decorations = createMarkedSuggestionDecorations(markedSuggestion, extension)
              return DecorationSet.create(state.doc, decorations)
            }

            return pluginState
          }
        }
      })
    ]
  },

  addCommands() {
    return {
      setAISuggestion:
        (text: string, agentType: AgentType = 'composer') =>
          ({ tr, dispatch }) => {
            this.storage.suggestion = text
            this.storage.agentType = agentType

            if (dispatch) {
              const pos = tr.doc.content.size
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
          ({ commands, tr, state, dispatch }) => {
            const markedSuggestion = this.storage.markedSuggestion
            if (markedSuggestion) {
              const { from, to } = markedSuggestion
              const markType = state.schema.marks.aisuggestion

              if (markType && dispatch) {
                tr.removeMark(from, to, markType)
                this.storage.markedSuggestion = null
                tr.setMeta(pluginKey, { action: 'clear' })
                dispatch(tr)
                commands.setTextSelection(to)
                return true
              }
            }

            const suggestion = this.storage.suggestion
            if (suggestion) {
              const doc = tr.doc
              const endPos = doc.content.size

              let textToInsert = suggestion
              if (doc.childCount > 0) {
                const lastChild = doc.lastChild
                if (lastChild && lastChild.type.name === 'paragraph' && lastChild.textContent.trim().length > 0) {
                  textToInsert = ` ${suggestion}`
                }
              }

              commands.setTextSelection(endPos)
              commands.insertContent(textToInsert)
              commands.clearAISuggestion()
              return true
            }
            return false
          },

      rejectAISuggestion:
        () =>
          ({ commands, tr, dispatch }) => {
            const markedSuggestion = this.storage.markedSuggestion
            if (markedSuggestion) {
              const { from, to } = markedSuggestion
              if (dispatch) {
                tr.delete(from, to)
                this.storage.markedSuggestion = null
                tr.setMeta(pluginKey, { action: 'clear' })
                dispatch(tr)
                return true
              }
            }

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
        if (this.storage.suggestion || this.storage.markedSuggestion) {
          return this.editor.commands.acceptAISuggestion()
        }
        return false
      },
      Escape: () => {
        if (this.storage.suggestion || this.storage.markedSuggestion) {
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
      setAISuggestion: (text: string, agentType?: AgentType) => ReturnType
      clearAISuggestion: () => ReturnType
      acceptAISuggestion: () => ReturnType
      rejectAISuggestion: () => ReturnType
    }
  }
}
