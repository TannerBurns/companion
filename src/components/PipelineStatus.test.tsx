import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'

// Mock the usePipeline hook
vi.mock('../hooks/usePipeline', () => ({
  usePipeline: vi.fn(),
  getTaskDisplayName: (type: string) => `Display: ${type}`,
}))

import { PipelineStatus } from './PipelineStatus'
import { usePipeline } from '../hooks/usePipeline'

const mockUsePipeline = usePipeline as ReturnType<typeof vi.fn>

describe('PipelineStatus', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders idle state when not busy', () => {
    mockUsePipeline.mockReturnValue({
      activeTasks: [],
      recentHistory: [],
      isBusy: false,
      taskCount: 0,
    })

    render(<PipelineStatus />)
    expect(screen.getByTitle('Pipeline status')).toBeInTheDocument()
  })

  it('renders busy state with task count', () => {
    mockUsePipeline.mockReturnValue({
      activeTasks: [
        {
          id: '1',
          task_type: 'sync_slack',
          status: 'running',
          message: 'Syncing',
          progress: 0.5,
          started_at: Date.now() / 1000,
          completed_at: null,
          error: null,
        },
      ],
      recentHistory: [],
      isBusy: true,
      taskCount: 1,
    })

    render(<PipelineStatus />)
    expect(screen.getByTitle('1 task(s) running')).toBeInTheDocument()
    expect(screen.getByText('1')).toBeInTheDocument()
  })

  it('shows dropdown when clicked', () => {
    mockUsePipeline.mockReturnValue({
      activeTasks: [],
      recentHistory: [],
      isBusy: false,
      taskCount: 0,
    })

    render(<PipelineStatus />)
    const button = screen.getByRole('button')
    fireEvent.click(button)

    expect(screen.getByText('No recent activity')).toBeInTheDocument()
  })

  it('shows running tasks in dropdown', () => {
    mockUsePipeline.mockReturnValue({
      activeTasks: [
        {
          id: '1',
          task_type: 'sync_slack',
          status: 'running',
          message: 'Syncing messages',
          progress: 0.5,
          started_at: Date.now() / 1000,
          completed_at: null,
          error: null,
        },
      ],
      recentHistory: [],
      isBusy: true,
      taskCount: 1,
    })

    render(<PipelineStatus />)
    fireEvent.click(screen.getByRole('button'))

    expect(screen.getByText('Running')).toBeInTheDocument()
    expect(screen.getByText('Syncing messages')).toBeInTheDocument()
    expect(screen.getByText('50%')).toBeInTheDocument()
  })

  it('shows recent history in dropdown', () => {
    mockUsePipeline.mockReturnValue({
      activeTasks: [],
      recentHistory: [
        {
          id: '1',
          task_type: 'sync_jira',
          status: 'completed',
          message: 'Sync complete',
          progress: 1,
          started_at: Date.now() / 1000,
          completed_at: Date.now() / 1000,
          error: null,
        },
      ],
      isBusy: false,
      taskCount: 0,
    })

    render(<PipelineStatus />)
    fireEvent.click(screen.getByRole('button'))

    expect(screen.getByText('Recent')).toBeInTheDocument()
    expect(screen.getByText('Sync complete')).toBeInTheDocument()
  })

  it('shows failed task with error icon', () => {
    mockUsePipeline.mockReturnValue({
      activeTasks: [],
      recentHistory: [
        {
          id: '1',
          task_type: 'sync_slack',
          status: 'failed',
          message: 'Sync failed',
          progress: null,
          started_at: Date.now() / 1000,
          completed_at: Date.now() / 1000,
          error: 'Connection timeout',
        },
      ],
      isBusy: false,
      taskCount: 0,
    })

    render(<PipelineStatus />)
    fireEvent.click(screen.getByRole('button'))

    expect(screen.getByText('Sync failed')).toBeInTheDocument()
  })

  it('closes dropdown when clicking outside', () => {
    mockUsePipeline.mockReturnValue({
      activeTasks: [],
      recentHistory: [],
      isBusy: false,
      taskCount: 0,
    })

    render(
      <div>
        <div data-testid="outside">Outside</div>
        <PipelineStatus />
      </div>
    )

    fireEvent.click(screen.getByRole('button'))
    expect(screen.getByText('No recent activity')).toBeInTheDocument()

    fireEvent.mouseDown(screen.getByTestId('outside'))
    expect(screen.queryByText('No recent activity')).not.toBeInTheDocument()
  })
})
