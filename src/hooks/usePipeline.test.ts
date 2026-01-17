import { describe, it, expect } from 'vitest'
import { getTaskDisplayName, getTaskIcon, type PipelineTaskType } from './usePipeline'

describe('getTaskDisplayName', () => {
  it('returns correct display name for sync_slack', () => {
    expect(getTaskDisplayName('sync_slack')).toBe('Syncing Slack')
  })

  it('returns correct display name for sync_jira', () => {
    expect(getTaskDisplayName('sync_jira')).toBe('Syncing Jira')
  })

  it('returns correct display name for sync_confluence', () => {
    expect(getTaskDisplayName('sync_confluence')).toBe('Syncing Confluence')
  })

  it('returns correct display name for ai_summarize', () => {
    expect(getTaskDisplayName('ai_summarize')).toBe('Summarizing content')
  })

  it('returns correct display name for ai_categorize', () => {
    expect(getTaskDisplayName('ai_categorize')).toBe('Categorizing items')
  })

  it('returns correct display name for generate_daily_digest', () => {
    expect(getTaskDisplayName('generate_daily_digest')).toBe('Generating daily digest')
  })

  it('returns correct display name for generate_weekly_digest', () => {
    expect(getTaskDisplayName('generate_weekly_digest')).toBe('Generating weekly digest')
  })

  it('returns task type as fallback for unknown type', () => {
    const unknownType = 'unknown_task' as PipelineTaskType
    expect(getTaskDisplayName(unknownType)).toBe('unknown_task')
  })
})

describe('getTaskIcon', () => {
  it('returns sync icon for sync tasks', () => {
    expect(getTaskIcon('sync_slack')).toBe('🔄')
    expect(getTaskIcon('sync_jira')).toBe('🔄')
    expect(getTaskIcon('sync_confluence')).toBe('🔄')
  })

  it('returns sparkle icon for ai_summarize', () => {
    expect(getTaskIcon('ai_summarize')).toBe('✨')
  })

  it('returns label icon for ai_categorize', () => {
    expect(getTaskIcon('ai_categorize')).toBe('🏷️')
  })

  it('returns newspaper icon for generate_daily_digest', () => {
    expect(getTaskIcon('generate_daily_digest')).toBe('📰')
  })

  it('returns chart icon for generate_weekly_digest', () => {
    expect(getTaskIcon('generate_weekly_digest')).toBe('📊')
  })

  it('returns gear icon as fallback for unknown type', () => {
    const unknownType = 'unknown_task' as PipelineTaskType
    expect(getTaskIcon(unknownType)).toBe('⚙️')
  })
})
