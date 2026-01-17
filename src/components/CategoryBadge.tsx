import { clsx } from 'clsx'

const categoryColors: Record<string, string> = {
  sales: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
  marketing: 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200',
  product: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
  engineering: 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200',
  research: 'bg-cyan-100 text-cyan-800 dark:bg-cyan-900 dark:text-cyan-200',
  other: 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200',
}

interface CategoryBadgeProps {
  category: string
  confidence?: number
  className?: string
}

export function CategoryBadge({ category, confidence, className }: CategoryBadgeProps) {
  const colorClass = categoryColors[category.toLowerCase()] || categoryColors.other

  return (
    <span
      className={clsx(
        'inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-medium',
        colorClass,
        className
      )}
    >
      {category}
      {confidence !== undefined && confidence < 0.7 && (
        <span className="opacity-60">?</span>
      )}
    </span>
  )
}
