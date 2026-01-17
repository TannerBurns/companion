import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed'

export type PipelineTaskType =
  | 'sync_slack'
  | 'sync_jira'
  | 'sync_confluence'
  | 'ai_summarize'
  | 'ai_categorize'
  | 'generate_daily_digest'
  | 'generate_weekly_digest'

export interface PipelineTask {
  id: string
  task_type: PipelineTaskType
  status: TaskStatus
  message: string
  progress: number | null
  started_at: number
  completed_at: number | null
  error: string | null
}

export interface PipelineState {
  active_tasks: PipelineTask[]
  recent_history: PipelineTask[]
  is_busy: boolean
}

const taskTypeDisplayNames: Record<PipelineTaskType, string> = {
  sync_slack: 'Syncing Slack',
  sync_jira: 'Syncing Jira',
  sync_confluence: 'Syncing Confluence',
  ai_summarize: 'Summarizing content',
  ai_categorize: 'Categorizing items',
  generate_daily_digest: 'Generating daily digest',
  generate_weekly_digest: 'Generating weekly digest',
}

const taskTypeIcons: Record<PipelineTaskType, string> = {
  sync_slack: '🔄',
  sync_jira: '🔄',
  sync_confluence: '🔄',
  ai_summarize: '✨',
  ai_categorize: '🏷️',
  generate_daily_digest: '📰',
  generate_weekly_digest: '📊',
}

export function getTaskDisplayName(taskType: PipelineTaskType): string {
  return taskTypeDisplayNames[taskType] || taskType
}

export function getTaskIcon(taskType: PipelineTaskType): string {
  return taskTypeIcons[taskType] || '⚙️'
}

export function usePipeline() {
  const [state, setState] = useState<PipelineState>({
    active_tasks: [],
    recent_history: [],
    is_busy: false,
  })

  useEffect(() => {
    const unlisten = listen<PipelineState>('pipeline:update', (event) => {
      setState(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  return {
    activeTasks: state.active_tasks,
    recentHistory: state.recent_history,
    isBusy: state.is_busy,
    taskCount: state.active_tasks.length,
  }
}
