import { useState, useRef, useEffect, useMemo } from 'react'
import {
  format,
  startOfWeek,
  subWeeks,
  addWeeks,
  endOfWeek,
  eachDayOfInterval,
  isSameDay,
} from 'date-fns'
import { ChevronLeft, ChevronRight, RefreshCw, Calendar } from 'lucide-react'
import { useWeeklyDigest, useSync } from '../hooks/useDigest'
import { ContentCard } from '../components/ContentCard'
import { Button } from '../components/ui/Button'
import { useAppStore } from '../store'
import type { DigestItem } from '../lib/api'

const CATEGORIES = ['all', 'engineering', 'product', 'sales', 'marketing', 'research', 'other'] as const

interface DayData {
  date: Date
  items: DigestItem[]
  count: number
}

export function WeeklySummaryView() {
  const { setView } = useAppStore()
  const [weekStart, setWeekStart] = useState(() => startOfWeek(new Date(), { weekStartsOn: 1 }))
  const [filter, setFilter] = useState<string>('all')
  const [selectedDay, setSelectedDay] = useState<Date | null>(null)

  const weekStartStr = format(weekStart, 'yyyy-MM-dd')
  const weekEnd = endOfWeek(weekStart, { weekStartsOn: 1 })
  const { data, isLoading, error, refetch } = useWeeklyDigest(weekStartStr)
  const { sync, isSyncing } = useSync()

  const syncTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    return () => {
      if (syncTimeoutRef.current) {
        clearTimeout(syncTimeoutRef.current)
      }
    }
  }, [])

  const handleSync = () => {
    sync(undefined)
    syncTimeoutRef.current = setTimeout(() => refetch(), 1000)
  }

  // Group items by day
  const dayData = useMemo((): DayData[] => {
    const days = eachDayOfInterval({ start: weekStart, end: weekEnd })
    return days.map(date => {
      const items = data?.items.filter(item => {
        const itemDate = new Date(item.createdAt)
        return isSameDay(itemDate, date)
      }) ?? []
      return { date, items, count: items.length }
    })
  }, [data?.items, weekStart, weekEnd])

  // Filter items by category and optionally by selected day
  const filteredItems = useMemo(() => {
    let items = data?.items ?? []
    
    if (selectedDay) {
      items = items.filter(item => isSameDay(new Date(item.createdAt), selectedDay))
    }
    
    if (filter !== 'all') {
      items = items.filter(item => item.category.toLowerCase() === filter)
    }
    
    return items
  }, [data?.items, filter, selectedDay])

  const maxDayCount = Math.max(...dayData.map(d => d.count), 1)

  return (
    <div className="mx-auto max-w-4xl">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <button
            onClick={() => {
              setWeekStart(d => subWeeks(d, 1))
              setSelectedDay(null)
            }}
            className="rounded-lg p-2 hover:bg-muted transition-colors"
            aria-label="Previous week"
          >
            <ChevronLeft className="h-5 w-5 text-foreground" />
          </button>

          <div className="text-center min-w-[240px]">
            <h2 className="text-2xl font-bold text-foreground">Weekly Summary</h2>
            <p className="text-muted-foreground">
              {format(weekStart, 'MMM d')} - {format(weekEnd, 'MMM d, yyyy')}
            </p>
          </div>

          <button
            onClick={() => {
              setWeekStart(d => addWeeks(d, 1))
              setSelectedDay(null)
            }}
            className="rounded-lg p-2 hover:bg-muted transition-colors"
            aria-label="Next week"
          >
            <ChevronRight className="h-5 w-5 text-foreground" />
          </button>
        </div>

        <Button
          onClick={handleSync}
          disabled={isSyncing}
          variant="default"
          size="md"
        >
          <RefreshCw className={`h-4 w-4 ${isSyncing ? 'animate-spin' : ''}`} />
          {isSyncing ? 'Syncing...' : 'Sync'}
        </Button>
      </div>

      {/* Day-by-day Timeline */}
      <div className="mb-6 p-4 bg-card border border-border rounded-lg">
        <div className="flex items-end justify-between gap-2">
          {dayData.map(({ date, count }) => {
            const isSelected = selectedDay && isSameDay(selectedDay, date)
            const heightPercent = maxDayCount > 0 ? (count / maxDayCount) * 100 : 0
            
            return (
              <button
                key={date.toISOString()}
                onClick={() => setSelectedDay(isSelected ? null : date)}
                className={`flex-1 flex flex-col items-center gap-2 p-2 rounded-lg transition-colors ${
                  isSelected
                    ? 'bg-primary-500/10 ring-2 ring-primary-500'
                    : 'hover:bg-muted'
                }`}
              >
                <div className="w-full flex items-end justify-center h-16">
                  <div
                    className={`w-8 rounded-t transition-all ${
                      count > 0
                        ? isSelected
                          ? 'bg-primary-500'
                          : 'bg-primary-500/60'
                        : 'bg-muted'
                    }`}
                    style={{ height: `${Math.max(heightPercent, 8)}%` }}
                  />
                </div>
                <div className="text-center">
                  <p className="text-xs text-muted-foreground">{format(date, 'EEE')}</p>
                  <p className={`text-sm font-medium ${isSelected ? 'text-primary-500' : 'text-foreground'}`}>
                    {format(date, 'd')}
                  </p>
                </div>
                {count > 0 && (
                  <span className={`text-xs font-medium ${isSelected ? 'text-primary-500' : 'text-muted-foreground'}`}>
                    {count}
                  </span>
                )}
              </button>
            )
          })}
        </div>
        {selectedDay && (
          <div className="mt-3 pt-3 border-t border-border flex items-center justify-between">
            <p className="text-sm text-muted-foreground">
              Showing items from <span className="font-medium text-foreground">{format(selectedDay, 'EEEE, MMM d')}</span>
            </p>
            <button
              onClick={() => setSelectedDay(null)}
              className="text-sm text-primary-500 hover:underline"
            >
              Clear filter
            </button>
          </div>
        )}
      </div>

      {/* Category Filter */}
      <div className="mb-6 flex flex-wrap gap-2">
        {CATEGORIES.map(cat => {
          const baseItems = selectedDay
            ? data?.items.filter(item => isSameDay(new Date(item.createdAt), selectedDay)) ?? []
            : data?.items ?? []
          
          const count = cat === 'all'
            ? baseItems.length
            : baseItems.filter(item => item.category.toLowerCase() === cat).length

          return (
            <button
              key={cat}
              onClick={() => setFilter(cat)}
              className={`rounded-full px-3 py-1.5 text-sm font-medium transition-colors ${
                filter === cat
                  ? 'bg-primary-500 text-white'
                  : 'bg-muted text-muted-foreground hover:bg-muted/80'
              }`}
            >
              {cat.charAt(0).toUpperCase() + cat.slice(1)}
              {count > 0 && (
                <span className="ml-1 opacity-70">({count})</span>
              )}
            </button>
          )
        })}
      </div>

      {/* Content */}
      {isLoading ? (
        <div className="flex h-64 items-center justify-center">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent" />
        </div>
      ) : error ? (
        <div className="bg-card border border-border rounded-xl p-12 text-center">
          <div className="mx-auto w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
            <Calendar className="h-8 w-8 text-muted-foreground" />
          </div>
          <h3 className="text-lg font-semibold text-foreground mb-2">
            No weekly summary available
          </h3>
          <p className="text-muted-foreground max-w-sm mx-auto mb-6">
            Connect your Slack and Atlassian accounts to start receiving AI-powered
            summaries of your work.
          </p>
          <Button onClick={() => setView('settings')}>
            Connect Accounts
          </Button>
        </div>
      ) : filteredItems.length === 0 ? (
        <div className="bg-card border border-border rounded-xl p-12 text-center">
          <div className="mx-auto w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
            <Calendar className="h-8 w-8 text-muted-foreground" />
          </div>
          <h3 className="text-lg font-semibold text-foreground mb-2">
            {selectedDay
              ? `No ${filter === 'all' ? '' : filter + ' '}items on ${format(selectedDay, 'EEEE')}`
              : filter === 'all'
              ? 'No items this week'
              : `No ${filter} items this week`}
          </h3>
          <p className="text-muted-foreground max-w-sm mx-auto mb-6">
            {selectedDay
              ? 'Try selecting a different day or clear the filter.'
              : 'Sync your accounts to get the latest updates.'}
          </p>
          {!selectedDay && (
            <Button onClick={handleSync} disabled={isSyncing}>
              <RefreshCw className={`h-4 w-4 ${isSyncing ? 'animate-spin' : ''}`} />
              Sync now
            </Button>
          )}
        </div>
      ) : (
        <div className="grid gap-4">
          {filteredItems.map(item => (
            <ContentCard key={item.id} item={item} />
          ))}
        </div>
      )}
    </div>
  )
}
