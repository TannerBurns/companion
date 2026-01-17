import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { Card } from './Card'

describe('Card', () => {
  describe('rendering', () => {
    it('renders children', () => {
      render(<Card>Card content</Card>)
      expect(screen.getByText('Card content')).toBeInTheDocument()
    })

    it('renders as a div', () => {
      const { container } = render(<Card>Content</Card>)
      expect(container.firstChild?.nodeName).toBe('DIV')
    })

    it('applies base styles', () => {
      const { container } = render(<Card>Content</Card>)
      expect(container.firstChild).toHaveClass('rounded-lg', 'border', 'bg-card', 'p-4', 'shadow-sm')
    })
  })

  describe('hoverable prop', () => {
    it('does not apply hover styles by default', () => {
      const { container } = render(<Card>Content</Card>)
      expect(container.firstChild).not.toHaveClass('hover:shadow-md')
    })

    it('applies hover styles when hoverable is true', () => {
      const { container } = render(<Card hoverable>Content</Card>)
      expect(container.firstChild).toHaveClass('cursor-pointer', 'hover:shadow-md')
    })
  })

  describe('onClick prop', () => {
    it('applies cursor-pointer when onClick is provided', () => {
      const { container } = render(<Card onClick={() => {}}>Content</Card>)
      expect(container.firstChild).toHaveClass('cursor-pointer')
    })

    it('calls onClick when clicked', () => {
      const handleClick = vi.fn()
      render(<Card onClick={handleClick}>Clickable</Card>)
      fireEvent.click(screen.getByText('Clickable'))
      expect(handleClick).toHaveBeenCalledTimes(1)
    })
  })

  describe('custom className', () => {
    it('applies custom className', () => {
      const { container } = render(<Card className="custom-class">Content</Card>)
      expect(container.firstChild).toHaveClass('custom-class')
    })

    it('merges custom className with default classes', () => {
      const { container } = render(<Card className="my-4">Content</Card>)
      expect(container.firstChild).toHaveClass('my-4')
      expect(container.firstChild).toHaveClass('rounded-lg')
    })
  })
})
