import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../lib/api'

export function useDailyDigest(date?: string, timezoneOffset?: number) {
  return useQuery({
    queryKey: ['daily-digest', date, timezoneOffset],
    queryFn: () => api.getDailyDigest(date, timezoneOffset),
    staleTime: 1000 * 60 * 5, // 5 minutes
    retry: 1,
  })
}

export function useWeeklyDigest(weekStart?: string, timezoneOffset?: number) {
  return useQuery({
    queryKey: ['weekly-digest', weekStart, timezoneOffset],
    queryFn: () => api.getWeeklyDigest(weekStart, timezoneOffset),
    staleTime: 1000 * 60 * 15, // 15 minutes
    retry: 1,
  })
}

interface SyncParams {
  sources?: string[]
  timezoneOffset?: number
}

export function useSync() {
  const queryClient = useQueryClient()

  const syncMutation = useMutation({
    mutationFn: async (params?: SyncParams) => {
      // Default to current timezone offset if not provided
      const timezoneOffset = params?.timezoneOffset ?? new Date().getTimezoneOffset()
      return await api.startSync(params?.sources, timezoneOffset)
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['daily-digest'] })
      queryClient.invalidateQueries({ queryKey: ['weekly-digest'] })
    },
  })

  const statusQuery = useQuery({
    queryKey: ['sync-status'],
    queryFn: () => api.getSyncStatus(),
    refetchInterval: (query) => {
      // Poll more frequently while syncing, disable polling otherwise
      const data = query.state.data
      return data?.isSyncing ? 2000 : false
    },
  })

  return {
    sync: (sources?: string[]) => syncMutation.mutate({ sources }),
    isSyncing: syncMutation.isPending || statusQuery.data?.isSyncing,
    status: statusQuery.data,
    error: syncMutation.error || statusQuery.error,
  }
}
