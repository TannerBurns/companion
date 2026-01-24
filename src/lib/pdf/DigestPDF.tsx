import {
  Document,
  Page,
  Text,
  View,
  StyleSheet,
  Link,
} from '@react-pdf/renderer'
import type { DigestItem, DigestResponse } from '../api'
import type { PDFDayGroup } from './types'

// Category colors matching the app theme
const categoryColors: Record<string, { bg: string; text: string }> = {
  engineering: { bg: '#dbeafe', text: '#1e40af' },
  product: { bg: '#fce7f3', text: '#9d174d' },
  sales: { bg: '#d1fae5', text: '#065f46' },
  marketing: { bg: '#fef3c7', text: '#92400e' },
  research: { bg: '#ede9fe', text: '#5b21b6' },
  overview: { bg: '#e0e7ff', text: '#3730a3' },
  other: { bg: '#f3f4f6', text: '#374151' },
}

// Source icons as text labels
const sourceLabels: Record<string, string> = {
  slack: 'Slack',
  confluence: 'Confluence',
  ai: 'AI',
}

const styles = StyleSheet.create({
  page: {
    padding: 40,
    fontFamily: 'Helvetica',
    fontSize: 10,
    color: '#1f2937',
  },
  header: {
    marginBottom: 24,
    borderBottom: '1 solid #e5e7eb',
    paddingBottom: 16,
  },
  title: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#111827',
    marginBottom: 4,
  },
  subtitle: {
    fontSize: 12,
    color: '#6b7280',
    marginBottom: 12,
  },
  categorySummary: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
    marginTop: 8,
  },
  categoryBadgeSmall: {
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 10,
    fontSize: 9,
  },
  dayHeader: {
    marginTop: 20,
    marginBottom: 12,
    paddingBottom: 8,
    borderBottom: '1 solid #e5e7eb',
  },
  dayTitle: {
    fontSize: 14,
    fontWeight: 'bold',
    color: '#374151',
  },
  daySubtitle: {
    fontSize: 10,
    color: '#6b7280',
    marginTop: 2,
  },
  itemContainer: {
    marginBottom: 16,
    padding: 12,
    backgroundColor: '#fafafa',
    borderRadius: 6,
    border: '1 solid #e5e7eb',
  },
  itemHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    marginBottom: 8,
  },
  itemTitleRow: {
    flexDirection: 'row',
    alignItems: 'center',
    flex: 1,
    gap: 8,
  },
  itemTitle: {
    fontSize: 12,
    fontWeight: 'bold',
    color: '#111827',
    flex: 1,
  },
  sourceLabel: {
    fontSize: 8,
    color: '#6b7280',
    backgroundColor: '#f3f4f6',
    paddingHorizontal: 6,
    paddingVertical: 2,
    borderRadius: 4,
  },
  categoryBadge: {
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 10,
    fontSize: 9,
  },
  itemSummary: {
    fontSize: 10,
    color: '#374151',
    lineHeight: 1.5,
    marginBottom: 8,
  },
  highlightsContainer: {
    marginTop: 6,
    marginBottom: 8,
    paddingLeft: 8,
  },
  highlightItem: {
    flexDirection: 'row',
    marginBottom: 4,
  },
  highlightBullet: {
    width: 12,
    fontSize: 10,
    color: '#6b7280',
  },
  highlightText: {
    flex: 1,
    fontSize: 9,
    color: '#4b5563',
    lineHeight: 1.4,
  },
  metadataRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    marginTop: 8,
    paddingTop: 8,
    borderTop: '1 solid #e5e7eb',
  },
  metadataLeft: {
    flexDirection: 'column',
    flex: 1,
    marginRight: 12,
  },
  metadataItem: {
    fontSize: 8,
    color: '#6b7280',
    marginBottom: 3,
  },
  importanceContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 4,
  },
  importanceLabel: {
    fontSize: 8,
    color: '#6b7280',
  },
  importanceDots: {
    flexDirection: 'row',
    gap: 2,
  },
  importanceDot: {
    width: 6,
    height: 6,
    borderRadius: 3,
  },
  linksContainer: {
    marginTop: 8,
    paddingTop: 6,
    borderTop: '1 solid #e5e7eb',
  },
  linksLabel: {
    fontSize: 8,
    color: '#6b7280',
    marginBottom: 4,
  },
  linksRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 6,
  },
  linkButton: {
    fontSize: 8,
    color: '#2563eb',
    backgroundColor: '#eff6ff',
    paddingHorizontal: 6,
    paddingVertical: 3,
    borderRadius: 4,
    textDecoration: 'none',
  },
  footer: {
    position: 'absolute',
    bottom: 30,
    left: 40,
    right: 40,
    textAlign: 'center',
    fontSize: 8,
    color: '#9ca3af',
    borderTop: '1 solid #e5e7eb',
    paddingTop: 10,
  },
})

interface ImportanceIndicatorProps {
  score: number
}

function ImportanceIndicator({ score }: ImportanceIndicatorProps) {
  const dots = []
  const normalizedScore = Math.min(5, Math.max(1, Math.round(score)))

  for (let i = 1; i <= 5; i++) {
    dots.push(
      <View
        key={i}
        style={[
          styles.importanceDot,
          {
            backgroundColor: i <= normalizedScore ? '#3b82f6' : '#e5e7eb',
          },
        ]}
      />
    )
  }

  return (
    <View style={styles.importanceContainer}>
      <Text style={styles.importanceLabel}>Importance:</Text>
      <View style={styles.importanceDots}>{dots}</View>
    </View>
  )
}

interface CategoryBadgeProps {
  category: string
}

