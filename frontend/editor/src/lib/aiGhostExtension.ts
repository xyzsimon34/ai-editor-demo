import React from 'react'
import { Extension } from '@tiptap/core'
import type { Mark, Node as ProseMirrorNode } from '@tiptap/pm/model'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { createRoot } from 'react-dom/client'

import Check from '@/components/icons/Check'
import Close from '@/components/icons/Close'

const pluginKey = new PluginKey('aiGhostExtension')

type AgentType = 'composer' | 'linter' | 'backseater'

interface ExtensionWithEditor {
  editor?: {
    commands: {
      acceptAISuggestion: () => boolean
      rejectAISuggestion: () => boolean
      acceptSingleSuggestion: (changeIndex: number) => boolean
      rejectSingleSuggestion: (changeIndex: number) => boolean
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
    operation?: string | null
  } | null
}

function mapToolToAgentType(tool: string | null | undefined): AgentType {
  if (tool === 'linter') return 'linter'
  if (tool === 'backseater') return 'backseater'
  return 'composer'
}

function getColorForAgentType(agentType: AgentType): string {
  if (agentType === 'linter') return '#ef4444'
  if (agentType === 'backseater') return '#eab308'
  return '#71717a'
}

function createActionButtonsWidget(
  position: number,
  extension: ExtensionWithEditor,
  showText?: string,
  textColor?: string
) {
  return Decoration.widget(
    position,
    (_view) => {
      const container = document.createElement('span')
      container.style.cssText = 'display: inline-flex; align-items: center; margin-left: 4px; pointer-events: auto;'

      if (showText && textColor) {
        const textSpan = document.createElement('span')
        textSpan.textContent = showText
        textSpan.style.cssText = `color: ${textColor}; opacity: 0.6; margin-right: 8px;`
        container.appendChild(textSpan)
      }

      const btnGroup = document.createElement('span')
      btnGroup.style.cssText = 'display: inline-flex; gap: 4px; user-select: none; align-items: center;'

      const acceptBtn = document.createElement('button')
      acceptBtn.style.cssText =
        'display: flex; align-items: center; justify-content: center; width: 16px; height: 16px; border-radius: 9999px; background-color: rgba(34, 197, 94, 0.2); color: #22c55e; border: 1px solid rgba(34, 197, 94, 0.3); cursor: pointer; transition: background-color 0.2s;'
      acceptBtn.title = 'Accept (Tab)'
      acceptBtn.onmouseenter = () => {
        acceptBtn.style.backgroundColor = 'rgba(34, 197, 94, 0.3)'
      }
      acceptBtn.onmouseleave = () => {
        acceptBtn.style.backgroundColor = 'rgba(34, 197, 94, 0.2)'
      }
      acceptBtn.onmousedown = (e) => {
        e.preventDefault()
        e.stopPropagation()
        if (extension.editor) {
          extension.editor.commands.acceptAISuggestion()
          extension.editor.commands.focus('end')
        }
      }
      const acceptRoot = createRoot(acceptBtn)
      acceptRoot.render(React.createElement(Check, { style: { width: 16, height: 16 } }))

      const rejectBtn = document.createElement('button')
      rejectBtn.style.cssText =
        'display: flex; align-items: center; justify-content: center; width: 16px; height: 16px; border-radius: 9999px; background-color: rgba(239, 68, 68, 0.2); color: #ef4444; border: 1px solid rgba(239, 68, 68, 0.3); cursor: pointer; transition: background-color 0.2s;'
      rejectBtn.title = 'Reject (Esc)'
      rejectBtn.onmouseenter = () => {
        rejectBtn.style.backgroundColor = 'rgba(239, 68, 68, 0.3)'
      }
      rejectBtn.onmouseleave = () => {
        rejectBtn.style.backgroundColor = 'rgba(239, 68, 68, 0.2)'
      }
      rejectBtn.onmousedown = (e) => {
        e.preventDefault()
        e.stopPropagation()
        if (extension.editor) {
          extension.editor.commands.rejectAISuggestion()
          extension.editor.commands.focus('end')
        }
      }
      const rejectRoot = createRoot(rejectBtn)
      rejectRoot.render(React.createElement(Close, { style: { width: 16, height: 16 } }))

      btnGroup.appendChild(acceptBtn)
      btnGroup.appendChild(rejectBtn)
      container.appendChild(btnGroup)

      return container
    },
    { side: 1 }
  )
}

function getDecorationStyleForSuggestion(suggestion: PendingAISuggestion): string {
  const { operation, agentType } = suggestion

  if (operation === 'delete') {
    // Red strikethrough for deletions
    return 'color: #ef4444; text-decoration: line-through; opacity: 0.8; background-color: rgba(239, 68, 68, 0.1);'
  } else if (operation === 'insert') {
    // Green underline for insertions
    return 'color: #22c55e; border-bottom: 2px solid #22c55e; opacity: 0.8; background-color: rgba(34, 197, 94, 0.1);'
  } else {
    // Default style based on agent type
    if (agentType === 'linter') {
      return 'color: #ef4444; opacity: 0.6;'
    } else if (agentType === 'backseater') {
      return 'color: #eab308; opacity: 0.6;'
    } else {
      return 'color: #71717a; opacity: 0.6;'
    }
  }
}

function createSingleSuggestionWidget(position: number, suggestionIndex: number, extension: ExtensionWithEditor) {
  return Decoration.widget(
    position,
    (_view) => {
      const container = document.createElement('span')
      container.style.cssText =
        'display: inline-flex; align-items: center; margin-left: 2px; margin-right: 2px; pointer-events: auto;'

      const btnGroup = document.createElement('span')
      btnGroup.style.cssText = 'display: inline-flex; gap: 2px; user-select: none; align-items: center;'

      const acceptBtn = document.createElement('button')
      acceptBtn.style.cssText =
        'display: flex; align-items: center; justify-content: center; width: 14px; height: 14px; border-radius: 9999px; background-color: rgba(34, 197, 94, 0.2); color: #22c55e; border: 1px solid rgba(34, 197, 94, 0.3); cursor: pointer; transition: background-color 0.2s;'
      acceptBtn.title = 'Accept this change'
      acceptBtn.onmouseenter = () => {
        acceptBtn.style.backgroundColor = 'rgba(34, 197, 94, 0.3)'
      }
      acceptBtn.onmouseleave = () => {
        acceptBtn.style.backgroundColor = 'rgba(34, 197, 94, 0.2)'
      }
      acceptBtn.onmousedown = (e) => {
        e.preventDefault()
        e.stopPropagation()
        if (extension.editor) {
          extension.editor.commands.acceptSingleSuggestion(suggestionIndex)
        }
      }
      const acceptRoot = createRoot(acceptBtn)
      acceptRoot.render(React.createElement(Check, { style: { width: 12, height: 12 } }))

      const rejectBtn = document.createElement('button')
      rejectBtn.style.cssText =
        'display: flex; align-items: center; justify-content: center; width: 14px; height: 14px; border-radius: 9999px; background-color: rgba(239, 68, 68, 0.2); color: #ef4444; border: 1px solid rgba(239, 68, 68, 0.3); cursor: pointer; transition: background-color 0.2s;'
      rejectBtn.title = 'Reject this change'
      rejectBtn.onmouseenter = () => {
        rejectBtn.style.backgroundColor = 'rgba(239, 68, 68, 0.3)'
      }
      rejectBtn.onmouseleave = () => {
        rejectBtn.style.backgroundColor = 'rgba(239, 68, 68, 0.2)'
      }
      rejectBtn.onmousedown = (e) => {
        e.preventDefault()
        e.stopPropagation()
        if (extension.editor) {
          extension.editor.commands.rejectSingleSuggestion(suggestionIndex)
        }
      }
      const rejectRoot = createRoot(rejectBtn)
      rejectRoot.render(React.createElement(Close, { style: { width: 12, height: 12 } }))

      btnGroup.appendChild(acceptBtn)
      btnGroup.appendChild(rejectBtn)
      container.appendChild(btnGroup)

      return container
    },
    { side: 1 }
  )
}

function createAllSuggestionDecorations(
  suggestions: PendingAISuggestion[],
  extension: ExtensionWithEditor
): Decoration[] {
  const decorations: Decoration[] = []

  const linterSuggestions = suggestions.filter((s) => s.operation === 'delete' || s.operation === 'insert')
  const composerSuggestions = suggestions.filter((s) => !s.operation && s.agentType !== 'linter')

  if (linterSuggestions.length > 0) {
    const changes: {
      deleteSuggestion?: PendingAISuggestion
      insertSuggestion?: PendingAISuggestion
      endPos: number
    }[] = []

    let i = 0
    while (i < linterSuggestions.length) {
      const current = linterSuggestions[i]
      const next = linterSuggestions[i + 1]

      if (current.operation === 'delete' && next?.operation === 'insert' && current.to === next.from) {
        changes.push({
          deleteSuggestion: current,
          insertSuggestion: next,
          endPos: next.to
        })
        i += 2
      } else {
        changes.push({
          deleteSuggestion: current.operation === 'delete' ? current : undefined,
          insertSuggestion: current.operation === 'insert' ? current : undefined,
          endPos: current.to
        })
        i += 1
      }
    }

    for (const suggestion of linterSuggestions) {
      const decorationStyle = getDecorationStyleForSuggestion(suggestion)
      const inlineDecoration = Decoration.inline(suggestion.from, suggestion.to, {
        style: decorationStyle
      })
      decorations.push(inlineDecoration)
    }

    changes.forEach((change, index) => {
      const widgetDecoration = createSingleSuggestionWidget(change.endPos, index, extension)
      decorations.push(widgetDecoration)
    })
  }

  if (composerSuggestions.length > 0) {
    const lastSuggestion = composerSuggestions[composerSuggestions.length - 1]

    for (const suggestion of composerSuggestions) {
      const decorationStyle = getDecorationStyleForSuggestion(suggestion)
      const inlineDecoration = Decoration.inline(suggestion.from, suggestion.to, {
        style: decorationStyle
      })
      decorations.push(inlineDecoration)
    }

    const widgetDecoration = createActionButtonsWidget(lastSuggestion.to, extension)
    decorations.push(widgetDecoration)
  }

  return decorations
}

interface PendingAISuggestion {
  from: number
  to: number
  text: string
  agentType: AgentType
  operation?: string | null
}

function _findPendingAISuggestion(doc: ProseMirrorNode): PendingAISuggestion | null {
  let startPos: number | null = null
  let endPos: number | null = null
  let agentType: AgentType = 'composer'
  let operation: string | null = null
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
            operation = aiMark.attrs?.operation || null
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
      agentType,
      operation
    }
  }

  return null
}

