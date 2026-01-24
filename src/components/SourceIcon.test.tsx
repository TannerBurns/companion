import { render } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { SourceIcon } from './SourceIcon'

describe('SourceIcon', () => {
  describe('source types', () => {
    it('renders MessageSquare icon for slack', () => {
      const { container } = render(<SourceIcon source="slack" />)
      const svg = container.querySelector('svg')
      expect(svg).toBeInTheDocument()
      expect(svg).toHaveClass('h-4', 'w-4')
    })

    it('renders FileText icon for confluence', () => {
      const { container } = render(<SourceIcon source="confluence" />)
      const svg = container.querySelector('svg')
      expect(svg).toBeInTheDocument()
    })

    it('renders Sparkles icon for ai', () => {
      const { container } = render(<SourceIcon source="ai" />)
      const svg = container.querySelector('svg')
      expect(svg).toBeInTheDocument()
    })
  })

  describe('className', () => {
    it('uses default className when not provided', () => {
      const { container } = render(<SourceIcon source="slack" />)
      const svg = container.querySelector('svg')
      expect(svg).toHaveClass('h-4', 'w-4')
    })

    it('applies custom className', () => {
      const { container } = render(<SourceIcon source="slack" className="h-6 w-6" />)
      const svg = container.querySelector('svg')
      expect(svg).toHaveClass('h-6', 'w-6')
    })

    it('always applies flex-shrink-0', () => {
      const { container } = render(<SourceIcon source="slack" className="h-8 w-8" />)
      const svg = container.querySelector('svg')
      expect(svg).toHaveClass('flex-shrink-0')
    })
  })
})
