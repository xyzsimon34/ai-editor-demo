import React from 'react'
import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { createRoot } from 'react-dom/client'
import type { Mark } from '@tiptap/pm/model'

import Check from '@/components/icons/Check'
import Close from '@/components/icons/Close'

const pluginKey = new PluginKey('aiGhostExtension')

export interface AIGhostStorage {
  suggestion: string | null
  agentType: 'composer' | 'linter' | 'backseater' | null
  markedSuggestion: {
    from: number
    to: number
    text: string
    agentType: 'composer' | 'linter' | 'backseater'
  } | null
}

// Helper to map tool name to agentType
function mapToolToAgentType(tool: string | null | undefined): 'composer' | 'linter' | 'backseater' {
  if (tool === 'linter') return 'linter'
  if (tool === 'backseater') return 'backseater'
  return 'composer'
}

// Scan document for pending AI suggestion marks
function findPendingAISuggestion(doc: any): { from: number; to: number; text: string; agentType: 'composer' | 'linter' | 'backseater' } | null {
  let startPos: number | null = null
  let endPos: number | null = null
  let agentType: 'composer' | 'linter' | 'backseater' = 'composer'
  const textParts: string[] = []
  let inMarkedRegion = false

  doc.descendants((node: any, pos: number) => {
    if (node.isText) {
      const aiMark = node.marks?.find((mark: Mark) => mark.type.name === 'aisuggestion')
      
      if (aiMark) {
        const status = aiMark.attrs?.status
        if (status === 'pending') {
          const tool = aiMark.attrs?.tool
          const currentAgentType = mapToolToAgentType(tool)
          
          if (!inMarkedRegion) {
            // Start of marked region
            startPos = pos
            agentType = currentAgentType
            inMarkedRegion = true
          }
          
          // Accumulate text and track end position
          textParts.push(node.textContent)
          endPos = pos + node.textContent.length
        } else {
          // Status is not pending - finalize if we were building a region
          if (inMarkedRegion && startPos !== null && endPos !== null) {
            return false // Stop searching, we found our region
          }
        }
      } else {
        // No mark on this text node
        if (inMarkedRegion) {
          // We were in a marked region but this node doesn't have the mark
          // Finalize the region
          return false // Stop searching
        }
      }
    } else {
      // Non-text node - if we were building a region, finalize it
      if (inMarkedRegion && startPos !== null && endPos !== null) {
        return false // Stop searching
      }
    }
  })

  // Return the found region if we have one
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
    const extension = this // Capture extension reference for widget callbacks (this in Plugin refers to Plugin, not Extension)
    
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

            // Handle programmatic suggestion (set via command) - only if no marked suggestion
            if (meta?.action === 'set') {
              // Check if there's a marked suggestion first
              const markedSuggestion = findPendingAISuggestion(newState.doc)
              if (!markedSuggestion) {
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
                return DecorationSet.create(tr.doc, [widget])
              }
            }

            // Always check for marked AI suggestions FIRST (before mapping)
            // This ensures decorations persist even through Yjs sync transactions
            const markedSuggestion = findPendingAISuggestion(newState.doc)
            
            // Update storage
            if (extension.editor) {
              extension.editor.storage.aiGhost.markedSuggestion = markedSuggestion
            }

            // If marked suggestion exists, always recreate decorations (don't rely on mapping)
            if (markedSuggestion) {
              let colorClass = 'text-zinc-500'
              if (markedSuggestion.agentType === 'linter') colorClass = 'text-red-500'
              if (markedSuggestion.agentType === 'backseater') colorClass = 'text-yellow-500'

              // Create inline decoration to style the marked text as ghost text
              const decoration = Decoration.inline(
                markedSuggestion.from,
                markedSuggestion.to,
                {
                  class: `${colorClass} opacity-60`,
                  style: 'opacity: 0.6;',
                }
              )

              // Create widget decoration for the buttons after the marked text
              const widget = Decoration.widget(
                markedSuggestion.to,
                (_view) => {
                  const container = document.createElement('span')
                  container.className = 'inline-flex items-center ml-1'
                  container.style.pointerEvents = 'auto'

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

              return DecorationSet.create(newState.doc, [decoration, widget])
            }

            // Map decorations through document changes (for programmatic suggestions only)
            decorationSet = decorationSet.map(tr.mapping, tr.doc)
            
            return decorationSet
          }
        },
        props: {
          decorations(state) {
            const pluginState = this.getState(state)
            
            // If plugin state has decorations, return them
            // Check if decoration set is not empty by checking if find() returns something
            if (pluginState && pluginState.find().length > 0) {
              return pluginState
            }
            
            // Fallback: If decorations are missing but marked suggestion exists, recreate them
            // This handles cases where Yjs sync transactions clear decorations
            const markedSuggestion = findPendingAISuggestion(state.doc)
            if (markedSuggestion) {
              let colorClass = 'text-zinc-500'
              if (markedSuggestion.agentType === 'linter') colorClass = 'text-red-500'
              if (markedSuggestion.agentType === 'backseater') colorClass = 'text-yellow-500'

              const decoration = Decoration.inline(
                markedSuggestion.from,
                markedSuggestion.to,
                {
                  class: `${colorClass} opacity-60`,
                  style: 'opacity: 0.6;',
                }
              )

              const widget = Decoration.widget(
                markedSuggestion.to,
                (_view) => {
                  const container = document.createElement('span')
                  container.className = 'inline-flex items-center ml-1'
                  container.style.pointerEvents = 'auto'

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

              return DecorationSet.create(state.doc, [decoration, widget])
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
        (text: string, agentType: 'composer' | 'linter' | 'backseater' = 'composer') =>
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
          // First check for marked suggestion (from backend)
          const markedSuggestion = this.storage.markedSuggestion
          if (markedSuggestion) {
            // Remove the aisuggestion mark from the text
            const { from, to } = markedSuggestion
            const markType = state.schema.marks.aisuggestion
            
            if (markType && dispatch) {
              tr.removeMark(from, to, markType)
              this.storage.markedSuggestion = null
              tr.setMeta(pluginKey, { action: 'clear' })
              dispatch(tr)
              // Set selection after dispatching
              commands.setTextSelection(to)
              return true
            }
          }

          // Fall back to programmatic suggestion
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

            // Set selection to end, then insert (this appends to last paragraph)
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
          // First check for marked suggestion (from backend)
          const markedSuggestion = this.storage.markedSuggestion
          if (markedSuggestion) {
            // Delete the marked text
            const { from, to } = markedSuggestion
            if (dispatch) {
              tr.delete(from, to)
              this.storage.markedSuggestion = null
              tr.setMeta(pluginKey, { action: 'clear' })
              dispatch(tr)
              return true
            }
          }

          // Fall back to programmatic suggestion
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
      setAISuggestion: (text: string, agentType?: 'composer' | 'linter' | 'backseater') => ReturnType
      clearAISuggestion: () => ReturnType
      acceptAISuggestion: () => ReturnType
      rejectAISuggestion: () => ReturnType
    }
  }
}
