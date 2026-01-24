import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { open as shellOpen } from '@tauri-apps/plugin-shell'
import { openUrl } from './openUrl'

vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn(),
}))

describe('openUrl', () => {
  const originalWindowOpen = window.open

  beforeEach(() => {
    vi.clearAllMocks()
    window.open = vi.fn()
  })

  afterEach(() => {
    window.open = originalWindowOpen
  })

  it('calls shell.open with the provided URL', async () => {
    vi.mocked(shellOpen).mockResolvedValue()
    
    await openUrl('https://example.com')
    
    expect(shellOpen).toHaveBeenCalledWith('https://example.com')
    expect(shellOpen).toHaveBeenCalledTimes(1)
  })

  it('does not call window.open when shell.open succeeds', async () => {
    vi.mocked(shellOpen).mockResolvedValue()
    
    await openUrl('https://example.com')
    
    expect(window.open).not.toHaveBeenCalled()
  })

  it('falls back to window.open when shell.open throws', async () => {
    vi.mocked(shellOpen).mockRejectedValue(new Error('Shell not available'))
    
    await openUrl('https://example.com')
    
    expect(window.open).toHaveBeenCalledWith(
      'https://example.com',
      '_blank',
      'noopener,noreferrer'
    )
  })

  it('handles Slack URLs', async () => {
    vi.mocked(shellOpen).mockResolvedValue()
    
    await openUrl('https://myworkspace.slack.com/archives/C123/p456')
    
    expect(shellOpen).toHaveBeenCalledWith('https://myworkspace.slack.com/archives/C123/p456')
  })

  it('handles app_redirect URLs', async () => {
    vi.mocked(shellOpen).mockResolvedValue()
    
    await openUrl('slack://app_redirect?channel=C123&message_ts=123.456')
    
    expect(shellOpen).toHaveBeenCalledWith('slack://app_redirect?channel=C123&message_ts=123.456')
  })
})
