import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createElement, type ReactNode } from 'react'

vi.mock('../lib/api', () => ({
  api: {
    getGeminiAuthType: vi.fn(),
    getSlackConnectionStatus: vi.fn(),
  },
}))

import { useSetupStatus } from './useSetupStatus'
import { api } from '../lib/api'

const mockGetGeminiAuthType = api.getGeminiAuthType as ReturnType<typeof vi.fn>
const mockGetSlackConnectionStatus = api.getSlackConnectionStatus as ReturnType<typeof vi.fn>

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

describe('useSetupStatus', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('loading state', () => {
    it('returns isLoading true while queries are pending', () => {
      mockGetGeminiAuthType.mockReturnValue(new Promise(() => {}))
      mockGetSlackConnectionStatus.mockReturnValue(new Promise(() => {}))
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      expect(result.current.isLoading).toBe(true)
      expect(result.current.geminiConfigured).toBe(false)
      expect(result.current.hasConnectedSource).toBe(false)
      expect(result.current.isComplete).toBe(false)
    })
  })

  describe('geminiConfigured', () => {
    it('returns false when gemini auth type is "none"', async () => {
      mockGetGeminiAuthType.mockResolvedValue('none')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: false })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.geminiConfigured).toBe(false)
    })

    it('returns true when gemini auth type is "api_key"', async () => {
      mockGetGeminiAuthType.mockResolvedValue('api_key')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: false })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.geminiConfigured).toBe(true)
    })

    it('returns true when gemini auth type is "service_account"', async () => {
      mockGetGeminiAuthType.mockResolvedValue('service_account')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: false })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.geminiConfigured).toBe(true)
    })
  })

  describe('hasConnectedSource', () => {
    it('returns false when slack is not connected', async () => {
      mockGetGeminiAuthType.mockResolvedValue('none')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: false })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.hasConnectedSource).toBe(false)
    })

    it('returns true when slack is connected', async () => {
      mockGetGeminiAuthType.mockResolvedValue('none')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: true })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.hasConnectedSource).toBe(true)
    })

    it('returns false when slack status is undefined', async () => {
      mockGetGeminiAuthType.mockResolvedValue('none')
      mockGetSlackConnectionStatus.mockResolvedValue({})
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.hasConnectedSource).toBe(false)
    })
  })

  describe('isComplete', () => {
    it('returns false when only gemini is configured', async () => {
      mockGetGeminiAuthType.mockResolvedValue('api_key')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: false })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.isComplete).toBe(false)
    })

    it('returns false when only slack is connected', async () => {
      mockGetGeminiAuthType.mockResolvedValue('none')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: true })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.isComplete).toBe(false)
    })

    it('returns true when both gemini and slack are configured', async () => {
      mockGetGeminiAuthType.mockResolvedValue('api_key')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: true })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.isComplete).toBe(true)
    })

    it('returns false when neither is configured', async () => {
      mockGetGeminiAuthType.mockResolvedValue('none')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: false })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.isComplete).toBe(false)
    })
  })

  describe('isLoading', () => {
    it('returns true when gemini query is still loading', async () => {
      mockGetGeminiAuthType.mockReturnValue(new Promise(() => {}))
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: true })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => {
        expect(result.current.hasConnectedSource).toBe(true)
      })
      expect(result.current.isLoading).toBe(true)
    })

    it('returns true when slack query is still loading', async () => {
      mockGetGeminiAuthType.mockResolvedValue('api_key')
      mockGetSlackConnectionStatus.mockReturnValue(new Promise(() => {}))
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => {
        expect(result.current.geminiConfigured).toBe(true)
      })
      expect(result.current.isLoading).toBe(true)
    })

    it('returns false when both queries are complete', async () => {
      mockGetGeminiAuthType.mockResolvedValue('api_key')
      mockGetSlackConnectionStatus.mockResolvedValue({ connected: true })
      const { wrapper } = createWrapper()

      const { result } = renderHook(() => useSetupStatus(), { wrapper })

      await waitFor(() => expect(result.current.isLoading).toBe(false))
      expect(result.current.isLoading).toBe(false)
    })
  })
})
