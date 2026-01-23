import type { DigestItem } from '../api'
import type { ExportMarkdownOptions } from './types'

function formatSource(source: string): string {
  const sourceMap: Record<string, string> = {
    slack: 'Slack',
    jira: 'Jira',
    confluence: 'Confluence',
    ai: 'AI',
  }
  return sourceMap[source] || source.charAt(0).toUpperCase() + source.slice(1)
}

function formatCategory(category: string): string {
  return category.charAt(0).toUpperCase() + category.slice(1)
}

function formatItem(item: DigestItem): string {
  const lines: string[] = []

  lines.push(`## [${formatSource(item.source)}] ${item.title}`)
  lines.push('')
  lines.push(`**Category:** ${formatCategory(item.category)} | **Importance:** ${item.importanceScore}/5`)
  lines.push('')
  lines.push(item.summary)
  lines.push('')

  if (item.highlights && item.highlights.length > 0) {
    lines.push('**Key Points:**')
    for (const highlight of item.highlights) {
      lines.push(`- ${highlight}`)
    }
    lines.push('')
  }

  const meta: string[] = []
  if (item.channels && item.channels.length > 0) {
    meta.push(`**Channels:** ${item.channels.map(c => `#${c}`).join(', ')}`)
  }
  if (item.people && item.people.length > 0) {
    meta.push(`**People:** ${item.people.map(p => `@${p}`).join(', ')}`)
  }
  if (item.messageCount !== undefined && item.messageCount > 0) {
    meta.push(`**Messages:** ${item.messageCount}`)
  }
  if (item.sourceUrl) {
    meta.push(`**Link:** [View Source](${item.sourceUrl})`)
  }
  if (meta.length > 0) {
    lines.push(meta.join(' | '))
    lines.push('')
  }

  return lines.join('\n')
}

function formatDailyDigest(options: ExportMarkdownOptions): string {
  const { digest, dateLabel } = options
  const lines: string[] = []
  const itemCount = digest.items.length

  lines.push('# Daily Digest')
  lines.push(`**${dateLabel}** - ${itemCount} ${itemCount === 1 ? 'item' : 'items'}`)
  lines.push('')

  // Calculate category counts from the actual exported items
  // to ensure consistency when a filter is applied
  const categoryCounts = new Map<string, number>()
  for (const item of digest.items) {
    const cat = item.category
    categoryCounts.set(cat, (categoryCounts.get(cat) || 0) + 1)
  }

  if (categoryCounts.size > 0) {
    lines.push('## Categories')
    for (const [name, count] of categoryCounts) {
      lines.push(`- ${formatCategory(name)}: ${count}`)
    }
    lines.push('')
  }

  lines.push('---')
  lines.push('')

  for (const item of digest.items) {
    lines.push(formatItem(item))
    lines.push('---')
    lines.push('')
  }

  return lines.join('\n').trim()
}

function formatWeeklyDigest(options: ExportMarkdownOptions): string {
  const { digest, dateLabel, dayGroups } = options
  const lines: string[] = []

  // When dayGroups are provided, use those items for counts and categories
  // to ensure consistency when a filter is applied
  const exportedItems = dayGroups && dayGroups.length > 0
    ? dayGroups.flatMap(g => g.items)
    : digest.items
  const itemCount = exportedItems.length

  lines.push('# Weekly Summary')
  lines.push(`**${dateLabel}** - ${itemCount} ${itemCount === 1 ? 'item' : 'items'}`)
  lines.push('')

  // Calculate category counts from the actual exported items
  const categoryCounts = new Map<string, number>()
  for (const item of exportedItems) {
    const cat = item.category
    categoryCounts.set(cat, (categoryCounts.get(cat) || 0) + 1)
  }

  if (categoryCounts.size > 0) {
    lines.push('## Categories')
    for (const [name, count] of categoryCounts) {
      lines.push(`- ${formatCategory(name)}: ${count}`)
    }
    lines.push('')
  }

  lines.push('---')
  lines.push('')

  if (dayGroups && dayGroups.length > 0) {
    for (const group of dayGroups) {
      lines.push(`# ${group.dateLabel}`)
      lines.push(`${group.items.length} ${group.items.length === 1 ? 'item' : 'items'}`)
      lines.push('')

      for (const item of group.items) {
        lines.push(formatItem(item))
        lines.push('---')
        lines.push('')
      }
    }
  } else {
    for (const item of digest.items) {
      lines.push(formatItem(item))
      lines.push('---')
      lines.push('')
    }
  }

  return lines.join('\n').trim()
}

export function formatDigestMarkdown(options: ExportMarkdownOptions): string {
  if (options.type === 'daily') {
    return formatDailyDigest(options)
  }
  return formatWeeklyDigest(options)
}
