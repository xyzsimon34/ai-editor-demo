import { Mark, mergeAttributes } from '@tiptap/core'

interface AttributeAttributes {
  status?: string
  aimodel?: string | null
  runid?: string | null
  tool?: string | null
}

function createDataAttribute(name: string, defaultValue: string | null = null) {
  return {
    default: defaultValue,
    parseHTML: (element: HTMLElement) => element.getAttribute(`data-${name}`),
    renderHTML: (attributes: AttributeAttributes) => ({ [`data-${name}`]: attributes[name as keyof AttributeAttributes] }),
  }
}

export const AISuggestion = Mark.create({
  name: 'aisuggestion',
  inclusive: false,

  addAttributes() {
    return {
      status: createDataAttribute('status', 'pending'),
      aimodel: createDataAttribute('aimodel'),
      runid: createDataAttribute('runid'),
      tool: createDataAttribute('tool'),
    }
  },

  parseHTML() {
    return [
      {
        tag: 'span[data-type="ai-suggestion"]',
      },
    ]
  },

  renderHTML({ HTMLAttributes }) {
    const status = HTMLAttributes.status || 'pending'
    const baseAttributes = {
      'data-type': 'ai-suggestion',
    }

    if (status === 'pending') {
      return [
        'span',
        mergeAttributes(HTMLAttributes, {
          ...baseAttributes,
          class: 'ai-suggestion-pending',
        }),
        0,
      ]
    }
    
    const classes = ['ai-suggestion', `ai-suggestion-${status}`].filter(Boolean).join(' ')
    
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        ...baseAttributes,
        class: classes,
      }),
      0,
    ]
  },
})