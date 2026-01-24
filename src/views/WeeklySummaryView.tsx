import { useState, useMemo, useCallback } from 'react'
import {
  format,
  startOfWeek,
  subWeeks,
  addWeeks,
  endOfWeek,
  eachDayOfInterval,
  isSameDay,
} from 'date-fns'
import { ChevronLeft, ChevronRight, Calendar, PanelLeftClose, PanelLeft, X } from 'lucide-react'
import { useWeeklyDigest } from '../hooks/useDigest'
import { ContentCard, ContentDetailModal, ExportMenu } from '../components'
import { Button } from '../components/ui/Button'
import { useAppStore } from '../store'
import { exportDigestPDF, type PDFDayGroup } from '../lib/pdf'
import { exportDigestMarkdown, type DayGroup as MarkdownDayGroup } from '../lib/markdown'
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

interface DayGroup {
  date: Date
  items: DigestItem[]
}

export function WeeklySummaryView() {
  const { setView, addLocalActivity, updateLocalActivity } = useAppStore()
  const [weekStart, setWeekStart] = useState(() => startOfWeek(new Date(), { weekStartsOn: 1 }))
  const [filter, setFilter] = useState<string>('all')
  const [importanceFilter, setImportanceFilter] = useState<ImportanceFilter>('all')
  const [sourceFilter, setSourceFilter] = useState<SourceFilter>('all')
  const [selectedItem, setSelectedItem] = useState<DigestItem | null>(null)
  const [isExporting, setIsExporting] = useState(false)
  const [sidebarOpen, setSidebarOpen] = useState(true)

  const weekStartStr = format(weekStart, 'yyyy-MM-dd')
  const weekEnd = endOfWeek(weekStart, { weekStartsOn: 1 })
  // Send timezone offset in minutes (e.g., PST is -480, EST is -300)
  const timezoneOffset = weekStart.getTimezoneOffset()
  const { data, isLoading, error } = useWeeklyDigest(weekStartStr, timezoneOffset)

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

  // Group filtered items by day (newest first), with overview items at top of each day
  const dayGroups = useMemo((): DayGroup[] => {
    const days = eachDayOfInterval({ start: weekStart, end: weekEnd }).reverse()
    return days
      .map(date => {
        const dayItems = filteredItems.filter(item => isSameDay(new Date(item.createdAt), date))
        // Sort items: overview first, then by importance score descending
        dayItems.sort((a, b) => {
          if (a.category === 'overview' && b.category !== 'overview') return -1
          if (a.category !== 'overview' && b.category === 'overview') return 1
          return b.importanceScore - a.importanceScore
        })
        return { date, items: dayItems }
      })
      .filter(group => group.items.length > 0)
  }, [filteredItems, weekStart, weekEnd])

  const totalItems = filteredItems.length

  const handleExportPDF = useCallback(async () => {
    if (!data || filteredItems.length === 0) return
    setIsExporting(true)
    
    const dateLabel = `${format(weekStart, 'MMM d')} - ${format(weekEnd, 'MMM d, yyyy')}`
    const activityId = addLocalActivity({
      type: 'pdf_export',
      message: `Weekly Summary - ${dateLabel}`,
      status: 'running',
    })
    
    try {
      const pdfDayGroups: PDFDayGroup[] = dayGroups.map(group => ({
        date: group.date,
        dateLabel: format(group.date, 'EEEE, MMMM d'),
        items: group.items,
      }))

      await exportDigestPDF({
        digest: data,
        type: 'weekly',
        dateLabel,
        dayGroups: pdfDayGroups,
      })
      updateLocalActivity(activityId, {
        status: 'completed',
        message: `Weekly Summary - ${dateLabel}`,
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
  }, [data, filteredItems, weekStart, weekEnd, dayGroups, addLocalActivity, updateLocalActivity])

  const handleExportMarkdown = useCallback(async () => {
    if (!data || filteredItems.length === 0) return
    setIsExporting(true)
    
    const dateLabel = `${format(weekStart, 'MMM d')} - ${format(weekEnd, 'MMM d, yyyy')}`
    const activityId = addLocalActivity({
      type: 'markdown_export',
      message: `Weekly Summary - ${dateLabel}`,
      status: 'running',
    })
    
    try {
      const markdownDayGroups: MarkdownDayGroup[] = dayGroups.map(group => ({
        date: group.date,
        dateLabel: format(group.date, 'EEEE, MMMM d'),
        items: group.items,
      }))

      await exportDigestMarkdown({
        digest: data,
        type: 'weekly',
        dateLabel,
        dayGroups: markdownDayGroups,
      })
      updateLocalActivity(activityId, {
        status: 'completed',
        message: `Weekly Summary - ${dateLabel}`,
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
  }, [data, filteredItems, weekStart, weekEnd, dayGroups, addLocalActivity, updateLocalActivity])

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
                  const count = cat === 'all'
                    ? data?.items.length ?? 0
                    : data?.items.filter(item => item.category.toLowerCase() === cat).length ?? 0
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
              onClick={() => setWeekStart(d => subWeeks(d, 1))}
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
              onClick={() => setWeekStart(d => addWeeks(d, 1))}
              className="rounded-lg p-2 hover:bg-muted transition-colors"
              aria-label="Next week"
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
        ) : totalItems === 0 ? (
          <div className="bg-card border border-border rounded-xl p-12 text-center">
            <div className="mx-auto w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
              <Calendar className="h-8 w-8 text-muted-foreground" />
            </div>
            <h3 className="text-lg font-semibold text-foreground mb-2">
              {filter === 'all' && importanceFilter === 'all' && sourceFilter === 'all'
                ? 'No items this week'
                : 'No items match filters'}
            </h3>
            <p className="text-muted-foreground max-w-sm mx-auto">
              {filter === 'all' && importanceFilter === 'all' && sourceFilter === 'all'
                ? 'Use the sync button in the header to get the latest updates.'
                : 'Try adjusting your filter selections.'}
            </p>
          </div>
        ) : (
          /* Vertical Timeline */
          <div className="relative">
            {/* Timeline line */}
            <div className="absolute left-4 top-0 bottom-0 w-0.5 bg-border" />

            <div className="space-y-8">
              {dayGroups.map(({ date, items }) => (
                <div key={date.toISOString()} className="relative">
                  {/* Day header with dot */}
                  <div className="flex items-center gap-4 mb-4">
                    <div className="relative z-10 flex h-8 w-8 items-center justify-center rounded-full bg-primary-500 text-white text-sm font-bold">
                      {format(date, 'd')}
                    </div>
                    <div>
                      <h3 className="text-lg font-semibold text-foreground">
                        {format(date, 'EEEE')}
                      </h3>
                      <p className="text-sm text-muted-foreground">
                        {format(date, 'MMMM d, yyyy')} · {items.length} {items.length === 1 ? 'item' : 'items'}
                      </p>
                    </div>
                  </div>

                  {/* Items for this day */}
                  <div className="ml-12 space-y-4">
                    {items.map(item => (
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
                </div>
              ))}
            </div>
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
