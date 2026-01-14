import { Extension } from '@tiptap/core'
import * as Y from 'yjs'

/**
 * Creates a Yjs extension that adds y-prosemirror plugins
 * This factory function preloads y-prosemirror to avoid async issues
 */
export async function createYjsExtension(yXmlFragment: Y.XmlFragment) {
  // Preload y-prosemirror
  const ypm = await import('y-prosemirror')
  const { ySyncPlugin, yUndoPlugin } = ypm

  return Extension.create({
    name: 'yjs',

    addProseMirrorPlugins() {
      return [
        ySyncPlugin(yXmlFragment),
        yUndoPlugin(),
      ]
    },
  })
}

