import { useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'

export function useAnalytics() {
  const trackEvent = useCallback(
    async (eventType: string, eventData: Record<string, unknown> = {}) => {
      try {
        await invoke('track_event', { eventType, eventData })
      } catch {
        // Analytics failures should not break the app
      }
    },
    []
  )

  const trackView = useCallback(
    (viewName: string) => {
      trackEvent('view', { view: viewName })
    },
    [trackEvent]
  )

  const trackSourceClick = useCallback(
    (source: string, itemId: string) => {
      trackEvent('source_click', { source, item_id: itemId })
    },
    [trackEvent]
  )

  const trackCategorization = useCallback(
    (itemId: string, fromCategory: string | null, toCategory: string) => {
      trackEvent('categorization', {
        item_id: itemId,
        from_category: fromCategory,
        to_category: toCategory,
      })
    },
    [trackEvent]
  )

  return {
    trackEvent,
    trackView,
    trackSourceClick,
    trackCategorization,
  }
}
