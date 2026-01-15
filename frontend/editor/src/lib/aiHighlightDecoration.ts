import { Extension } from '@tiptap/core'
import { Node as ProseMirrorNode } from '@tiptap/pm/model'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'

const pluginKey = new PluginKey('aiHighlightDecoration')

interface MatchPosition {
  start: number
  end: number
  text: string
}

function findDecorations(doc: ProseMirrorNode, searchText: string) {
  const allMatches: MatchPosition[] = []
  const searchRegex = new RegExp(searchText.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g')

  doc.descendants((node: ProseMirrorNode, pos: number) => {
    if (node.isText && node.text) {
      let match
      searchRegex.lastIndex = 0

      while ((match = searchRegex.exec(node.text)) !== null) {
        const start = pos + match.index
        const end = start + match[0].length
        allMatches.push({ start, end, text: match[0] })
      }
    }
  })

  const decorations: Decoration[] = []
  if (allMatches.length > 0) {
    const lastMatch = allMatches[allMatches.length - 1]

    decorations.push(
      Decoration.inline(lastMatch.start, lastMatch.end, {
        class: 'ai-highlight',
        style: 'background-color: #fef08a; border-radius: 2px; padding: 0 2px;'
      })
    )
  }

  return DecorationSet.create(doc, decorations)
}

export const AIHighlightDecorationExtension = Extension.create({
  name: 'aiHighlightDecoration',

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: pluginKey,

        state: {
          init() {
            return DecorationSet.empty
          },

          apply(tr, decorationSet) {
            decorationSet = decorationSet.map(tr.mapping, tr.doc)

            const meta = tr.getMeta(pluginKey)

            if (meta?.action === 'highlight') {
              const searchText = meta.searchText || '[AI was here]'
              return findDecorations(tr.doc, searchText)
            }

            if (meta?.action === 'clear') {
              return DecorationSet.empty
            }

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
      highlightAIText:
        (searchText = '[AI was here]') =>
        ({ tr, dispatch }) => {
          if (dispatch) {
            const transaction = tr.setMeta(pluginKey, {
              action: 'highlight',
              searchText
            })
            dispatch(transaction)
          }
          return true
        },

      clearAIHighlight:
        () =>
        ({ tr, dispatch }) => {
          if (dispatch) {
            const transaction = tr.setMeta(pluginKey, {
              action: 'clear'
            })
            dispatch(transaction)
          }
          return true
        }
    }
  }
})

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    aiHighlightDecoration: {
      highlightAIText: (searchText?: string) => ReturnType
      clearAIHighlight: () => ReturnType
    }
  }
}
