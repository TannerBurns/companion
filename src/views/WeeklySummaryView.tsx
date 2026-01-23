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
import { ChevronLeft, ChevronRight, Calendar } from 'lucide-react'
import { useWeeklyDigest } from '../hooks/useDigest'
import { ContentCard, ContentDetailModal, ExportMenu } from '../components'
import { Button } from '../components/ui/Button'
import { useAppStore } from '../store'
import { exportDigestPDF, type PDFDayGroup } from '../lib/pdf'
import { exportDigestMarkdown, type DayGroup as MarkdownDayGroup } from '../lib/markdown'
import type { DigestItem } from '../lib/api'

const CATEGORIES = ['all', 'engineering', 'product', 'sales', 'marketing', 'research', 'other'] as const

interface DayGroup {
  date: Date
  items: DigestItem[]
}

export function WeeklySummaryView() {
  const { setView, addLocalActivity, updateLocalActivity } = useAppStore()
  const [weekStart, setWeekStart] = useState(() => startOfWeek(new Date(), { weekStartsOn: 1 }))
  const [filter, setFilter] = useState<string>('all')
  const [selectedItem, setSelectedItem] = useState<DigestItem | null>(null)
  const [isExporting, setIsExporting] = useState(false)

  const weekStartStr = format(weekStart, 'yyyy-MM-dd')
  const weekEnd = endOfWeek(weekStart, { weekStartsOn: 1 })
  // Send timezone offset in minutes (e.g., PST is -480, EST is -300)
  const timezoneOffset = weekStart.getTimezoneOffset()
  const { data, isLoading, error } = useWeeklyDigest(weekStartStr, timezoneOffset)

  // Filter items by category
  const filteredItems = useMemo(() => {
    const items = data?.items ?? []
    if (filter === 'all') return items
    return items.filter(item => item.category.toLowerCase() === filter)
  }, [data?.items, filter])

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
    if (!data || data.items.length === 0) return
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
  }, [data, weekStart, weekEnd, dayGroups, addLocalActivity, updateLocalActivity])

  const handleExportMarkdown = useCallback(async () => {
    if (!data || data.items.length === 0) return
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
  }, [data, weekStart, weekEnd, dayGroups, addLocalActivity, updateLocalActivity])

  return (
    <div className="mx-auto max-w-4xl">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div className="w-24" /> {/* Spacer for centering */}
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
        <div className="w-24 flex justify-end">
          <ExportMenu
            onExportPDF={handleExportPDF}
            onExportMarkdown={handleExportMarkdown}
            disabled={isLoading || !data || data.items.length === 0}
            isExporting={isExporting}
          />
        </div>
      </div>

      {/* Category Filter */}
      <div className="mb-6 flex flex-wrap gap-2">
        {CATEGORIES.map(cat => {
          const count = cat === 'all'
            ? data?.items.length ?? 0
            : data?.items.filter(item => item.category.toLowerCase() === cat).length ?? 0

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
      ) : totalItems === 0 ? (
        <div className="bg-card border border-border rounded-xl p-12 text-center">
          <div className="mx-auto w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
            <Calendar className="h-8 w-8 text-muted-foreground" />
          </div>
          <h3 className="text-lg font-semibold text-foreground mb-2">
            {filter === 'all' ? 'No items this week' : `No ${filter} items this week`}
          </h3>
          <p className="text-muted-foreground max-w-sm mx-auto">
            {filter === 'all'
              ? 'Use the sync button in the header to get the latest updates.'
              : 'Try selecting a different category.'}
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

      {/* Content Detail Modal */}
      <ContentDetailModal
        item={selectedItem}
        onClose={() => setSelectedItem(null)}
      />
    </div>
  )
}
