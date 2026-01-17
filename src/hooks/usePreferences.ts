import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import type { Preferences } from '../lib/api'
import { api } from '../lib/api'

const DEFAULT_PREFERENCES: Preferences = {
  syncIntervalMinutes: 15,
  enabledSources: [],
  enabledCategories: ['sales', 'marketing', 'product', 'engineering', 'research'],
  notificationsEnabled: true,
}

export function usePreferences() {
  const queryClient = useQueryClient()

  const query = useQuery({
    queryKey: ['preferences'],
    queryFn: () => api.getPreferences(),
    staleTime: Infinity,
  })

  const mutation = useMutation({
    mutationFn: (preferences: Preferences) => api.savePreferences(preferences),
    onMutate: async (newPreferences) => {
      await queryClient.cancelQueries({ queryKey: ['preferences'] })
      const previous = queryClient.getQueryData<Preferences>(['preferences'])
      queryClient.setQueryData(['preferences'], newPreferences)
      return { previous }
    },
    onError: (_err, _newPrefs, context) => {
      if (context?.previous) {
        queryClient.setQueryData(['preferences'], context.previous)
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ['preferences'] })
    },
  })

  // Use query data or defaults (never undefined)
  const preferences = query.data ?? DEFAULT_PREFERENCES

  return {
    preferences,
    isLoading: query.isLoading,
    error: query.error,
    save: mutation.mutate,
    isSaving: mutation.isPending,
    saveError: mutation.error,
  }
}

export function useApiKey(service: string) {
  const queryClient = useQueryClient()

  const query = useQuery({
    queryKey: ['apiKey', service],
    queryFn: () => api.hasApiKey(service),
    staleTime: Infinity,
  })

  const mutation = useMutation({
    mutationFn: ({ service, apiKey }: { service: string; apiKey: string }) =>
      api.saveApiKey(service, apiKey),
    onSuccess: () => {
      // Invalidate the query to refetch the key status
      queryClient.invalidateQueries({ queryKey: ['apiKey', service] })
    },
  })

  return {
    hasKey: query.data ?? false,
    isLoading: query.isLoading,
    saveApiKey: (apiKey: string) => mutation.mutate({ service, apiKey }),
    isSaving: mutation.isPending,
    error: mutation.error,
    isSuccess: mutation.isSuccess,
  }
}
