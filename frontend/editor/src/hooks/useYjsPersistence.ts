import { useEffect, useState } from 'react'
import { IndexeddbPersistence } from 'y-indexeddb'
import type * as Y from 'yjs'

interface UseYjsPersistenceOptions {
  docId: string
  ydoc: Y.Doc
}

interface UseYjsPersistenceReturn {
  isLocalSynced: boolean
  persistence: IndexeddbPersistence | null
}

export function useYjsPersistence({ docId, ydoc }: UseYjsPersistenceOptions): UseYjsPersistenceReturn {
  const [isLocalSynced, setIsLocalSynced] = useState(false)
  const [persistence, setPersistence] = useState<IndexeddbPersistence | null>(null)

  useEffect(() => {
    const idbPersistence = new IndexeddbPersistence(docId, ydoc)
    idbPersistence.once('synced', () => setIsLocalSynced(true))
    setPersistence(idbPersistence)

    return () => {
      idbPersistence.destroy()
    }
  }, [docId, ydoc])

  return { isLocalSynced, persistence }
}
