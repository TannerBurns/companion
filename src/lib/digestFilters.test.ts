import { describe, it, expect } from 'vitest'
import {
  getImportanceLevel,
  filterDigestItems,
  countByImportance,
  countBySource,
  getAvailableSources,
  countActiveFilters,
} from './digestFilters'
import type { DigestItem } from './api'

// Helper to create test items
function createItem(overrides: Partial<DigestItem> = {}): DigestItem {
  return {
    id: 'test-id',
    title: 'Test Item',
    summary: 'Test summary',
    category: 'engineering',
    source: 'slack',
    importanceScore: 0.5,
    createdAt: Date.now(),
    ...overrides,
  }
}

describe('getImportanceLevel', () => {
  describe('high threshold (>= 0.8)', () => {
    it('returns high for score of 0.8', () => {
      expect(getImportanceLevel(0.8)).toBe('high')
    })

    it('returns high for score of 1.0', () => {
      expect(getImportanceLevel(1.0)).toBe('high')
    })

    it('returns high for score of 0.95', () => {
      expect(getImportanceLevel(0.95)).toBe('high')
    })
  })

  describe('medium threshold (>= 0.5, < 0.8)', () => {
    it('returns medium for score of 0.5', () => {
      expect(getImportanceLevel(0.5)).toBe('medium')
    })

    it('returns medium for score of 0.79', () => {
      expect(getImportanceLevel(0.79)).toBe('medium')
    })

    it('returns medium for score of 0.65', () => {
      expect(getImportanceLevel(0.65)).toBe('medium')
    })
  })

  describe('low threshold (< 0.5)', () => {
    it('returns low for score of 0.49', () => {
      expect(getImportanceLevel(0.49)).toBe('low')
    })

    it('returns low for score of 0', () => {
      expect(getImportanceLevel(0)).toBe('low')
    })

    it('returns low for score of 0.25', () => {
      expect(getImportanceLevel(0.25)).toBe('low')
    })
  })

  describe('edge cases', () => {
    it('handles negative scores as low', () => {
      expect(getImportanceLevel(-0.5)).toBe('low')
    })

    it('handles scores above 1 as high', () => {
      expect(getImportanceLevel(1.5)).toBe('high')
    })
  })
})

describe('filterDigestItems', () => {
  const items: DigestItem[] = [
    createItem({ id: '1', category: 'Engineering', source: 'slack', importanceScore: 0.9 }),
    createItem({ id: '2', category: 'Product', source: 'slack', importanceScore: 0.6 }),
    createItem({ id: '3', category: 'Engineering', source: 'confluence', importanceScore: 0.3 }),
    createItem({ id: '4', category: 'Sales', source: 'slack', importanceScore: 0.85 }),
  ]

  describe('category filter', () => {
    it('returns all items when category is "all"', () => {
      const result = filterDigestItems(items, 'all', 'all', 'all')
      expect(result).toHaveLength(4)
    })

    it('filters by category (case insensitive)', () => {
      const result = filterDigestItems(items, 'engineering', 'all', 'all')
      expect(result).toHaveLength(2)
      expect(result.every(item => item.category.toLowerCase() === 'engineering')).toBe(true)
    })

    it('returns empty array for non-matching category', () => {
      const result = filterDigestItems(items, 'marketing', 'all', 'all')
      expect(result).toHaveLength(0)
    })
  })

  describe('importance filter', () => {
    it('returns all items when importance is "all"', () => {
      const result = filterDigestItems(items, 'all', 'all', 'all')
      expect(result).toHaveLength(4)
    })

    it('filters by high importance', () => {
      const result = filterDigestItems(items, 'all', 'high', 'all')
      expect(result).toHaveLength(2) // 0.9 and 0.85
      expect(result.every(item => item.importanceScore >= 0.8)).toBe(true)
    })

    it('filters by medium importance', () => {
      const result = filterDigestItems(items, 'all', 'medium', 'all')
      expect(result).toHaveLength(1) // 0.6
      expect(result[0].importanceScore).toBe(0.6)
    })

    it('filters by low importance', () => {
      const result = filterDigestItems(items, 'all', 'low', 'all')
      expect(result).toHaveLength(1) // 0.3
      expect(result[0].importanceScore).toBe(0.3)
    })
  })

  describe('source filter', () => {
    it('returns all items when source is "all"', () => {
      const result = filterDigestItems(items, 'all', 'all', 'all')
      expect(result).toHaveLength(4)
    })

    it('filters by slack source', () => {
      const result = filterDigestItems(items, 'all', 'all', 'slack')
      expect(result).toHaveLength(3)
      expect(result.every(item => item.source === 'slack')).toBe(true)
    })

    it('filters by confluence source', () => {
      const result = filterDigestItems(items, 'all', 'all', 'confluence')
      expect(result).toHaveLength(1)
      expect(result[0].source).toBe('confluence')
    })
  })

  describe('combined filters (AND logic)', () => {
    it('applies category and importance filters together', () => {
      const result = filterDigestItems(items, 'engineering', 'high', 'all')
      expect(result).toHaveLength(1) // Only item 1 matches both
      expect(result[0].id).toBe('1')
    })

    it('applies all three filters together', () => {
      const result = filterDigestItems(items, 'engineering', 'high', 'slack')
      expect(result).toHaveLength(1)
      expect(result[0].id).toBe('1')
    })

    it('returns empty when filters exclude all items', () => {
      const result = filterDigestItems(items, 'engineering', 'high', 'confluence')
      expect(result).toHaveLength(0)
    })
  })

  describe('empty input', () => {
    it('returns empty array for empty input', () => {
      const result = filterDigestItems([], 'all', 'all', 'all')
      expect(result).toHaveLength(0)
    })
  })
})

