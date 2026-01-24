import { useEffect, useCallback } from 'react'
import { X, ExternalLink, Hash, Users, MessageSquare } from 'lucide-react'
import { CategoryBadge } from './CategoryBadge'
import { SourceIcon } from './SourceIcon'
import { ImportanceIndicator } from './ImportanceIndicator'
import { Button } from './ui/Button'
import type { DigestItem } from '../lib/api'

interface ContentDetailModalProps {
  item: DigestItem | null
  onClose: () => void
}

export function ContentDetailModal({ item, onClose }: ContentDetailModalProps) {
  const handleEscape = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose()
    }
  }, [onClose])

  useEffect(() => {
    if (item) {
      document.addEventListener('keydown', handleEscape)
      // Prevent body scroll when modal is open
      document.body.style.overflow = 'hidden'
    }
    return () => {
      document.removeEventListener('keydown', handleEscape)
      document.body.style.overflow = ''
    }
  }, [item, handleEscape])

  if (!item) return null

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose()
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={handleBackdropClick}
    >
      <div className="bg-background border border-border rounded-xl shadow-xl w-full max-w-2xl max-h-[85vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-start justify-between p-6 border-b border-border">
          <div className="flex-1 pr-4">
            <div className="flex items-center gap-3 mb-3">
              <SourceIcon source={item.source} />
              <span className="text-sm text-muted-foreground">
                {new Date(item.createdAt).toLocaleDateString('en-US', {
                  weekday: 'long',
                  year: 'numeric',
                  month: 'long',
                  day: 'numeric',
                })}
              </span>
            </div>
            <div className="flex items-center gap-2 mb-2">
              <ImportanceIndicator score={item.importanceScore} />
              <CategoryBadge category={item.category} confidence={item.categoryConfidence} />
            </div>
            <h2 className="text-xl font-semibold text-foreground">
              {item.title || item.summary.slice(0, 60)}
            </h2>
          </div>
          <button
            onClick={onClose}
            className="rounded-lg p-2 hover:bg-muted transition-colors text-muted-foreground hover:text-foreground"
            aria-label="Close"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          {/* Summary */}
          <div className="mb-6">
            <h3 className="text-sm font-medium text-foreground mb-2">Summary</h3>
            <p className="text-muted-foreground leading-relaxed">{item.summary}</p>
          </div>

          {/* Highlights */}
          {item.highlights && item.highlights.length > 0 && (
            <div className="mb-6">
              <h3 className="text-sm font-medium text-foreground mb-3">Key Highlights</h3>
              <ul className="space-y-2">
                {item.highlights.map((highlight, index) => (
                  <li key={index} className="flex items-start gap-3 text-muted-foreground">
                    <span className="text-primary-500 mt-0.5 flex-shrink-0">•</span>
                    <span>{highlight}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* Source Info - Channels, People, Message Count */}
          {(item.channels?.length || item.people?.length || item.messageCount) && (
            <div className="p-4 bg-muted/50 rounded-lg space-y-3">
              <h3 className="text-sm font-medium text-foreground">Sources</h3>
              
              {/* Channels */}
              {item.channels && item.channels.length > 0 && (
                <div className="flex items-start gap-3">
                  <Hash className="h-4 w-4 text-muted-foreground mt-0.5 flex-shrink-0" />
                  <div>
                    <div className="text-xs text-muted-foreground mb-1">Channels</div>
                    <div className="flex flex-wrap gap-1.5">
                      {item.channels.map((channel, index) => (
                        <span
                          key={index}
                          className="inline-flex items-center px-2 py-0.5 rounded bg-background text-sm text-foreground"
                        >
                          {channel}
                        </span>
                      ))}
                    </div>
                  </div>
                </div>
              )}
              
              {/* People */}
              {item.people && item.people.length > 0 && (
                <div className="flex items-start gap-3">
                  <Users className="h-4 w-4 text-muted-foreground mt-0.5 flex-shrink-0" />
                  <div>
                    <div className="text-xs text-muted-foreground mb-1">People</div>
                    <div className="flex flex-wrap gap-1.5">
                      {item.people.map((person, index) => (
                        <span
                          key={index}
                          className="inline-flex items-center px-2 py-0.5 rounded bg-background text-sm text-foreground"
                        >
                          {person}
                        </span>
                      ))}
                    </div>
                  </div>
                </div>
              )}
              
              {/* Message Count */}
              {item.messageCount && item.messageCount > 0 && (
                <div className="flex items-center gap-3">
                  <MessageSquare className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                  <div>
                    <span className="text-sm text-muted-foreground">
                      Based on <span className="font-medium text-foreground">{item.messageCount}</span> messages
                    </span>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        {item.sourceUrl && (
          <div className="flex items-center justify-end gap-3 p-6 border-t border-border bg-muted/30">
            <Button
              onClick={() => window.open(item.sourceUrl, '_blank', 'noopener,noreferrer')}
            >
              <ExternalLink className="h-4 w-4" />
              View in {item.source === 'slack' ? 'Slack' : item.source === 'confluence' ? 'Confluence' : item.source === 'ai' ? 'AI' : 'Source'}
            </Button>
          </div>
        )}
      </div>
    </div>
  )
}
