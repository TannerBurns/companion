import { useState, useCallback, useMemo } from 'react'
import { format, subDays, addDays, isToday, isFuture } from 'date-fns'
import { ChevronLeft, ChevronRight, Calendar, PanelLeftClose, PanelLeft, X } from 'lucide-react'
import { useDailyDigest } from '../hooks/useDigest'
import { ContentCard, ContentDetailModal, ExportMenu } from '../components'
import { Button } from '../components/ui/Button'
import { useAppStore } from '../store'
import { exportDigestPDF } from '../lib/pdf'
import { exportDigestMarkdown } from '../lib/markdown'
import {
  filterDigestItems,
  countByImportance,
  countBySource,
  getAvailableSources,
  countActiveFilters,
  CATEGORIES,
  IMPORTANCE_LEVELS,
  IMPORTANCE_CONFIG,
  SOURCE_CONFIG,
  type ImportanceFilter,
  type SourceFilter,
} from '../lib/digestFilters'
import type { DigestItem } from '../lib/api'

export function DailyDigestView() {
  const { setView, addLocalActivity, updateLocalActivity } = useAppStore()
  const [date, setDate] = useState(new Date())
  const [filter, setFilter] = useState<string>('all')
  const [importanceFilter, setImportanceFilter] = useState<ImportanceFilter>('all')
  const [sourceFilter, setSourceFilter] = useState<SourceFilter>('all')
  const [selectedItem, setSelectedItem] = useState<DigestItem | null>(null)
  const [isExporting, setIsExporting] = useState(false)
  const [sidebarOpen, setSidebarOpen] = useState(true)

  const dateStr = format(date, 'yyyy-MM-dd')
  // Send timezone offset in minutes (e.g., PST is -480, EST is -300)
  const timezoneOffset = date.getTimezoneOffset()
  const { data, isLoading, error } = useDailyDigest(dateStr, timezoneOffset)

  const filteredItems = useMemo(
    () => filterDigestItems(data?.items ?? [], filter, importanceFilter, sourceFilter),
    [data?.items, filter, importanceFilter, sourceFilter]
  )

  const importanceCounts = useMemo(
    () => countByImportance(data?.items ?? []),
    [data?.items]
  )

  const sourceCounts = useMemo(
    () => countBySource(data?.items ?? []),
    [data?.items]
  )

  const availableSources = useMemo(
    () => getAvailableSources(sourceCounts),
    [sourceCounts]
  )

  const canGoForward = !isToday(date) && !isFuture(date)

  const handleExportPDF = useCallback(async () => {
    if (!data || filteredItems.length === 0) return
    setIsExporting(true)
    
    const dateLabel = format(date, 'MMMM d, yyyy')
    const activityId = addLocalActivity({
      type: 'pdf_export',
      message: `Daily Digest - ${dateLabel}`,
      status: 'running',
    })
    
    try {
      // Create a filtered digest to export only visible items
      const filteredDigest = {
        ...data,
        items: filteredItems,
        categories: data.categories.filter(cat =>
          filteredItems.some(item => item.category.toLowerCase() === cat.name.toLowerCase())
        ),
      }
      await exportDigestPDF({
        digest: filteredDigest,
        type: 'daily',
        dateLabel,
      })
      updateLocalActivity(activityId, {
        status: 'completed',
        message: `Daily Digest - ${dateLabel}`,
      })
    } catch (error) {
      console.error('Failed to export PDF:', error)
      updateLocalActivity(activityId, {
        status: 'failed',
        error: error instanceof Error ? error.message : 'Export failed',
      })
    } finally {
      setIsExporting(false)
    }
  }, [data, filteredItems, date, addLocalActivity, updateLocalActivity])

  const handleExportMarkdown = useCallback(async () => {
    if (!data || filteredItems.length === 0) return
    setIsExporting(true)
    
    const dateLabel = format(date, 'MMMM d, yyyy')
    const activityId = addLocalActivity({
      type: 'markdown_export',
      message: `Daily Digest - ${dateLabel}`,
      status: 'running',
    })
    
    try {
      // Create a filtered digest to export only visible items
      const filteredDigest = {
        ...data,
        items: filteredItems,
        categories: data.categories.filter(cat =>
          filteredItems.some(item => item.category.toLowerCase() === cat.name.toLowerCase())
        ),
      }
      await exportDigestMarkdown({
        digest: filteredDigest,
        type: 'daily',
        dateLabel,
      })
      updateLocalActivity(activityId, {
        status: 'completed',
        message: `Daily Digest - ${dateLabel}`,
      })
    } catch (error) {
      console.error('Failed to export Markdown:', error)
      updateLocalActivity(activityId, {
        status: 'failed',
        error: error instanceof Error ? error.message : 'Export failed',
      })
    } finally {
      setIsExporting(false)
    }
  }, [data, filteredItems, date, addLocalActivity, updateLocalActivity])

  const activeFilterCount = countActiveFilters(filter, importanceFilter, sourceFilter)

  return (
    <div className="flex gap-6">
      {/* Filter Sidebar */}
      <div
        className={`shrink-0 transition-all duration-300 ease-in-out ${
          sidebarOpen ? 'w-56' : 'w-0'
        }`}
      >
        <div
          className={`sticky top-4 transition-opacity duration-300 ${
            sidebarOpen ? 'opacity-100' : 'opacity-0 pointer-events-none'
          }`}
        >
          <div className="bg-card border border-border rounded-xl p-4 space-y-5">
            {/* Sidebar Header */}
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold text-foreground">Filters</h3>
              <div className="flex items-center gap-1">
                {activeFilterCount > 0 && (
                  <button
                    onClick={() => {
                      setFilter('all')
                      setImportanceFilter('all')
                      setSourceFilter('all')
                    }}
                    className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                    title="Clear all filters"
                  >
                    <X className="h-4 w-4" />
                  </button>
                )}
                <button
                  onClick={() => setSidebarOpen(false)}
                  className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                  title="Collapse sidebar"
                >
                  <PanelLeftClose className="h-4 w-4" />
                </button>
              </div>
            </div>

            {/* Category Section */}
            <div className="space-y-2">
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Category</span>
              <div className="space-y-1">
                {CATEGORIES.map(cat => {
                  const count = cat !== 'all'
                    ? data?.categories.find(c => c.name.toLowerCase() === cat)?.count
                    : data?.items.length
                  const isActive = filter === cat

                  return (
                    <button
                      key={cat}
                      onClick={() => setFilter(cat)}
                      className={`w-full flex items-center justify-between rounded-lg px-3 py-2 text-sm font-medium transition-all ${
                        isActive
                          ? 'bg-primary-500/10 text-primary-600 dark:text-primary-400'
                          : 'text-muted-foreground hover:text-foreground hover:bg-muted'
                      }`}
                    >
                      <span>{cat.charAt(0).toUpperCase() + cat.slice(1)}</span>
                      {count !== undefined && count > 0 && (
                        <span className={`text-xs px-1.5 py-0.5 rounded-full ${
                          isActive ? 'bg-primary-500/20' : 'bg-muted'
                        }`}>
                          {count}
                        </span>
                      )}
                    </button>
                  )
                })}
              </div>
            </div>

            {/* Importance Section */}
            <div className="space-y-2">
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Importance</span>
              <div className="space-y-1">
                {IMPORTANCE_LEVELS.map(level => {
                  const config = IMPORTANCE_CONFIG[level]
                  const Icon = config.icon
                  const count = importanceCounts[level]
                  const isActive = importanceFilter === level

                  return (
                    <button
                      key={level}
                      onClick={() => setImportanceFilter(level)}
                      className={`w-full flex items-center justify-between rounded-lg px-3 py-2 text-sm font-medium transition-all ${
                        isActive
                          ? 'bg-primary-500/10 text-primary-600 dark:text-primary-400'
                          : 'text-muted-foreground hover:text-foreground hover:bg-muted'
                      }`}
                    >
                      <span className="flex items-center gap-2">
                        {Icon && <Icon className={`h-4 w-4 ${config.color}`} />}
                        {config.label}
                      </span>
                      {count > 0 && (
                        <span className={`text-xs px-1.5 py-0.5 rounded-full ${
                          isActive ? 'bg-primary-500/20' : 'bg-muted'
                        }`}>
                          {count}
                        </span>
                      )}
                    </button>
                  )
                })}
              </div>
            </div>

            {/* Source Section - Only show if multiple sources available */}
            {availableSources.length > 1 && (
              <div className="space-y-2">
                <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Source</span>
                <div className="space-y-1">
                  {availableSources.map(source => {
                    const config = SOURCE_CONFIG[source]
                    const Icon = config.icon
                    const count = sourceCounts[source]
                    const isActive = sourceFilter === source

                    return (
                      <button
                        key={source}
                        onClick={() => setSourceFilter(source)}
                        className={`w-full flex items-center justify-between rounded-lg px-3 py-2 text-sm font-medium transition-all ${
                          isActive
                            ? 'bg-primary-500/10 text-primary-600 dark:text-primary-400'
                            : 'text-muted-foreground hover:text-foreground hover:bg-muted'
                        }`}
                      >
                        <span className="flex items-center gap-2">
                          {Icon && <Icon className="h-4 w-4" />}
                          {config.label}
                        </span>
                        {count > 0 && (
                          <span className={`text-xs px-1.5 py-0.5 rounded-full ${
                            isActive ? 'bg-primary-500/20' : 'bg-muted'
                          }`}>
                            {count}
                          </span>
                        )}
                      </button>
                    )
                  })}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 min-w-0">
        {/* Header */}
        <div className="mb-6 flex items-center justify-between">
          <div className="flex items-center gap-2">
            {!sidebarOpen && (
              <button
                onClick={() => setSidebarOpen(true)}
                className="p-2 rounded-lg hover:bg-muted transition-colors relative"
                title="Show filters"
              >
                <PanelLeft className="h-5 w-5 text-foreground" />
                {activeFilterCount > 0 && (
                  <span className="absolute -top-1 -right-1 h-4 w-4 rounded-full bg-primary-500 text-white text-xs flex items-center justify-center">
                    {activeFilterCount}
                  </span>
                )}
              </button>
            )}
          </div>
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
          <div className="flex justify-end">
            <ExportMenu
              onExportPDF={handleExportPDF}
              onExportMarkdown={handleExportMarkdown}
              disabled={isLoading || filteredItems.length === 0}
              isExporting={isExporting}
            />
          </div>
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
              {filter === 'all' && importanceFilter === 'all' && sourceFilter === 'all'
                ? 'No items for this day'
                : 'No items match filters'}
            </h3>
            <p className="text-muted-foreground max-w-sm mx-auto">
              {filter === 'all' && importanceFilter === 'all' && sourceFilter === 'all'
                ? 'Use the sync button in the header to get the latest updates.'
                : 'Try adjusting your filter selections.'}
            </p>
          </div>
        ) : (
          <div className="grid gap-4">
            {filteredItems.map(item => (
              <ContentCard
                key={item.id}
                item={item}
                onViewDetail={(id) => {
                  const found = data?.items.find(i => i.id === id)
                  if (found) setSelectedItem(found)
                }}
              />
            ))}
          </div>
        )}
      </div>

      {/* Content Detail Modal */}
      <ContentDetailModal
        item={selectedItem}
        onClose={() => setSelectedItem(null)}
      />
    </div>
  )
}
