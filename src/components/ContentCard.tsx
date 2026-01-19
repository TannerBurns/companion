import { useState } from 'react'
import { ExternalLink, ChevronDown, ChevronUp } from 'lucide-react'
import { Card } from './ui/Card'
import { CategoryBadge } from './CategoryBadge'
import { SourceIcon } from './SourceIcon'
import { ImportanceIndicator } from './ImportanceIndicator'
import type { DigestItem } from '../lib/api'

interface ContentCardProps {
  item: DigestItem
  onViewDetail?: (id: string) => void
}

export function ContentCard({ item, onViewDetail }: ContentCardProps) {
  const [expanded, setExpanded] = useState(false)

  return (
    <Card hoverable className="group">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2 text-muted-foreground">
          <SourceIcon source={item.source} />
          <span className="text-xs">
            {new Date(item.createdAt).toLocaleDateString()}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <ImportanceIndicator score={item.importanceScore} />
          <CategoryBadge category={item.category} confidence={item.categoryConfidence} />
        </div>
      </div>

      <h3
        className="mt-2 font-medium text-foreground line-clamp-2 cursor-pointer hover:text-primary-500"
        onClick={() => onViewDetail?.(item.id)}
      >
        {item.title || item.summary.slice(0, 60)}
      </h3>

      <p className={`mt-1 text-sm text-muted-foreground ${expanded ? '' : 'line-clamp-2'}`}>
        {item.summary}
      </p>

      {expanded && item.highlights && item.highlights.length > 0 && (
        <ul className="mt-3 space-y-1">
          {item.highlights.map((h, i) => (
            <li key={i} className="flex items-start gap-2 text-sm text-muted-foreground">
              <span className="text-primary-500 mt-0.5">•</span>
              <span>{h}</span>
            </li>
          ))}
        </ul>
      )}

      <div className="mt-3 flex items-center justify-between">
        {item.highlights && item.highlights.length > 0 ? (
          <button
            onClick={(e) => {
              e.stopPropagation()
              setExpanded(!expanded)
            }}
            className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
          >
            {expanded ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
            {expanded ? 'Less' : 'More'}
          </button>
        ) : (
          <div />
        )}

        <div className="flex items-center gap-2">
          {item.sourceUrl && (
            <a
              href={item.sourceUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 text-xs text-primary-500 hover:underline"
              onClick={(e) => e.stopPropagation()}
            >
              <ExternalLink className="h-3 w-3" />
              View Source
            </a>
          )}
        </div>
      </div>
    </Card>
  )
}
