import { useState } from 'react'
import { ChevronDown, ChevronUp, Hash, Users, MessageSquare } from 'lucide-react'
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

      {/* Source Info - shown when expanded */}
      {expanded && (item.channels?.length || item.people?.length || item.messageCount) && (
        <div className="mt-4 pt-3 border-t border-border space-y-2">
          {/* Channels */}
          {item.channels && item.channels.length > 0 && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Hash className="h-3 w-3 flex-shrink-0" />
              <span className="flex flex-wrap gap-1">
                {item.channels.map((channel, index) => (
                  <span key={index} className="text-foreground">
                    {channel}{index < item.channels!.length - 1 ? ',' : ''}
                  </span>
                ))}
              </span>
            </div>
          )}
          
          {/* People */}
          {item.people && item.people.length > 0 && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Users className="h-3 w-3 flex-shrink-0" />
              <span className="flex flex-wrap gap-1">
                {item.people.slice(0, 5).map((person, index) => (
                  <span key={index} className="text-foreground">
                    {person}{index < Math.min(item.people!.length, 5) - 1 ? ',' : ''}
                  </span>
                ))}
                {item.people.length > 5 && (
                  <span className="text-muted-foreground">+{item.people.length - 5} more</span>
                )}
              </span>
            </div>
          )}
          
          {/* Message Count */}
          {item.messageCount && item.messageCount > 0 && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <MessageSquare className="h-3 w-3 flex-shrink-0" />
              <span>
                <span className="text-foreground">{item.messageCount}</span> messages
              </span>
            </div>
          )}
        </div>
      )}

      {item.highlights && item.highlights.length > 0 && (
        <div className="mt-3">
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
        </div>
      )}
    </Card>
  )
}