function CategoryBadge({ category }: CategoryBadgeProps) {
  const colors = categoryColors[category.toLowerCase()] ?? categoryColors.other
  return (
    <Text
      style={[
        styles.categoryBadge,
        { backgroundColor: colors.bg, color: colors.text },
      ]}
    >
      {category.charAt(0).toUpperCase() + category.slice(1)}
    </Text>
  )
}

interface DigestItemRowProps {
  item: DigestItem
}

function DigestItemRow({ item }: DigestItemRowProps) {
  return (
    <View style={styles.itemContainer} wrap={false}>
      <View style={styles.itemHeader}>
        <View style={styles.itemTitleRow}>
          <Text style={styles.sourceLabel}>
            {sourceLabels[item.source] ?? item.source}
          </Text>
          <Text style={styles.itemTitle}>{item.title}</Text>
        </View>
        <CategoryBadge category={item.category} />
      </View>

      <Text style={styles.itemSummary}>{item.summary}</Text>

      {item.highlights && item.highlights.length > 0 && (
        <View style={styles.highlightsContainer}>
          {item.highlights.map((highlight, idx) => (
            <View key={idx} style={styles.highlightItem}>
              <Text style={styles.highlightBullet}>•</Text>
              <Text style={styles.highlightText}>{highlight}</Text>
            </View>
          ))}
        </View>
      )}

      <View style={styles.metadataRow}>
        <View style={styles.metadataLeft}>
          {item.channels && item.channels.length > 0 && (
            <Text style={styles.metadataItem}>
              Channels: {item.channels.join(', ')}
            </Text>
          )}
          {item.people && item.people.length > 0 && (
            <Text style={styles.metadataItem}>
              People: {item.people.join(', ')}
            </Text>
          )}
          {item.messageCount && (
            <Text style={styles.metadataItem}>
              {item.messageCount} messages
            </Text>
          )}
        </View>
        <ImportanceIndicator score={item.importanceScore} />
      </View>

      {/* Source Links */}
      {(item.sourceUrls?.length || item.sourceUrl) && (
        <View style={styles.linksContainer}>
          <Text style={styles.linksLabel}>
            {(item.sourceUrls?.length ?? 0) > 1 ? 'View Original Messages:' : 'View in Slack:'}
          </Text>
          <View style={styles.linksRow}>
            {item.sourceUrls && item.sourceUrls.length > 0 ? (
              item.sourceUrls.map((url, idx) => (
                <Link key={idx} src={url} style={styles.linkButton}>
                  {(item.sourceUrls?.length ?? 0) > 1 ? `Message ${idx + 1}` : 'Open in Slack'}
                </Link>
              ))
            ) : item.sourceUrl ? (
              <Link src={item.sourceUrl} style={styles.linkButton}>
                Open in Slack
              </Link>
            ) : null}
          </View>
        </View>
      )}
    </View>
  )
}

interface DigestPDFProps {
  digest: DigestResponse
  type: 'daily' | 'weekly'
  dateLabel: string
  dayGroups?: PDFDayGroup[]
}

export function DigestPDF({ digest, type, dateLabel, dayGroups }: DigestPDFProps) {
  const title = type === 'daily' ? 'Daily Digest' : 'Weekly Summary'

  // When dayGroups are provided, use those items for counts and categories
  // to ensure consistency when a filter is applied
  const exportedItems = dayGroups && dayGroups.length > 0
    ? dayGroups.flatMap(g => g.items)
    : digest.items
  const itemCount = exportedItems.length

  // Calculate category counts from the actual exported items
  const categoryCounts: Array<{ name: string; count: number }> = []
  const categoryMap = new Map<string, number>()
  for (const item of exportedItems) {
    const cat = item.category
    categoryMap.set(cat, (categoryMap.get(cat) || 0) + 1)
  }
  for (const [name, count] of categoryMap) {
    categoryCounts.push({ name, count })
  }

  return (
    <Document>
      <Page size="A4" style={styles.page}>
        {/* Header */}
        <View style={styles.header}>
          <Text style={styles.title}>{title}</Text>
          <Text style={styles.subtitle}>
            {dateLabel} • {itemCount} {itemCount === 1 ? 'item' : 'items'}
          </Text>
          {categoryCounts.length > 0 && (
            <View style={styles.categorySummary}>
              {categoryCounts.map(cat => {
                const colors = categoryColors[cat.name.toLowerCase()] ?? categoryColors.other
                return (
                  <Text
                    key={cat.name}
                    style={[
                      styles.categoryBadgeSmall,
                      { backgroundColor: colors.bg, color: colors.text },
                    ]}
                  >
                    {cat.name.charAt(0).toUpperCase() + cat.name.slice(1)}: {cat.count}
                  </Text>
                )
              })}
            </View>
          )}
        </View>

        {/* Content - either grouped by day (weekly) or flat list (daily) */}
        {dayGroups && dayGroups.length > 0 ? (
          // Weekly view with day groupings
          dayGroups.map(group => (
            <View key={group.dateLabel}>
              <View style={styles.dayHeader}>
                <Text style={styles.dayTitle}>{group.dateLabel}</Text>
                <Text style={styles.daySubtitle}>
                  {group.items.length} {group.items.length === 1 ? 'item' : 'items'}
                </Text>
              </View>
              {group.items.map(item => (
                <DigestItemRow key={item.id} item={item} />
              ))}
            </View>
          ))
        ) : (
          // Daily view - flat list
          digest.items.map(item => (
            <DigestItemRow key={item.id} item={item} />
          ))
        )}

        {/* Footer */}
        <Text style={styles.footer} fixed>
          Generated by Companion • {new Date().toLocaleDateString()}
        </Text>
      </Page>
    </Document>
  )
}
