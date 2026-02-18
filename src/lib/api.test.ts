import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { api } from './api'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = invoke as ReturnType<typeof vi.fn>

describe('api.generateWeeklyBreakdown', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('invokes generate_weekly_breakdown with weekStart and timezoneOffset', async () => {
    mockInvoke.mockResolvedValue({ breakdownText: 'test' })

    await api.generateWeeklyBreakdown('2026-02-16', -300)

    expect(mockInvoke).toHaveBeenCalledWith('generate_weekly_breakdown', {
      weekStart: '2026-02-16',
      timezoneOffset: -300,
    })
  })

  it('passes undefined values when params are omitted', async () => {
    mockInvoke.mockResolvedValue({ breakdownText: 'test' })

    await api.generateWeeklyBreakdown()

    expect(mockInvoke).toHaveBeenCalledWith('generate_weekly_breakdown', {
      weekStart: undefined,
      timezoneOffset: undefined,
    })
  })
})
