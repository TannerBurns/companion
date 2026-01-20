import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { ContentDetailModal } from './ContentDetailModal'
import type { DigestItem } from '../lib/api'

describe('ContentDetailModal', () => {
  const mockItem: DigestItem = {
    id: '1',
    title: 'Test Title',
    summary: 'This is a test summary for the digest item.',
    highlights: ['First highlight', 'Second highlight'],
    category: 'engineering',
    categoryConfidence: 0.9,
    source: 'slack',
    sourceUrl: 'https://slack.com/archives/C123/p456',
    importanceScore: 8,
    createdAt: 1705363200000, // Jan 15, 2024
  }

  const mockOnClose = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    document.body.style.overflow = ''
  })

  describe('when item is null', () => {
    it('renders nothing', () => {
      const { container } = render(<ContentDetailModal item={null} onClose={mockOnClose} />)
      expect(container.firstChild).toBeNull()
    })
  })

  describe('when item is provided', () => {
    it('renders the modal', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      expect(screen.getByText('Test Title')).toBeInTheDocument()
    })

    it('displays the title', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      expect(screen.getByRole('heading', { level: 2 })).toHaveTextContent('Test Title')
    })

    it('displays the summary', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      expect(screen.getByText('This is a test summary for the digest item.')).toBeInTheDocument()
    })

    it('displays all highlights', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      expect(screen.getByText('First highlight')).toBeInTheDocument()
      expect(screen.getByText('Second highlight')).toBeInTheDocument()
    })

    it('displays Key Highlights heading when highlights exist', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      expect(screen.getByText('Key Highlights')).toBeInTheDocument()
    })

    it('does not display Key Highlights when no highlights', () => {
      const itemWithoutHighlights = { ...mockItem, highlights: [] }
      render(<ContentDetailModal item={itemWithoutHighlights} onClose={mockOnClose} />)
      expect(screen.queryByText('Key Highlights')).not.toBeInTheDocument()
    })

    it('does not display Key Highlights when highlights is undefined', () => {
      const itemWithoutHighlights = { ...mockItem, highlights: undefined }
      render(<ContentDetailModal item={itemWithoutHighlights} onClose={mockOnClose} />)
      expect(screen.queryByText('Key Highlights')).not.toBeInTheDocument()
    })
  })

  describe('title fallback', () => {
    it('uses summary slice when title is empty', () => {
      const itemWithoutTitle = { ...mockItem, title: '' }
      render(<ContentDetailModal item={itemWithoutTitle} onClose={mockOnClose} />)
      expect(screen.getByRole('heading', { level: 2 })).toHaveTextContent('This is a test summary for the digest item.')
    })
  })

  describe('close button', () => {
    it('renders close button with aria-label', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument()
    })

    it('calls onClose when close button is clicked', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      fireEvent.click(screen.getByRole('button', { name: 'Close' }))
      expect(mockOnClose).toHaveBeenCalledTimes(1)
    })
  })

  describe('backdrop click', () => {
    it('calls onClose when backdrop is clicked', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      const backdrop = document.querySelector('.fixed.inset-0')
      fireEvent.click(backdrop!)
      expect(mockOnClose).toHaveBeenCalledTimes(1)
    })

    it('does not call onClose when modal content is clicked', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      fireEvent.click(screen.getByText('Test Title'))
      expect(mockOnClose).not.toHaveBeenCalled()
    })
  })

  describe('escape key', () => {
    it('calls onClose when Escape key is pressed', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      fireEvent.keyDown(document, { key: 'Escape' })
      expect(mockOnClose).toHaveBeenCalledTimes(1)
    })

    it('does not call onClose for other keys', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      fireEvent.keyDown(document, { key: 'Enter' })
      expect(mockOnClose).not.toHaveBeenCalled()
    })
  })

  describe('body scroll lock', () => {
    it('sets body overflow to hidden when modal opens', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      expect(document.body.style.overflow).toBe('hidden')
    })

    it('restores body overflow when modal closes', () => {
      const { unmount } = render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      unmount()
      expect(document.body.style.overflow).toBe('')
    })
  })

  describe('footer with source URL', () => {
    it('shows View in Slack button when sourceUrl exists and source is slack', () => {
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      expect(screen.getByRole('button', { name: /View in Slack/i })).toBeInTheDocument()
    })

    it('shows View in Confluence button when source is confluence', () => {
      const confluenceItem = { ...mockItem, source: 'confluence' as const }
      render(<ContentDetailModal item={confluenceItem} onClose={mockOnClose} />)
      expect(screen.getByRole('button', { name: /View in Confluence/i })).toBeInTheDocument()
    })

    it('shows View in Source button for other sources', () => {
      const jiraItem = { ...mockItem, source: 'jira' as const }
      render(<ContentDetailModal item={jiraItem} onClose={mockOnClose} />)
      expect(screen.getByRole('button', { name: /View in Source/i })).toBeInTheDocument()
    })

    it('does not show footer when sourceUrl is undefined', () => {
      const itemWithoutUrl = { ...mockItem, sourceUrl: undefined }
      render(<ContentDetailModal item={itemWithoutUrl} onClose={mockOnClose} />)
      expect(screen.queryByRole('button', { name: /View in/i })).not.toBeInTheDocument()
    })

    it('opens sourceUrl in new tab when View button is clicked', () => {
      const windowOpen = vi.spyOn(window, 'open').mockImplementation(() => null)
      render(<ContentDetailModal item={mockItem} onClose={mockOnClose} />)
      fireEvent.click(screen.getByRole('button', { name: /View in Slack/i }))
      expect(windowOpen).toHaveBeenCalledWith(
        'https://slack.com/archives/C123/p456',
        '_blank',
        'noopener,noreferrer'
      )
      windowOpen.mockRestore()
    })
  })
})
