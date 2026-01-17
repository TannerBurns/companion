import { useEffect, useRef } from 'react'
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

type NotificationHandlers = {
  onDigestReady?: (notification: DigestNotification) => void
  onImportantItem?: (notification: ImportantItemNotification) => void
  onSyncComplete?: (notification: SyncCompleteNotification) => void
}

export function useNotifications(handlers?: NotificationHandlers) {
  const handlersRef = useRef(handlers)

  useEffect(() => {
    handlersRef.current = handlers
  })

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = []

    unlisteners.push(
      listen<DigestNotification>('digest:ready', (event) => {
        handlersRef.current?.onDigestReady?.(event.payload)
      })
    )

    unlisteners.push(
      listen<ImportantItemNotification>('item:important', (event) => {
        handlersRef.current?.onImportantItem?.(event.payload)
      })
    )

    unlisteners.push(
      listen<SyncCompleteNotification>('sync:complete', (event) => {
        handlersRef.current?.onSyncComplete?.(event.payload)
      })
    )

    return () => {
      unlisteners.forEach((unlisten) => {
        unlisten.then((fn) => fn())
      })
    }
  }, [])
}
