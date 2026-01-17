import { useState } from 'react'
import { format, subDays, addDays, isToday } from 'date-fns'
import { ChevronLeft, ChevronRight, RefreshCw, Calendar } from 'lucide-react'
import { useDailyDigest, useSync } from '../hooks/useDigest'
import { ContentCard } from '../components/ContentCard'
import { Button } from '../components/ui/Button'
import { useAppStore } from '../store'

const CATEGORIES = ['all', 'engineering', 'product', 'sales', 'marketing', 'research', 'other'] as const

export function DailyDigestView() {
  const { setView } = useAppStore()
  const [date, setDate] = useState(new Date())
  const [filter, setFilter] = useState<string>('all')

  const dateStr = format(date, 'yyyy-MM-dd')
  const { data, isLoading, error, refetch } = useDailyDigest(dateStr)
  const { sync, isSyncing } = useSync()

  const filteredItems = data?.items.filter(
    item => filter === 'all' || item.category.toLowerCase() === filter
  ) ?? []

  const handleSync = () => {
    sync(undefined)
    setTimeout(() => refetch(), 1000)
  }

  const canGoForward = !isToday(date)

  return (
    <div className="mx-auto max-w-4xl">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setDate(d => subDays(d, 1))}
            className="rounded-lg p-2 hover:bg-muted transition-colors"
            aria-label="Previous day"
          >
            <ChevronLeft className="h-5 w-5 text-foreground" />
          </button>

          <div className="text-center min-w-[200px]">
            <h2 className="text-2xl font-bold text-foreground">Daily Digest</h2>
            <p className="text-muted-foreground">
              {format(date, 'EEEE, MMMM d, yyyy')}
            </p>
          </div>

          <button
            onClick={() => setDate(d => addDays(d, 1))}
            className="rounded-lg p-2 hover:bg-muted transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={!canGoForward}
            aria-label="Next day"
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

      {/* Category Filter */}
      <div className="mb-6 flex flex-wrap gap-2">
        {CATEGORIES.map(cat => {
          const count = cat !== 'all'
            ? data?.categories.find(c => c.name.toLowerCase() === cat)?.count
            : data?.items.length

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
              {count !== undefined && count > 0 && (
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
            No digest available yet
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
            {filter === 'all' ? 'No items for this day' : `No ${filter} items`}
          </h3>
          <p className="text-muted-foreground max-w-sm mx-auto mb-6">
            {filter === 'all'
              ? 'Sync your accounts to get the latest updates.'
              : 'Try selecting a different category or sync for new updates.'
            }
          </p>
          <Button onClick={handleSync} disabled={isSyncing}>
            <RefreshCw className={`h-4 w-4 ${isSyncing ? 'animate-spin' : ''}`} />
            Sync now
          </Button>
        </div>
      ) : (
        <div className="grid gap-4">
          {filteredItems.map(item => (
            <ContentCard key={item.id} item={item} />
          ))}
        </div>
      )}

      {/* Category Summary Cards - show when no filter */}
      {!isLoading && !error && filter === 'all' && data?.categories && data.categories.length > 0 && filteredItems.length === 0 && (
        <div className="mt-8 grid grid-cols-2 gap-4">
          {data.categories.map((cat) => (
            <div
              key={cat.name}
              className="bg-card border border-border rounded-lg p-4 cursor-pointer hover:shadow-md transition-shadow"
              onClick={() => setFilter(cat.name.toLowerCase())}
            >
              <div className="flex items-center justify-between">
                <h4 className="font-medium text-foreground capitalize">{cat.name}</h4>
                <span className="text-2xl font-bold text-primary-500">{cat.count}</span>
              </div>
              <p className="text-sm text-muted-foreground mt-1">
                {cat.count === 1 ? '1 item' : `${cat.count} items`}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
