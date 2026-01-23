import { useQuery } from '@tanstack/react-query'
import { api } from '../lib/api'

export interface SetupStatus {
  geminiConfigured: boolean
  hasConnectedSource: boolean
  isComplete: boolean
  isLoading: boolean
}

export function useSetupStatus(): SetupStatus {
  const geminiQuery = useQuery({
    queryKey: ['gemini-auth-type'],
    queryFn: () => api.getGeminiAuthType(),
    staleTime: 30_000,
  })

  const slackQuery = useQuery({
    queryKey: ['slack-connection-status'],
    queryFn: () => api.getSlackConnectionStatus(),
    staleTime: 30_000,
  })

  const geminiConfigured = geminiQuery.data !== undefined && geminiQuery.data !== 'none'
  const hasConnectedSource = slackQuery.data?.connected === true
  const isLoading = geminiQuery.isLoading || slackQuery.isLoading

  return {
    geminiConfigured,
    hasConnectedSource,
    isComplete: geminiConfigured && hasConnectedSource,
    isLoading,
  }
}
