import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { SourcesSection } from './SourcesSection'

// Mock the API
const mockGetSlackConnectionStatus = vi.fn()

vi.mock('../../lib/api', () => ({
  api: {
    getSlackConnectionStatus: () => mockGetSlackConnectionStatus(),
    connectSlack: vi.fn(),
    disconnectSlack: vi.fn(),
  },
}))

// Mock the store
const mockSetSlackState = vi.fn()
vi.mock('../../store', () => ({
  useAppStore: () => ({
    slack: {
      connected: false,
      teamId: null,
      teamName: null,
      userId: null,
      selectedChannelCount: 0,
    },
    setSlackState: mockSetSlackState,
    showChannelSelector: false,
    setShowChannelSelector: vi.fn(),
  }),
}))

// Mock the components - SourceCard uses 'name' prop
vi.mock('../../components', () => ({
  SourceCard: ({ name, children }: { name: string; children?: React.ReactNode }) => (
    <div data-testid={`source-card-${name?.toLowerCase()}`}>
      <span>{name}</span>
      {children}
    </div>
  ),
  SlackChannelSelector: () => null,
}))

describe('SourcesSection', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetSlackConnectionStatus.mockResolvedValue({
      connected: false,
      teamId: null,
      teamName: null,
      userId: null,
      selectedChannelCount: 0,
    })
  })

  it('renders the section heading', () => {
    render(<SourcesSection />)
    expect(screen.getByText('Connected Sources')).toBeInTheDocument()
  })

  it('renders Slack source card', () => {
    render(<SourcesSection />)
    expect(screen.getByTestId('source-card-slack')).toBeInTheDocument()
  })

  it('renders Confluence source card', () => {
    render(<SourcesSection />)
    expect(screen.getByTestId('source-card-confluence')).toBeInTheDocument()
  })

  it('loads Slack connection status on mount', async () => {
    render(<SourcesSection />)
    await waitFor(() => {
      expect(mockGetSlackConnectionStatus).toHaveBeenCalled()
    })
  })

  it('updates state when Slack status is loaded', async () => {
    mockGetSlackConnectionStatus.mockResolvedValue({
      connected: true,
      teamId: 'T123',
      teamName: 'Test Team',
      userId: 'U123',
      selectedChannelCount: 5,
    })
    render(<SourcesSection />)
    await waitFor(() => {
      expect(mockSetSlackState).toHaveBeenCalledWith({
        connected: true,
        teamId: 'T123',
        teamName: 'Test Team',
        userId: 'U123',
        selectedChannelCount: 5,
      })
    })
  })
})
