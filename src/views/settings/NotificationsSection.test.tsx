import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { NotificationsSection } from './NotificationsSection'

describe('NotificationsSection', () => {
  it('renders the section title', () => {
    render(<NotificationsSection />)
    expect(screen.getByText('Notifications')).toBeInTheDocument()
  })

  it('renders the description', () => {
    render(<NotificationsSection />)
    expect(screen.getByText('Configure when and how you receive notifications.')).toBeInTheDocument()
  })

  it('renders the daily digest notification option', () => {
    render(<NotificationsSection />)
    expect(screen.getByText('Daily Digest Notification')).toBeInTheDocument()
    expect(screen.getByText('Get notified when your daily digest is ready')).toBeInTheDocument()
  })

  it('shows coming soon badge', () => {
    render(<NotificationsSection />)
    expect(screen.getByText('Coming Soon')).toBeInTheDocument()
  })

  it('renders a disabled toggle', () => {
    render(<NotificationsSection />)
    const toggle = screen.getByRole('button')
    expect(toggle).toBeDisabled()
  })
})
