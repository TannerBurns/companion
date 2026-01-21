import { useState, useRef, useEffect } from 'react'
import { clsx } from 'clsx'
import { Activity, CheckCircle, XCircle, Loader2, ChevronDown } from 'lucide-react'
import { usePipeline, getTaskDisplayName, type PipelineTask } from '../hooks/usePipeline'

function formatTime(timestamp: number): string {
  const date = new Date(timestamp * 1000)
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function TaskItem({ task }: { task: PipelineTask }) {
  const isRunning = task.status === 'running'
  const isCompleted = task.status === 'completed'
  const isFailed = task.status === 'failed'

  return (
    <div className="flex items-start gap-3 px-3 py-2">
      <div className="mt-0.5">
        {isRunning && <Loader2 className="h-4 w-4 animate-spin text-primary-500" />}
        {isCompleted && <CheckCircle className="h-4 w-4 text-green-500" />}
        {isFailed && <XCircle className="h-4 w-4 text-red-500" />}
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-foreground truncate">{task.message}</p>
        <p className="text-xs text-muted-foreground">
          {getTaskDisplayName(task.task_type)}
          {task.progress !== null && isRunning && (
            <span className="ml-2">{Math.round(task.progress * 100)}%</span>
          )}
        </p>
      </div>
      <span className="text-xs text-muted-foreground">{formatTime(task.started_at)}</span>
    </div>
  )
}

export function PipelineStatus() {
  const { activeTasks, recentHistory, isBusy, taskCount, hasUnseenActivity, markActivitySeen } = usePipeline()
  const [isOpen, setIsOpen] = useState(false)
  const dropdownRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false)
      }
    }

    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  useEffect(() => {
    if (isOpen && hasUnseenActivity) {
      markActivitySeen()
    }
  }, [isOpen, hasUnseenActivity, markActivitySeen])

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={clsx(
          'relative flex items-center gap-1.5 rounded-lg px-2 py-1.5 text-sm transition-colors',
          isBusy
            ? 'bg-primary-50 text-primary-600 dark:bg-primary-900/30 dark:text-primary-400'
            : 'text-muted-foreground hover:bg-muted'
        )}
        title={isBusy ? `${taskCount} task(s) running` : 'Pipeline status'}
      >
        {isBusy ? (
          <Loader2 className="h-4 w-4 animate-spin" />
        ) : (
          <Activity className="h-4 w-4" />
        )}
        {isBusy && <span className="font-medium">{taskCount}</span>}
        <ChevronDown
          className={clsx('h-3 w-3 transition-transform', isOpen && 'rotate-180')}
        />
        {/* Unseen activity indicator */}
        {hasUnseenActivity && !isOpen && (
          <span className="absolute -top-0.5 -right-0.5 flex h-2.5 w-2.5">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-primary-400 opacity-75" />
            <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-primary-500" />
          </span>
        )}
      </button>

      {isOpen && (
        <div className="absolute right-0 top-full z-50 mt-1 w-80 rounded-lg border border-border bg-card shadow-lg">
          {activeTasks.length > 0 && (
            <div className="border-b border-border">
              <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Running
              </div>
              {activeTasks.map((task) => (
                <TaskItem key={task.id} task={task} />
              ))}
            </div>
          )}

          {recentHistory.length > 0 && (
            <div>
              <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Recent
              </div>
              {recentHistory.slice(0, 5).map((task) => (
                <TaskItem key={task.id} task={task} />
              ))}
            </div>
          )}

          {activeTasks.length === 0 && recentHistory.length === 0 && (
            <div className="px-4 py-6 text-center text-sm text-muted-foreground">
              No recent activity
            </div>
          )}
        </div>
      )}
    </div>
  )
}
