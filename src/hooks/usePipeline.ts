import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useAppStore, type LocalActivity } from '../store'

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed'

export type PipelineTaskType =
  | 'sync_slack'
  | 'sync_jira'
  | 'sync_confluence'
  | 'ai_summarize'
  | 'ai_categorize'
  | 'generate_daily_digest'
  | 'generate_weekly_digest'
  | 'pdf_export'

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
  pdf_export: 'Exporting PDF',
}

const taskTypeIcons: Record<PipelineTaskType, string> = {
  sync_slack: '🔄',
  sync_jira: '🔄',
  sync_confluence: '🔄',
  ai_summarize: '✨',
  ai_categorize: '🏷️',
  generate_daily_digest: '📰',
  generate_weekly_digest: '📊',
  pdf_export: '📄',
}

export function getTaskDisplayName(taskType: PipelineTaskType): string {
  return taskTypeDisplayNames[taskType] || taskType
}

export function getTaskIcon(taskType: PipelineTaskType): string {
  return taskTypeIcons[taskType] || '⚙️'
}

// Convert local activity to pipeline task format
function localActivityToPipelineTask(activity: LocalActivity): PipelineTask {
  return {
    id: activity.id,
    task_type: activity.type as PipelineTaskType,
    status: activity.status,
    message: activity.message,
    progress: null,
    started_at: Math.floor(activity.startedAt / 1000), // Convert to seconds
    completed_at: activity.completedAt ? Math.floor(activity.completedAt / 1000) : null,
    error: activity.error,
  }
}

export function usePipeline() {
  const [state, setState] = useState<PipelineState>({
    active_tasks: [],
    recent_history: [],
    is_busy: false,
  })

  const localActivities = useAppStore((s) => s.localActivities)
  const clearOldLocalActivities = useAppStore((s) => s.clearOldLocalActivities)
  const hasUnseenActivity = useAppStore((s) => s.hasUnseenActivity)
  const markActivitySeen = useAppStore((s) => s.markActivitySeen)

  useEffect(() => {
    const unlisten = listen<PipelineState>('pipeline:update', (event) => {
      setState(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  useEffect(() => {
    const interval = setInterval(clearOldLocalActivities, 60000)
    return () => clearInterval(interval)
  }, [clearOldLocalActivities])

  const localRunning = localActivities
    .filter((a) => a.status === 'running')
    .map(localActivityToPipelineTask)
  
  const localCompleted = localActivities
    .filter((a) => a.status !== 'running')
    .map(localActivityToPipelineTask)

  const allActiveTasks = [...localRunning, ...state.active_tasks]
  const allRecentHistory = [...localCompleted, ...state.recent_history]
    .sort((a, b) => (b.completed_at ?? b.started_at) - (a.completed_at ?? a.started_at))
    .slice(0, 10)

  return {
    activeTasks: allActiveTasks,
    recentHistory: allRecentHistory,
    isBusy: state.is_busy || localRunning.length > 0,
    taskCount: allActiveTasks.length,
    hasUnseenActivity,
    markActivitySeen,
  }
}
