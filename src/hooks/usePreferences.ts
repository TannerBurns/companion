import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import type { Preferences } from '../lib/api'
import { api } from '../lib/api'

export function usePreferences() {
  const queryClient = useQueryClient()

  const query = useQuery({
    queryKey: ['preferences'],
    queryFn: () => api.getPreferences(),
    staleTime: Infinity, // Preferences don't change unless we change them
  })

  const mutation = useMutation({
    mutationFn: (preferences: Preferences) => api.savePreferences(preferences),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['preferences'] })
    },
  })

  return {
    preferences: query.data,
    isLoading: query.isLoading,
    error: query.error,
    save: mutation.mutate,
    isSaving: mutation.isPending,
    saveError: mutation.error,
  }
}

export function useApiKey() {
  const mutation = useMutation({
    mutationFn: ({ service, apiKey }: { service: string; apiKey: string }) =>
      api.saveApiKey(service, apiKey),
  })

  return {
    saveApiKey: (service: string, apiKey: string) => mutation.mutate({ service, apiKey }),
    isSaving: mutation.isPending,
    error: mutation.error,
    isSuccess: mutation.isSuccess,
  }
}
