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

  lines.push('# Daily Digest')
  lines.push(`**${dateLabel}** - ${digest.items.length} items`)
  lines.push('')

  if (digest.categories.length > 0) {
    lines.push('## Categories')
    for (const cat of digest.categories) {
      lines.push(`- ${formatCategory(cat.name)}: ${cat.count}`)
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

  lines.push('# Weekly Summary')
  lines.push(`**${dateLabel}** - ${digest.items.length} items`)
  lines.push('')

  if (digest.categories.length > 0) {
    lines.push('## Categories')
    for (const cat of digest.categories) {
      lines.push(`- ${formatCategory(cat.name)}: ${cat.count}`)
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
