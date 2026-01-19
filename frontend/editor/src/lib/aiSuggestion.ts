import { Mark, mergeAttributes } from '@tiptap/core'

export const AISuggestion = Mark.create({
  // 1. Name must match the key sent from Rust
  name: 'aisuggestion',
  inclusive: false,

  // 2. Define the attributes we expect from the backend
  addAttributes() {
    return {
      status: {
        default: 'pending',
        parseHTML: element => element.getAttribute('data-status'),
        renderHTML: attributes => ({ 'data-status': attributes.status }),
      },
      aimodel: {
        default: null,
        parseHTML: element => element.getAttribute('data-aimodel'),
        renderHTML: attributes => ({ 'data-aimodel': attributes.aimodel }),
      },
      runid: {
        default: null,
        parseHTML: element => element.getAttribute('data-runid'),
        renderHTML: attributes => ({ 'data-runid': attributes.runid }),
      },
      tool: {
        default: null,
        parseHTML: element => element.getAttribute('data-tool'),
        renderHTML: attributes => ({ 'data-tool': attributes.tool }),
      },
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
    
    // Build class string based on status
    const classes = [
      'ai-suggestion',
      `ai-suggestion-${status}`
    ].filter(Boolean).join(' ')
    
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        'data-type': 'ai-suggestion',
        class: classes,
      }),
      0, // Render text content inside
    ]
  },
})