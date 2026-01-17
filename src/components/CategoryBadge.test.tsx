import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { CategoryBadge } from './CategoryBadge'

describe('CategoryBadge', () => {
  describe('category display', () => {
    it('displays the category text', () => {
      render(<CategoryBadge category="Sales" />)
      expect(screen.getByText('Sales')).toBeInTheDocument()
    })

    it('preserves category case in display', () => {
      render(<CategoryBadge category="ENGINEERING" />)
      expect(screen.getByText('ENGINEERING')).toBeInTheDocument()
    })
  })

  describe('category colors', () => {
    const categories = ['sales', 'marketing', 'product', 'engineering', 'research']

    categories.forEach(category => {
      it(`applies correct color class for ${category}`, () => {
        const { container } = render(<CategoryBadge category={category} />)
        const badge = container.firstChild as HTMLElement
        expect(badge.className).toContain('bg-')
        expect(badge.className).toContain('text-')
      })
    })

    it('applies fallback color for unknown category', () => {
      const { container } = render(<CategoryBadge category="unknown-category" />)
      const badge = container.firstChild as HTMLElement
      expect(badge.className).toContain('bg-gray-100')
    })

    it('handles case-insensitive category matching', () => {
      const { container: lower } = render(<CategoryBadge category="sales" />)
      const { container: upper } = render(<CategoryBadge category="SALES" />)
      const { container: mixed } = render(<CategoryBadge category="Sales" />)

      const lowerClasses = (lower.firstChild as HTMLElement).className
      const upperClasses = (upper.firstChild as HTMLElement).className
      const mixedClasses = (mixed.firstChild as HTMLElement).className

      expect(lowerClasses).toContain('bg-green-100')
      expect(upperClasses).toContain('bg-green-100')
      expect(mixedClasses).toContain('bg-green-100')
    })
  })

  describe('confidence indicator', () => {
    it('shows ? for low confidence (< 0.7)', () => {
      render(<CategoryBadge category="Sales" confidence={0.5} />)
      expect(screen.getByText('?')).toBeInTheDocument()
    })

    it('shows ? for confidence exactly at threshold boundary', () => {
      render(<CategoryBadge category="Sales" confidence={0.69} />)
      expect(screen.getByText('?')).toBeInTheDocument()
    })

    it('does not show ? for high confidence (>= 0.7)', () => {
      render(<CategoryBadge category="Sales" confidence={0.7} />)
      expect(screen.queryByText('?')).not.toBeInTheDocument()
    })

    it('does not show ? for confidence = 1.0', () => {
      render(<CategoryBadge category="Sales" confidence={1.0} />)
      expect(screen.queryByText('?')).not.toBeInTheDocument()
    })

    it('does not show ? when confidence is undefined', () => {
      render(<CategoryBadge category="Sales" />)
      expect(screen.queryByText('?')).not.toBeInTheDocument()
    })
  })

  describe('custom className', () => {
    it('applies custom className', () => {
      const { container } = render(<CategoryBadge category="Sales" className="custom-class" />)
      expect(container.firstChild).toHaveClass('custom-class')
    })
  })
})
