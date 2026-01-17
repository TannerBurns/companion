import { clsx } from 'clsx'

interface ImportanceIndicatorProps {
  score: number
  showLabel?: boolean
  className?: string
}

export function ImportanceIndicator({ score, showLabel = false, className }: ImportanceIndicatorProps) {
  const level = score >= 0.8 ? 'high' : score >= 0.5 ? 'medium' : 'low'

  const levelColors = {
    high: 'text-red-500',
    medium: 'text-yellow-500',
    low: 'text-gray-400',
  }

  const levelLabels = {
    high: 'High',
    medium: 'Medium',
    low: 'Low',
  }

  return (
    <div className={clsx('flex items-center gap-1.5', className)}>
      <div className="flex items-center gap-0.5">
        {[0, 1, 2].map((i) => (
          <div
            key={i}
            className={clsx(
              'h-1.5 w-1.5 rounded-full',
              i === 0 && levelColors[level],
              i === 0 && 'bg-current',
              i === 1 && (level !== 'low' ? `${levelColors[level]} bg-current` : 'bg-gray-300 dark:bg-gray-600'),
              i === 2 && (level === 'high' ? `${levelColors[level]} bg-current` : 'bg-gray-300 dark:bg-gray-600')
            )}
          />
        ))}
      </div>
      {showLabel && (
        <span className={clsx('text-xs font-medium', levelColors[level])}>
          {levelLabels[level]}
        </span>
      )}
    </div>
  )
}
