import * as Y from 'yjs'

// Dynamic import for y-prosemirror
let ySyncPlugin: any
let yUndoPlugin: any

const loadYProsemirror = async () => {
  if (!ySyncPlugin) {
    const ypm = await import('y-prosemirror')
    ySyncPlugin = ypm.ySyncPlugin
    yUndoPlugin = ypm.yUndoPlugin
  }
  return { ySyncPlugin, yUndoPlugin }
}

/**
 * Creates y-prosemirror plugins for Yjs collaboration
 * Call this and add the returned plugins to editorProps.plugins
 */
export async function createYjsPlugins(yXmlFragment: Y.XmlFragment): Promise<any[]> {
  const { ySyncPlugin: ySync, yUndoPlugin: yUndo } = await loadYProsemirror()
  return [
    ySync(yXmlFragment),
    yUndo(),
  ]
}

