import { useState } from 'react'
import { format, startOfWeek, subWeeks, addWeeks, endOfWeek } from 'date-fns'
import { ChevronLeft, ChevronRight, Calendar, TrendingUp } from 'lucide-react'
import { useWeeklyDigest } from '../hooks/useDigest'
import { ContentCard } from '../components/ContentCard'
import { Button } from '../components/ui/Button'
import { useAppStore } from '../store'

const categoryIcons: Record<string, string> = {
  engineering: '🔧',
  product: '📦',
  sales: '💰',
  marketing: '📣',
  research: '🔬',
  other: '📋',
}

export function WeeklySummaryView() {
  const { setView } = useAppStore()
  const [weekStart, setWeekStart] = useState(() => startOfWeek(new Date(), { weekStartsOn: 1 }))

  const weekStartStr = format(weekStart, 'yyyy-MM-dd')
  const weekEnd = endOfWeek(weekStart, { weekStartsOn: 1 })
  const { data, isLoading, error } = useWeeklyDigest(weekStartStr)

  const totalItems = data?.items.length ?? 0

  return (
    <div className="mx-auto max-w-4xl">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setWeekStart(d => subWeeks(d, 1))}
            className="rounded-lg p-2 hover:bg-muted transition-colors"
            aria-label="Previous week"
          >
            <ChevronLeft className="h-5 w-5 text-foreground" />
          </button>

          <div className="text-center min-w-[280px]">
            <h2 className="text-2xl font-bold text-foreground">Weekly Summary</h2>
            <p className="text-muted-foreground">
              {format(weekStart, 'MMM d')} - {format(weekEnd, 'MMM d, yyyy')}
            </p>
          </div>

          <button
            onClick={() => setWeekStart(d => addWeeks(d, 1))}
            className="rounded-lg p-2 hover:bg-muted transition-colors"
            aria-label="Next week"
          >
            <ChevronRight className="h-5 w-5 text-foreground" />
          </button>
        </div>

        {totalItems > 0 && (
          <div className="flex items-center gap-2 text-muted-foreground">
            <TrendingUp className="h-4 w-4" />
            <span className="text-sm font-medium">{totalItems} items this week</span>
          </div>
        )}
      </div>

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
            No weekly summary yet
          </h3>
          <p className="text-muted-foreground max-w-sm mx-auto mb-6">
            Connect your accounts and sync data to generate weekly summaries.
          </p>
          <Button onClick={() => setView('settings')}>
            Connect Accounts
          </Button>
        </div>
      ) : (
        <>
          {/* Category breakdown */}
          {data?.categories && data.categories.length > 0 && (
            <div className="mb-8">
              <h3 className="text-lg font-semibold text-foreground mb-4">Category Breakdown</h3>
              <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                {data.categories.map(cat => (
                  <div
                    key={cat.name}
                    className="rounded-lg border border-border bg-card p-4 hover:shadow-md transition-shadow"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span className="text-xl">
                          {categoryIcons[cat.name.toLowerCase()] || categoryIcons.other}
                        </span>
                        <span className="font-medium capitalize text-foreground">{cat.name}</span>
                      </div>
                      <span className="text-2xl font-bold text-primary-500">{cat.count}</span>
                    </div>
                    {cat.topItems.length > 0 && (
                      <p className="mt-2 text-sm text-muted-foreground line-clamp-2">
                        {cat.topItems[0].title || cat.topItems[0].summary.slice(0, 60)}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Top items this week */}
          {data?.items && data.items.length > 0 ? (
            <div>
              <h3 className="text-lg font-semibold text-foreground mb-4">Top Items This Week</h3>
              <div className="grid gap-4">
                {data.items.slice(0, 10).map(item => (
                  <ContentCard key={item.id} item={item} />
                ))}
              </div>

              {data.items.length > 10 && (
                <p className="mt-4 text-center text-sm text-muted-foreground">
                  Showing top 10 of {data.items.length} items
                </p>
              )}
            </div>
          ) : (
            <div className="bg-card border border-border rounded-xl p-12 text-center">
              <div className="mx-auto w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
                <Calendar className="h-8 w-8 text-muted-foreground" />
              </div>
              <h3 className="text-lg font-semibold text-foreground mb-2">
                No items this week
              </h3>
              <p className="text-muted-foreground max-w-sm mx-auto">
                Sync your accounts to get the latest updates.
              </p>
            </div>
          )}
        </>
      )}
    </div>
  )
}
