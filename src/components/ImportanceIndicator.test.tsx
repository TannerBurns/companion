import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { ImportanceIndicator } from './ImportanceIndicator'

describe('ImportanceIndicator', () => {
  describe('score thresholds', () => {
    it('renders high level for score >= 0.8', () => {
      render(<ImportanceIndicator score={0.8} showLabel />)
      expect(screen.getByText('High')).toBeInTheDocument()
    })

    it('renders high level for score = 1.0', () => {
      render(<ImportanceIndicator score={1.0} showLabel />)
      expect(screen.getByText('High')).toBeInTheDocument()
    })

    it('renders medium level for score >= 0.5 and < 0.8', () => {
      render(<ImportanceIndicator score={0.5} showLabel />)
      expect(screen.getByText('Medium')).toBeInTheDocument()
    })

    it('renders medium level for score = 0.79', () => {
      render(<ImportanceIndicator score={0.79} showLabel />)
      expect(screen.getByText('Medium')).toBeInTheDocument()
    })

    it('renders low level for score < 0.5', () => {
      render(<ImportanceIndicator score={0.49} showLabel />)
      expect(screen.getByText('Low')).toBeInTheDocument()
    })

    it('renders low level for score = 0', () => {
      render(<ImportanceIndicator score={0} showLabel />)
      expect(screen.getByText('Low')).toBeInTheDocument()
    })
  })

  describe('label visibility', () => {
    it('does not show label by default', () => {
      render(<ImportanceIndicator score={0.9} />)
      expect(screen.queryByText('High')).not.toBeInTheDocument()
    })

    it('shows label when showLabel is true', () => {
      render(<ImportanceIndicator score={0.9} showLabel />)
      expect(screen.getByText('High')).toBeInTheDocument()
    })

    it('hides label when showLabel is false', () => {
      render(<ImportanceIndicator score={0.9} showLabel={false} />)
      expect(screen.queryByText('High')).not.toBeInTheDocument()
    })
  })

  describe('indicator dots', () => {
    it('renders three indicator dots', () => {
      const { container } = render(<ImportanceIndicator score={0.5} />)
      const dots = container.querySelectorAll('.rounded-full')
      expect(dots).toHaveLength(3)
    })
  })

  describe('custom className', () => {
    it('applies custom className', () => {
      const { container } = render(<ImportanceIndicator score={0.5} className="custom-class" />)
      expect(container.firstChild).toHaveClass('custom-class')
    })
  })
})
