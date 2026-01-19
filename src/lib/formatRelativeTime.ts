/**
 * Formats a timestamp as a relative time string.
 * @param timestamp - Unix timestamp in milliseconds, or undefined
 * @param isFuture - If true, formats as future time; if false, formats as past time
 * @param now - Optional current time for testing (defaults to new Date())
 */
export function formatRelativeTime(
  timestamp: number | undefined,
  isFuture: boolean,
  now: Date = new Date()
): string {
  if (!timestamp) return isFuture ? 'Not scheduled' : 'Never'
  
  const date = new Date(timestamp)
  const diffMs = isFuture ? date.getTime() - now.getTime() : now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  
  if (diffMins < 1) return isFuture ? 'Less than a minute' : 'Just now'
  if (diffMins < 60) return `${diffMins} minute${diffMins === 1 ? '' : 's'}${isFuture ? '' : ' ago'}`
  
  const diffHours = Math.floor(diffMins / 60)
  if (diffHours < 24) return `${diffHours} hour${diffHours === 1 ? '' : 's'}${isFuture ? '' : ' ago'}`
  
  return date.toLocaleDateString(undefined, { 
    month: 'short', 
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit'
  })
}
