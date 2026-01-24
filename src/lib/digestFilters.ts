import { Flame, TrendingDown, Minus, Hash, FileText, Sparkles, type LucideIcon } from 'lucide-react'
import type { DigestItem } from './api'

export type ImportanceLevel = 'high' | 'medium' | 'low'
export type ImportanceFilter = 'all' | ImportanceLevel
export type SourceFilter = 'all' | 'slack' | 'confluence' | 'ai'

/** All importance filter options for UI iteration */
export const IMPORTANCE_LEVELS: readonly ImportanceFilter[] = ['all', 'high', 'medium', 'low'] as const

/** All digest categories for UI iteration */
export const CATEGORIES = ['all', 'engineering', 'product', 'sales', 'marketing', 'research', 'other'] as const

/** UI configuration for importance filter buttons */
export const IMPORTANCE_CONFIG: Record<ImportanceFilter, { label: string; icon: LucideIcon | null; color: string }> = {
  all: { label: 'All', icon: null, color: '' },
  high: { label: 'High', icon: Flame, color: 'text-red-500' },
  medium: { label: 'Medium', icon: Minus, color: 'text-yellow-500' },
  low: { label: 'Low', icon: TrendingDown, color: 'text-blue-500' },
}

/** UI configuration for source filter buttons */
export const SOURCE_CONFIG: Record<SourceFilter, { label: string; icon: LucideIcon | null }> = {
  all: { label: 'All', icon: null },
  slack: { label: 'Slack', icon: Hash },
  confluence: { label: 'Confluence', icon: FileText },
  ai: { label: 'AI', icon: Sparkles },
}

/**
 * Maps an importance score (0-1) to a level category.
 * - High: score >= 0.8
 * - Medium: score >= 0.5
 * - Low: score < 0.5
 */
export function getImportanceLevel(score: number): ImportanceLevel {
  if (score >= 0.8) return 'high'
  if (score >= 0.5) return 'medium'
  return 'low'
}

/**
 * Filters digest items by category, importance, and source.
 * All filters use AND logic.
 */
export function filterDigestItems(
  items: DigestItem[],
  categoryFilter: string,
  importanceFilter: ImportanceFilter,
  sourceFilter: SourceFilter
): DigestItem[] {
  let result = items

  if (categoryFilter !== 'all') {
    result = result.filter(item => item.category.toLowerCase() === categoryFilter)
  }
  if (importanceFilter !== 'all') {
    result = result.filter(item => getImportanceLevel(item.importanceScore) === importanceFilter)
  }
  if (sourceFilter !== 'all') {
    result = result.filter(item => item.source === sourceFilter)
  }

  return result
}

/**
 * Counts items by importance level.
 */
export function countByImportance(items: DigestItem[]): Record<ImportanceFilter, number> {
  return {
    all: items.length,
    high: items.filter(item => getImportanceLevel(item.importanceScore) === 'high').length,
    medium: items.filter(item => getImportanceLevel(item.importanceScore) === 'medium').length,
    low: items.filter(item => getImportanceLevel(item.importanceScore) === 'low').length,
  }
}

/**
 * Counts items by source.
 */
export function countBySource(items: DigestItem[]): Record<SourceFilter, number> {
  return {
    all: items.length,
    slack: items.filter(item => item.source === 'slack').length,
    confluence: items.filter(item => item.source === 'confluence').length,
    ai: items.filter(item => item.source === 'ai').length,
  }
}

/**
 * Returns only the sources that have items.
 */
export function getAvailableSources(sourceCounts: Record<SourceFilter, number>): SourceFilter[] {
  const sources: SourceFilter[] = ['all']
  if (sourceCounts.slack > 0) sources.push('slack')
  if (sourceCounts.confluence > 0) sources.push('confluence')
  if (sourceCounts.ai > 0) sources.push('ai')
  return sources
}

/**
 * Counts the number of active filters.
 */
export function countActiveFilters(
  categoryFilter: string,
  importanceFilter: ImportanceFilter,
  sourceFilter: SourceFilter
): number {
  return (
    (categoryFilter !== 'all' ? 1 : 0) +
    (importanceFilter !== 'all' ? 1 : 0) +
    (sourceFilter !== 'all' ? 1 : 0)
  )
}