describe('countByImportance', () => {
  it('counts items by importance level', () => {
    const items = [
      createItem({ importanceScore: 0.9 }),
      createItem({ importanceScore: 0.85 }),
      createItem({ importanceScore: 0.6 }),
      createItem({ importanceScore: 0.3 }),
    ]

    const counts = countByImportance(items)
    expect(counts.all).toBe(4)
    expect(counts.high).toBe(2)
    expect(counts.medium).toBe(1)
    expect(counts.low).toBe(1)
  })

  it('handles empty array', () => {
    const counts = countByImportance([])
    expect(counts.all).toBe(0)
    expect(counts.high).toBe(0)
    expect(counts.medium).toBe(0)
    expect(counts.low).toBe(0)
  })

  it('handles all items at same level', () => {
    const items = [
      createItem({ importanceScore: 0.9 }),
      createItem({ importanceScore: 0.95 }),
      createItem({ importanceScore: 0.8 }),
    ]

    const counts = countByImportance(items)
    expect(counts.all).toBe(3)
    expect(counts.high).toBe(3)
    expect(counts.medium).toBe(0)
    expect(counts.low).toBe(0)
  })
})

describe('countBySource', () => {
  it('counts items by source', () => {
    const items = [
      createItem({ source: 'slack' }),
      createItem({ source: 'slack' }),
      createItem({ source: 'confluence' }),
      createItem({ source: 'ai' }),
    ]

    const counts = countBySource(items)
    expect(counts.all).toBe(4)
    expect(counts.slack).toBe(2)
    expect(counts.confluence).toBe(1)
    expect(counts.ai).toBe(1)
  })

  it('handles empty array', () => {
    const counts = countBySource([])
    expect(counts.all).toBe(0)
    expect(counts.slack).toBe(0)
    expect(counts.confluence).toBe(0)
    expect(counts.ai).toBe(0)
  })

  it('handles single source', () => {
    const items = [
      createItem({ source: 'slack' }),
      createItem({ source: 'slack' }),
    ]

    const counts = countBySource(items)
    expect(counts.all).toBe(2)
    expect(counts.slack).toBe(2)
    expect(counts.confluence).toBe(0)
    expect(counts.ai).toBe(0)
  })
})

describe('getAvailableSources', () => {
  it('returns only "all" when no items have sources', () => {
    const counts = { all: 0, slack: 0, confluence: 0, ai: 0 }
    expect(getAvailableSources(counts)).toEqual(['all'])
  })

  it('includes slack when slack has items', () => {
    const counts = { all: 5, slack: 5, confluence: 0, ai: 0 }
    expect(getAvailableSources(counts)).toEqual(['all', 'slack'])
  })

  it('includes confluence when confluence has items', () => {
    const counts = { all: 3, slack: 0, confluence: 3, ai: 0 }
    expect(getAvailableSources(counts)).toEqual(['all', 'confluence'])
  })

  it('includes ai when ai has items', () => {
    const counts = { all: 2, slack: 0, confluence: 0, ai: 2 }
    expect(getAvailableSources(counts)).toEqual(['all', 'ai'])
  })

  it('includes all sources when all have items', () => {
    const counts = { all: 6, slack: 3, confluence: 2, ai: 1 }
    expect(getAvailableSources(counts)).toEqual(['all', 'slack', 'confluence', 'ai'])
  })
})

describe('countActiveFilters', () => {
  it('returns 0 when all filters are "all"', () => {
    expect(countActiveFilters('all', 'all', 'all')).toBe(0)
  })

  it('counts category filter', () => {
    expect(countActiveFilters('engineering', 'all', 'all')).toBe(1)
  })

  it('counts importance filter', () => {
    expect(countActiveFilters('all', 'high', 'all')).toBe(1)
  })

  it('counts source filter', () => {
    expect(countActiveFilters('all', 'all', 'slack')).toBe(1)
  })

  it('counts multiple active filters', () => {
    expect(countActiveFilters('engineering', 'high', 'all')).toBe(2)
  })

  it('counts all active filters', () => {
    expect(countActiveFilters('engineering', 'high', 'slack')).toBe(3)
  })
})
