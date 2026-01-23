import { describe, it, expect } from 'vitest'
import { formatDigestMarkdown } from './formatMarkdown'
import type { DigestItem, DigestResponse } from '../api'
import type { DayGroup } from './types'

function createItem(overrides: Partial<DigestItem> = {}): DigestItem {
  return {
    id: '1',
    title: 'Test Item',
    summary: 'This is a test summary.',
    category: 'engineering',
    source: 'slack',
    importanceScore: 3,
    createdAt: Date.now(),
    ...overrides,
  }
}

function createDigest(overrides: Partial<DigestResponse> = {}): DigestResponse {
  return {
    date: '2024-01-15',
    items: [createItem()],
    categories: [{ name: 'engineering', count: 1, topItems: [] }],
    ...overrides,
  }
}

describe('formatDigestMarkdown', () => {
  describe('daily digest', () => {
    it('formats a basic daily digest with header and item count', () => {
      const result = formatDigestMarkdown({
        digest: createDigest(),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('# Daily Digest')
      expect(result).toContain('**January 15, 2024** - 1 item')
    })

    it('includes category summary when categories exist', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({
          items: [
            createItem({ id: '1', category: 'engineering' }),
            createItem({ id: '2', category: 'engineering' }),
            createItem({ id: '3', category: 'product' }),
            createItem({ id: '4', category: 'product' }),
            createItem({ id: '5', category: 'product' }),
          ],
          categories: [
            { name: 'engineering', count: 2, topItems: [] },
            { name: 'product', count: 3, topItems: [] },
          ],
        }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('## Categories')
      expect(result).toContain('- Engineering: 2')
      expect(result).toContain('- Product: 3')
    })

    it('uses category counts from items when filter applied', () => {
      // Simulate a filter being applied: digest.categories has stale counts
      // but digest.items contains only the filtered subset
      const result = formatDigestMarkdown({
        digest: createDigest({
          // After filtering, only 3 engineering items are in the digest
          items: [
            createItem({ id: '1', title: 'Engineering Item 1', category: 'engineering' }),
            createItem({ id: '2', title: 'Engineering Item 2', category: 'engineering' }),
            createItem({ id: '3', title: 'Engineering Item 3', category: 'engineering' }),
          ],
          // Original categories had higher counts before filtering
          categories: [
            { name: 'engineering', count: 10, topItems: [] },
          ],
        }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      // Should use count from actual items (3), not stale category count (10)
      expect(result).toContain('**January 15, 2024** - 3 items')
      expect(result).toContain('- Engineering: 3')
      expect(result).not.toContain('- Engineering: 10')
    })

    it('omits category section when no items', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [], categories: [] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).not.toContain('## Categories')
    })

    it('formats multiple items with separators', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({
          items: [
            createItem({ id: '1', title: 'First Item' }),
            createItem({ id: '2', title: 'Second Item' }),
          ],
        }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('First Item')
      expect(result).toContain('Second Item')
      expect(result.match(/---/g)?.length).toBeGreaterThanOrEqual(3)
    })
  })

  describe('weekly digest', () => {
    it('formats a basic weekly summary', () => {
      const result = formatDigestMarkdown({
        digest: createDigest(),
        type: 'weekly',
        dateLabel: 'Jan 15 - Jan 21, 2024',
      })

      expect(result).toContain('# Weekly Summary')
      expect(result).toContain('**Jan 15 - Jan 21, 2024** - 1 item')
    })

    it('groups items by day when dayGroups provided', () => {
      const dayGroups: DayGroup[] = [
        {
          date: new Date('2024-01-15'),
          dateLabel: 'Monday, January 15',
          items: [createItem({ title: 'Monday Item' })],
        },
        {
          date: new Date('2024-01-16'),
          dateLabel: 'Tuesday, January 16',
          items: [createItem({ title: 'Tuesday Item' })],
        },
      ]

      const result = formatDigestMarkdown({
        digest: createDigest({ items: [...dayGroups[0].items, ...dayGroups[1].items] }),
        type: 'weekly',
        dateLabel: 'Jan 15 - Jan 21, 2024',
        dayGroups,
      })

      expect(result).toContain('# Monday, January 15')
      expect(result).toContain('1 item')
      expect(result).toContain('Monday Item')
      expect(result).toContain('# Tuesday, January 16')
      expect(result).toContain('Tuesday Item')
    })

    it('uses plural "items" for multiple items in a day', () => {
      const dayGroups: DayGroup[] = [
        {
          date: new Date('2024-01-15'),
          dateLabel: 'Monday, January 15',
          items: [
            createItem({ id: '1', title: 'Item 1' }),
            createItem({ id: '2', title: 'Item 2' }),
          ],
        },
      ]

      const result = formatDigestMarkdown({
        digest: createDigest({ items: dayGroups[0].items }),
        type: 'weekly',
        dateLabel: 'Jan 15 - Jan 21, 2024',
        dayGroups,
      })

      expect(result).toContain('2 items')
    })

    it('falls back to flat list when no dayGroups', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({
          items: [
            createItem({ title: 'Item A' }),
            createItem({ title: 'Item B' }),
          ],
        }),
        type: 'weekly',
        dateLabel: 'Jan 15 - Jan 21, 2024',
      })

      expect(result).toContain('Item A')
      expect(result).toContain('Item B')
      expect(result).not.toMatch(/# (Monday|Tuesday|Wednesday)/)
    })

    it('uses category counts from dayGroups items when filter applied', () => {
      // Simulate a filter being applied: digest has items from multiple categories,
      // but dayGroups only contains the filtered subset
      const dayGroups: DayGroup[] = [
        {
          date: new Date('2024-01-15'),
          dateLabel: 'Monday, January 15',
          items: [
            createItem({ id: '1', title: 'Engineering Item 1', category: 'engineering' }),
            createItem({ id: '2', title: 'Engineering Item 2', category: 'engineering' }),
          ],
        },
      ]

      const result = formatDigestMarkdown({
        digest: createDigest({
          // Original digest has 4 items across multiple categories
          items: [
            createItem({ id: '1', category: 'engineering' }),
            createItem({ id: '2', category: 'engineering' }),
            createItem({ id: '3', category: 'product' }),
            createItem({ id: '4', category: 'sales' }),
          ],
          categories: [
            { name: 'engineering', count: 2, topItems: [] },
            { name: 'product', count: 1, topItems: [] },
            { name: 'sales', count: 1, topItems: [] },
          ],
        }),
        type: 'weekly',
        dateLabel: 'Jan 15 - Jan 21, 2024',
        dayGroups,
      })

      // Should use item count from dayGroups (2), not digest (4)
      expect(result).toContain('**Jan 15 - Jan 21, 2024** - 2 items')
      // Should only include category from filtered items
      expect(result).toContain('- Engineering: 2')
      // Should NOT include categories not in the filtered dayGroups
      expect(result).not.toContain('- Product:')
      expect(result).not.toContain('- Sales:')
    })
  })

  describe('item formatting', () => {
    it('formats source labels correctly', () => {
      const sources: Array<{ source: DigestItem['source']; expected: string }> = [
        { source: 'slack', expected: '[Slack]' },
        { source: 'jira', expected: '[Jira]' },
        { source: 'confluence', expected: '[Confluence]' },
        { source: 'ai', expected: '[AI]' },
      ]

      for (const { source, expected } of sources) {
        const result = formatDigestMarkdown({
          digest: createDigest({ items: [createItem({ source })] }),
          type: 'daily',
          dateLabel: 'January 15, 2024',
        })
        expect(result).toContain(expected)
      }
    })

    it('capitalizes category names', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [createItem({ category: 'engineering' })] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('**Category:** Engineering')
    })

    it('includes importance score', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [createItem({ importanceScore: 4 })] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('**Importance:** 4/5')
    })

    it('includes summary text', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [createItem({ summary: 'Detailed summary here.' })] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('Detailed summary here.')
    })

    it('formats highlights as key points', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({
          items: [createItem({ highlights: ['Point one', 'Point two', 'Point three'] })],
        }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('**Key Points:**')
      expect(result).toContain('- Point one')
      expect(result).toContain('- Point two')
      expect(result).toContain('- Point three')
    })

    it('omits key points section when no highlights', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [createItem({ highlights: undefined })] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).not.toContain('**Key Points:**')
    })

    it('formats channels with # prefix', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({
          items: [createItem({ channels: ['general', 'engineering'] })],
        }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('**Channels:** #general, #engineering')
    })

    it('formats people with @ prefix', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({
          items: [createItem({ people: ['alice', 'bob'] })],
        }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('**People:** @alice, @bob')
    })

    it('includes message count when positive', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [createItem({ messageCount: 42 })] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('**Messages:** 42')
    })

    it('omits message count when zero', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [createItem({ messageCount: 0 })] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).not.toContain('**Messages:**')
    })

    it('formats source URL as link', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({
          items: [createItem({ sourceUrl: 'https://example.com/item' })],
        }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('**Link:** [View Source](https://example.com/item)')
    })

    it('omits metadata line when no optional fields present', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({
          items: [createItem({
            channels: undefined,
            people: undefined,
            messageCount: undefined,
            sourceUrl: undefined,
          })],
        }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).not.toContain('**Channels:**')
      expect(result).not.toContain('**People:**')
      expect(result).not.toContain('**Messages:**')
      expect(result).not.toContain('**Link:**')
    })
  })

  describe('edge cases', () => {
    it('handles empty items array', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).toContain('# Daily Digest')
      expect(result).toContain('0 items')
    })

    it('handles empty highlights array', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [createItem({ highlights: [] })] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).not.toContain('**Key Points:**')
    })

    it('handles empty channels array', () => {
      const result = formatDigestMarkdown({
        digest: createDigest({ items: [createItem({ channels: [] })] }),
        type: 'daily',
        dateLabel: 'January 15, 2024',
      })

      expect(result).not.toContain('**Channels:**')
    })

    it('handles empty dayGroups array for weekly', () => {
      const result = formatDigestMarkdown({
        digest: createDigest(),
        type: 'weekly',
        dateLabel: 'Jan 15 - Jan 21, 2024',
        dayGroups: [],
      })

      expect(result).toContain('# Weekly Summary')
      expect(result).toContain('Test Item')
    })
  })
})
