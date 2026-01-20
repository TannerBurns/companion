import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { Slack } from 'lucide-react'
import { SourceCard } from './SourceCard'

describe('SourceCard', () => {
  const defaultProps = {
    icon: Slack,
    name: 'Slack',
    description: 'Connect your Slack workspace',
    connected: false,
    onConnect: vi.fn(),
    onDisconnect: vi.fn(),
  }

  describe('basic rendering', () => {
    it('renders the source name', () => {
      render(<SourceCard {...defaultProps} />)
      expect(screen.getByText('Slack')).toBeInTheDocument()
    })

    it('renders the description', () => {
      render(<SourceCard {...defaultProps} />)
      expect(screen.getByText('Connect your Slack workspace')).toBeInTheDocument()
    })

    it('renders the icon', () => {
      const { container } = render(<SourceCard {...defaultProps} />)
      const svg = container.querySelector('svg')
      expect(svg).toBeInTheDocument()
    })
  })

  describe('connected state', () => {
    it('shows Connected badge when connected', () => {
      render(<SourceCard {...defaultProps} connected={true} />)
      expect(screen.getByText('Connected')).toBeInTheDocument()
    })

    it('does not show Connected badge when not connected', () => {
      render(<SourceCard {...defaultProps} connected={false} />)
      expect(screen.queryByText('Connected')).not.toBeInTheDocument()
    })

    it('shows Disconnect button when connected', () => {
      render(<SourceCard {...defaultProps} connected={true} />)
      expect(screen.getByRole('button', { name: 'Disconnect' })).toBeInTheDocument()
    })

    it('shows Connect button when not connected', () => {
      render(<SourceCard {...defaultProps} connected={false} />)
      expect(screen.getByRole('button', { name: 'Connect' })).toBeInTheDocument()
    })
  })

  describe('connecting state', () => {
    it('shows Connecting... when isConnecting is true', () => {
      render(<SourceCard {...defaultProps} isConnecting={true} />)
      expect(screen.getByText('Connecting...')).toBeInTheDocument()
    })

    it('disables button when isConnecting', () => {
      render(<SourceCard {...defaultProps} isConnecting={true} />)
      expect(screen.getByRole('button')).toBeDisabled()
    })
  })

  describe('comingSoon state', () => {
    it('shows Coming Soon badge when comingSoon is true', () => {
      render(<SourceCard {...defaultProps} comingSoon={true} />)
      expect(screen.getByText('Coming Soon')).toBeInTheDocument()
    })

    it('does not show Coming Soon badge when comingSoon is false', () => {
      render(<SourceCard {...defaultProps} comingSoon={false} />)
      expect(screen.queryByText('Coming Soon')).not.toBeInTheDocument()
    })

    it('hides connect/disconnect button when comingSoon is true', () => {
      render(<SourceCard {...defaultProps} comingSoon={true} />)
      expect(screen.queryByRole('button', { name: 'Connect' })).not.toBeInTheDocument()
      expect(screen.queryByRole('button', { name: 'Disconnect' })).not.toBeInTheDocument()
    })

    it('applies opacity class when comingSoon is true', () => {
      const { container } = render(<SourceCard {...defaultProps} comingSoon={true} />)
      expect(container.firstChild).toHaveClass('opacity-75')
    })

    it('does not apply opacity class when comingSoon is false', () => {
      const { container } = render(<SourceCard {...defaultProps} comingSoon={false} />)
      expect(container.firstChild).not.toHaveClass('opacity-75')
    })
  })

  describe('click handlers', () => {
    it('calls onConnect when Connect button is clicked', () => {
      const onConnect = vi.fn()
      render(<SourceCard {...defaultProps} onConnect={onConnect} />)
      fireEvent.click(screen.getByRole('button', { name: 'Connect' }))
      expect(onConnect).toHaveBeenCalledTimes(1)
    })

    it('calls onDisconnect when Disconnect button is clicked', () => {
      const onDisconnect = vi.fn()
      render(<SourceCard {...defaultProps} connected={true} onDisconnect={onDisconnect} />)
      fireEvent.click(screen.getByRole('button', { name: 'Disconnect' }))
      expect(onDisconnect).toHaveBeenCalledTimes(1)
    })
  })

  describe('children prop', () => {
    it('renders children when provided', () => {
      render(
        <SourceCard {...defaultProps}>
          <button>Configure Channels</button>
        </SourceCard>
      )
      expect(screen.getByRole('button', { name: 'Configure Channels' })).toBeInTheDocument()
    })

    it('renders children in a bordered container', () => {
      const { container } = render(
        <SourceCard {...defaultProps}>
          <span data-testid="child">Child content</span>
        </SourceCard>
      )
      const childContainer = container.querySelector('.border-t')
      expect(childContainer).toBeInTheDocument()
      expect(screen.getByTestId('child')).toBeInTheDocument()
    })

    it('does not render children container when no children provided', () => {
      const { container } = render(<SourceCard {...defaultProps} />)
      const childContainer = container.querySelector('.border-t.border-border')
      expect(childContainer).not.toBeInTheDocument()
    })
  })
})