interface CurrentSuggestionState {
  startPos: number
  endPos: number
  textParts: string[]
  agentType: AgentType
  operation: string | null
}

function findAllPendingAISuggestions(doc: ProseMirrorNode): PendingAISuggestion[] {
  const suggestions: PendingAISuggestion[] = []
  const state: { current: CurrentSuggestionState | null } = { current: null }

  const saveCurrent = () => {
    if (state.current) {
      suggestions.push({
        from: state.current.startPos,
        to: state.current.endPos,
        text: state.current.textParts.join(''),
        agentType: state.current.agentType,
        operation: state.current.operation
      })
      state.current = null
    }
  }

  doc.descendants((node: ProseMirrorNode, pos: number) => {
    if (node.isText) {
      const aiMark = node.marks?.find((mark: Mark) => mark.type.name === 'aisuggestion')

      if (aiMark && aiMark.attrs?.status === 'pending') {
        const tool = aiMark.attrs?.tool
        const operation = aiMark.attrs?.operation || null
        const agentType = mapToolToAgentType(tool)

        if (!state.current) {
          state.current = {
            startPos: pos,
            endPos: pos + node.textContent.length,
            textParts: [node.textContent],
            agentType,
            operation
          }
        } else if (state.current.operation === operation) {
          state.current.textParts.push(node.textContent)
          state.current.endPos = pos + node.textContent.length
        } else {
          saveCurrent()
          state.current = {
            startPos: pos,
            endPos: pos + node.textContent.length,
            textParts: [node.textContent],
            agentType,
            operation
          }
        }
      } else {
        saveCurrent()
      }
    } else {
      saveCurrent()
    }
  })

  saveCurrent()

  return suggestions
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
              const allSuggestions = findAllPendingAISuggestions(newState.doc)
              if (allSuggestions.length === 0) {
                const { text, pos, agentType } = meta
                const color = getColorForAgentType(agentType)
                const widget = createActionButtonsWidget(pos, extension, text, color)
                return DecorationSet.create(tr.doc, [widget])
              }
            }

            const allSuggestions = findAllPendingAISuggestions(newState.doc)

            if (extension.editor) {
              extension.editor.storage.aiGhost.markedSuggestion = allSuggestions[0] || null
            }

            if (allSuggestions.length > 0) {
              const decorations = createAllSuggestionDecorations(allSuggestions, extension)
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

            const allSuggestions = findAllPendingAISuggestions(state.doc)
            if (allSuggestions.length > 0) {
              const decorations = createAllSuggestionDecorations(allSuggestions, extension)
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
          const allSuggestions = findAllPendingAISuggestions(state.doc)

          if (allSuggestions.length > 0) {
            const markType = state.schema.marks.aisuggestion

            if (markType && dispatch) {
              const sortedSuggestions = [...allSuggestions].sort((a, b) => b.from - a.from)

              for (const suggestion of sortedSuggestions) {
                const { from, to, operation } = suggestion

                if (operation === 'delete') {
                  tr.delete(from, to)
                } else if (operation === 'insert') {
                  tr.removeMark(from, to, markType)
                } else {
                  tr.removeMark(from, to, markType)
                }
              }

              this.storage.markedSuggestion = null
              tr.setMeta(pluginKey, { action: 'clear' })
              dispatch(tr)
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
        ({ commands, tr, state, dispatch }) => {
          const allSuggestions = findAllPendingAISuggestions(state.doc)

          if (allSuggestions.length > 0) {
            const markType = state.schema.marks.aisuggestion

            if (markType && dispatch) {
              const sortedSuggestions = [...allSuggestions].sort((a, b) => b.from - a.from)

              for (const suggestion of sortedSuggestions) {
                const { from, to, operation } = suggestion

                if (operation === 'delete') {
                  tr.removeMark(from, to, markType)
                } else if (operation === 'insert') {
                  tr.delete(from, to)
                } else {
                  tr.delete(from, to)
                }
              }

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
        },

      acceptSingleSuggestion:
        (changeIndex: number) =>
        ({ tr, state, dispatch }) => {
          const allSuggestions = findAllPendingAISuggestions(state.doc)
          const markType = state.schema.marks.aisuggestion

          if (!markType || !dispatch) return false

          const actionableSuggestions = allSuggestions.filter(
            (s) => s.operation === 'delete' || s.operation === 'insert'
          )

          const changes: PendingAISuggestion[][] = []
          let i = 0
          while (i < actionableSuggestions.length) {
            const current = actionableSuggestions[i]
            const next = actionableSuggestions[i + 1]

            if (current.operation === 'delete' && next?.operation === 'insert' && current.to === next.from) {
              changes.push([current, next])
              i += 2
            } else {
              changes.push([current])
              i += 1
            }
          }

          if (changeIndex >= changes.length) return false

          const changeToAccept = changes[changeIndex]
          const sorted = [...changeToAccept].sort((a, b) => b.from - a.from)

          for (const suggestion of sorted) {
            const { from, to, operation } = suggestion
            if (operation === 'delete') {
              tr.delete(from, to)
            } else if (operation === 'insert') {
              tr.removeMark(from, to, markType)
            }
          }

          dispatch(tr)
          return true
        },

      rejectSingleSuggestion:
        (changeIndex: number) =>
        ({ tr, state, dispatch }) => {
          const allSuggestions = findAllPendingAISuggestions(state.doc)
          const markType = state.schema.marks.aisuggestion

          if (!markType || !dispatch) return false

          const actionableSuggestions = allSuggestions.filter(
            (s) => s.operation === 'delete' || s.operation === 'insert'
          )

          const changes: PendingAISuggestion[][] = []
          let i = 0
          while (i < actionableSuggestions.length) {
            const current = actionableSuggestions[i]
            const next = actionableSuggestions[i + 1]

            if (current.operation === 'delete' && next?.operation === 'insert' && current.to === next.from) {
              changes.push([current, next])
              i += 2
            } else {
              changes.push([current])
              i += 1
            }
          }

          if (changeIndex >= changes.length) return false

          const changeToReject = changes[changeIndex]
          const sorted = [...changeToReject].sort((a, b) => b.from - a.from)

          for (const suggestion of sorted) {
            const { from, to, operation } = suggestion
            if (operation === 'delete') {
              tr.removeMark(from, to, markType)
            } else if (operation === 'insert') {
              tr.delete(from, to)
            }
          }

          dispatch(tr)
          return true
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
      acceptSingleSuggestion: (changeIndex: number) => ReturnType
      rejectSingleSuggestion: (changeIndex: number) => ReturnType
    }
  }
}
