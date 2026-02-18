import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { DataSection } from './DataSection'

// Mock the API
const mockGetDataStats = vi.fn().mockResolvedValue({
  contentItems: 150,
  aiSummaries: 45,
  slackUsers: 12,
  syncStates: 3,
})

vi.mock('../../lib/api', () => ({
  api: {
    getDataStats: () => mockGetDataStats(),
    clearSyncedData: vi.fn().mockResolvedValue({ itemsDeleted: 195 }),
    factoryReset: vi.fn().mockResolvedValue(undefined),
  },
}))

// Mock the store
const mockResetSlackState = vi.fn()
vi.mock('../../store', () => ({
  useAppStore: () => ({
    resetSlackState: mockResetSlackState,
  }),
}))

function renderWithQueryClient(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>
  )
}

async function renderDataSection() {
  renderWithQueryClient(<DataSection />)
  await waitFor(() => {
    expect(mockGetDataStats).toHaveBeenCalled()
  })
}

describe('DataSection', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetDataStats.mockResolvedValue({
      contentItems: 150,
      aiSummaries: 45,
      slackUsers: 12,
      syncStates: 3,
    })
  })

  it('renders the section title', async () => {
    await renderDataSection()
    expect(screen.getByText('Data Management')).toBeInTheDocument()
  })

  it('displays data statistics', async () => {
    await renderDataSection()
    await waitFor(() => {
      expect(screen.getByText('150')).toBeInTheDocument()
      expect(screen.getByText('45')).toBeInTheDocument()
      expect(screen.getByText('12')).toBeInTheDocument()
      expect(screen.getByText('3')).toBeInTheDocument()
    })
  })

  it('shows storage heading', async () => {
    await renderDataSection()
    expect(screen.getByText('Storage')).toBeInTheDocument()
  })

  it('shows clear synced data heading', async () => {
    await renderDataSection()
    expect(screen.getByText('Clear Synced Data')).toBeInTheDocument()
  })

  it('shows factory reset heading', async () => {
    await renderDataSection()
    expect(screen.getByText('Factory Reset')).toBeInTheDocument()
  })

  it('shows confirmation dialog when reset button is clicked', async () => {
    await renderDataSection()
    // Find the reset button by looking for button with Reset text within the factory reset section
    const resetButtons = screen.getAllByRole('button')
    const resetButton = resetButtons.find(btn => btn.textContent?.includes('Reset') && !btn.textContent?.includes('Everything'))
    expect(resetButton).toBeDefined()
    fireEvent.click(resetButton!)
    await waitFor(() => {
      expect(screen.getByText('Confirm Factory Reset')).toBeInTheDocument()
    })
  })

  it('shows cancel button in reset dialog', async () => {
    await renderDataSection()
    const resetButtons = screen.getAllByRole('button')
    const resetButton = resetButtons.find(btn => btn.textContent?.includes('Reset') && !btn.textContent?.includes('Everything'))
    fireEvent.click(resetButton!)
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument()
    })
  })
})
