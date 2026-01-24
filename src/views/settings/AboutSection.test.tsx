import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { AboutSection } from './AboutSection'

// Mock Tauri app API
vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('1.0.0'),
  getName: vi.fn().mockResolvedValue('Companion'),
}))

// Mock useUpdater hook
const mockCheckForUpdates = vi.fn()
const mockDownloadAndInstall = vi.fn()
const mockHandleRestart = vi.fn()
const mockUseUpdater = vi.fn(() => ({
  state: { status: 'idle' },
  checkForUpdates: mockCheckForUpdates,
  downloadAndInstall: mockDownloadAndInstall,
  handleRestart: mockHandleRestart,
}))

vi.mock('../../hooks/useUpdater', () => ({
  useUpdater: () => mockUseUpdater(),
}))

describe('AboutSection', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockUseUpdater.mockReturnValue({
      state: { status: 'idle' },
      checkForUpdates: mockCheckForUpdates,
      downloadAndInstall: mockDownloadAndInstall,
      handleRestart: mockHandleRestart,
    })
  })

  it('renders the section title', () => {
    render(<AboutSection />)
    expect(screen.getByText('About')).toBeInTheDocument()
  })

  it('renders the description', () => {
    render(<AboutSection />)
    expect(screen.getByText('Application information and updates.')).toBeInTheDocument()
  })

  it('displays app name and version', async () => {
    render(<AboutSection />)
    await waitFor(() => {
      expect(screen.getByText('Companion')).toBeInTheDocument()
      expect(screen.getByText('Version 1.0.0')).toBeInTheDocument()
    })
  })

  it('shows check for updates button in idle state', () => {
    render(<AboutSection />)
    expect(screen.getByRole('button', { name: /check for updates/i })).toBeInTheDocument()
  })

  it('calls checkForUpdates when button is clicked', () => {
    render(<AboutSection />)
    fireEvent.click(screen.getByRole('button', { name: /check for updates/i }))
    expect(mockCheckForUpdates).toHaveBeenCalled()
  })

  it('shows checking state', () => {
    mockUseUpdater.mockReturnValue({
      state: { status: 'checking' },
      checkForUpdates: mockCheckForUpdates,
      downloadAndInstall: mockDownloadAndInstall,
      handleRestart: mockHandleRestart,
    })
    render(<AboutSection />)
    expect(screen.getByText('Checking for updates...')).toBeInTheDocument()
  })

  it('shows no update available message', () => {
    mockUseUpdater.mockReturnValue({
      state: { status: 'no-update' },
      checkForUpdates: mockCheckForUpdates,
      downloadAndInstall: mockDownloadAndInstall,
      handleRestart: mockHandleRestart,
    })
    render(<AboutSection />)
    expect(screen.getByText('You are running the latest version')).toBeInTheDocument()
  })

  it('shows update available with download button', () => {
    mockUseUpdater.mockReturnValue({
      state: { status: 'available', update: { version: '2.0.0' } } as never,
      checkForUpdates: mockCheckForUpdates,
      downloadAndInstall: mockDownloadAndInstall,
      handleRestart: mockHandleRestart,
    })
    render(<AboutSection />)
    expect(screen.getByText('Version 2.0.0 is available')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /download & install/i })).toBeInTheDocument()
  })

  it('calls downloadAndInstall when download button is clicked', () => {
    mockUseUpdater.mockReturnValue({
      state: { status: 'available', update: { version: '2.0.0' } } as never,
      checkForUpdates: mockCheckForUpdates,
      downloadAndInstall: mockDownloadAndInstall,
      handleRestart: mockHandleRestart,
    })
    render(<AboutSection />)
    fireEvent.click(screen.getByRole('button', { name: /download & install/i }))
    expect(mockDownloadAndInstall).toHaveBeenCalled()
  })

  it('shows download progress', () => {
    mockUseUpdater.mockReturnValue({
      state: { status: 'downloading', progress: 50 } as never,
      checkForUpdates: mockCheckForUpdates,
      downloadAndInstall: mockDownloadAndInstall,
      handleRestart: mockHandleRestart,
    })
    render(<AboutSection />)
    expect(screen.getByText('Downloading update... 50%')).toBeInTheDocument()
  })

  it('shows restart button when update is ready', () => {
    mockUseUpdater.mockReturnValue({
      state: { status: 'ready' } as never,
      checkForUpdates: mockCheckForUpdates,
      downloadAndInstall: mockDownloadAndInstall,
      handleRestart: mockHandleRestart,
    })
    render(<AboutSection />)
    expect(screen.getByText('Update ready to install')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /restart now/i })).toBeInTheDocument()
  })

  it('calls handleRestart when restart button is clicked', () => {
    mockUseUpdater.mockReturnValue({
      state: { status: 'ready' } as never,
      checkForUpdates: mockCheckForUpdates,
      downloadAndInstall: mockDownloadAndInstall,
      handleRestart: mockHandleRestart,
    })
    render(<AboutSection />)
    fireEvent.click(screen.getByRole('button', { name: /restart now/i }))
    expect(mockHandleRestart).toHaveBeenCalled()
  })

  it('shows error message', () => {
    mockUseUpdater.mockReturnValue({
      state: { status: 'error', message: 'Network error' } as never,
      checkForUpdates: mockCheckForUpdates,
      downloadAndInstall: mockDownloadAndInstall,
      handleRestart: mockHandleRestart,
    })
    render(<AboutSection />)
    expect(screen.getByText('Network error')).toBeInTheDocument()
  })
})
