import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createElement, type ReactNode } from 'react'

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}))

import { useSyncCompletedListener } from './useDigest'
import { listen } from '@tauri-apps/api/event'

const mockListen = listen as ReturnType<typeof vi.fn>

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return {
    queryClient,
    wrapper: ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client: queryClient }, children),
  }
}

describe('useSyncCompletedListener', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('sets up listener for sync:completed on mount', () => {
    const unlisten = vi.fn()
    mockListen.mockResolvedValue(unlisten)
    const { wrapper } = createWrapper()

    renderHook(() => useSyncCompletedListener(), { wrapper })

    expect(mockListen).toHaveBeenCalledTimes(1)
    expect(mockListen).toHaveBeenCalledWith('sync:completed', expect.any(Function))
  })

  it('invalidates digest queries when sync:completed event fires', async () => {
    const unlisten = vi.fn()
    let eventCallback: () => void = () => {}
    mockListen.mockImplementation((_event: string, callback: () => void) => {
      eventCallback = callback
      return Promise.resolve(unlisten)
    })
    const { queryClient, wrapper } = createWrapper()
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries')

    renderHook(() => useSyncCompletedListener(), { wrapper })
    await vi.waitFor(() => expect(mockListen).toHaveBeenCalled())

    eventCallback()

    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['daily-digest'] })
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['weekly-digest'] })
  })

  it('cleans up listener on unmount', async () => {
    const unlisten = vi.fn()
    mockListen.mockResolvedValue(unlisten)
    const { wrapper } = createWrapper()

    const { unmount } = renderHook(() => useSyncCompletedListener(), { wrapper })
    await vi.waitFor(() => expect(mockListen).toHaveBeenCalled())

    unmount()

    await vi.waitFor(() => expect(unlisten).toHaveBeenCalled())
  })

  it('calls unlisten immediately if unmounted before listen resolves', async () => {
    const unlisten = vi.fn()
    let resolvePromise: (fn: () => void) => void = () => {}
    mockListen.mockReturnValue(
      new Promise((resolve) => {
        resolvePromise = resolve
      })
    )
    const { wrapper } = createWrapper()

    const { unmount } = renderHook(() => useSyncCompletedListener(), { wrapper })
    unmount()
    resolvePromise(unlisten)

    await vi.waitFor(() => expect(unlisten).toHaveBeenCalled())
  })

  it('logs warning when listen fails', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const error = new Error('Tauri not available')
    mockListen.mockRejectedValue(error)
    const { wrapper } = createWrapper()

    renderHook(() => useSyncCompletedListener(), { wrapper })

    await vi.waitFor(() => {
      expect(warnSpy).toHaveBeenCalledWith(
        'Failed to set up sync:completed listener:',
        error
      )
    })

    warnSpy.mockRestore()
  })
})
