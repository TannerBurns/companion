import { MessageSquare, FileText, CheckSquare, Sparkles } from 'lucide-react'
import { clsx } from 'clsx'

interface SourceIconProps {
  source: 'slack' | 'jira' | 'confluence' | 'ai'
  className?: string
}

export function SourceIcon({ source, className = 'h-4 w-4' }: SourceIconProps) {
  const iconClass = clsx(className, 'flex-shrink-0')

  switch (source) {
    case 'slack':
      return <MessageSquare className={iconClass} />
    case 'jira':
      return <CheckSquare className={iconClass} />
    case 'confluence':
      return <FileText className={iconClass} />
    case 'ai':
      return <Sparkles className={iconClass} />
  }
}
