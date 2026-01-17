import { useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'

export interface DigestNotification {
  itemCount: number
  date: string
}

export interface ImportantItemNotification {
  title: string
  source: string
  id: string
}

export interface SyncCompleteNotification {
  itemsSynced: number
  source: string
}

/**
 * Hook to listen for notification events from the backend
 */
export function useNotifications(handlers?: {
  onDigestReady?: (notification: DigestNotification) => void
  onImportantItem?: (notification: ImportantItemNotification) => void
  onSyncComplete?: (notification: SyncCompleteNotification) => void
}) {
  useEffect(() => {
    const unlisteners: Promise<() => void>[] = []

    if (handlers?.onDigestReady) {
      unlisteners.push(
        listen<DigestNotification>('digest:ready', (event) => {
          handlers.onDigestReady?.(event.payload)
        })
      )
    }

    if (handlers?.onImportantItem) {
      unlisteners.push(
        listen<ImportantItemNotification>('item:important', (event) => {
          handlers.onImportantItem?.(event.payload)
        })
      )
    }

    if (handlers?.onSyncComplete) {
      unlisteners.push(
        listen<SyncCompleteNotification>('sync:complete', (event) => {
          handlers.onSyncComplete?.(event.payload)
        })
      )
    }

    return () => {
      unlisteners.forEach((unlisten) => {
        unlisten.then((fn) => fn())
      })
    }
  }, [handlers?.onDigestReady, handlers?.onImportantItem, handlers?.onSyncComplete])
}
