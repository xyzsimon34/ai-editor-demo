import { Extension } from '@tiptap/core'
import * as Y from 'yjs'

export async function createYjsExtension(yXmlFragment: Y.XmlFragment) {
  const ypm = await import('y-prosemirror')
  const { ySyncPlugin, yUndoPlugin } = ypm

  return Extension.create({
    name: 'yjs',

    // This function runs when the editor is ready and plugins are needed
    addProseMirrorPlugins() {
      const syncPlugin = ySyncPlugin(yXmlFragment)
      
      return [syncPlugin, yUndoPlugin()]
    }
  })
}