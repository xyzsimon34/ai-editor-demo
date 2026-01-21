import { Node, mergeAttributes } from '@tiptap/core'

export const AIGeneratedNode = Node.create({
  name: 'ai_generated',

  group: 'block',

  content: 'inline*',

  atom: false,

  addAttributes() {
    return {
      'data-ai-generated': {
        default: 'true',
        parseHTML: (element) => element.getAttribute('data-ai-generated'),
        renderHTML: (attributes) => {
          if (!attributes['data-ai-generated']) {
            return {}
          }
          return {
            'data-ai-generated': attributes['data-ai-generated']
          }
        }
      },
      'data-ai-id': {
        default: null,
        parseHTML: (element) => element.getAttribute('data-ai-id'),
        renderHTML: (attributes) => {
          if (!attributes['data-ai-id']) {
            return {}
          }
          return {
            'data-ai-id': attributes['data-ai-id']
          }
        }
      }
    }
  },

  parseHTML() {
    return [
      {
        tag: 'ai_generated[data-ai-generated="true"]',
        getAttrs: (node) => {
          if (typeof node === 'string') return false
          const element = node as HTMLElement
          return {
            'data-ai-generated': element.getAttribute('data-ai-generated'),
            'data-ai-id': element.getAttribute('data-ai-id')
          }
        }
      }
    ]
  },

  renderHTML({ HTMLAttributes }) {
    return ['ai_generated', mergeAttributes(HTMLAttributes, { 'data-ai-generated': 'true' }), 0]
  },

  addNodeView() {
    return ({ node }) => {
      const span = document.createElement('span')
      span.className = 'bg-blue-500/10 border-l-2 border-blue-500/30 pl-2 -ml-2'
      span.setAttribute('data-ai-generated', 'true')
      if (node.attrs['data-ai-id']) {
        span.setAttribute('data-ai-id', node.attrs['data-ai-id'])
      }
      return {
        dom: span,
        contentDOM: span
      }
    }
  }
})
