import { WifiOff } from 'lucide-react'
import { useConnectionStatus } from '../hooks/useConnectionStatus'

export function OfflineIndicator() {
  const isOnline = useConnectionStatus()

  if (isOnline) return null

  return (
    <div className="fixed bottom-4 left-4 z-50 flex items-center gap-2 rounded-lg bg-yellow-100 px-3 py-2 text-sm text-yellow-800 shadow-lg dark:bg-yellow-900 dark:text-yellow-200">
      <WifiOff className="h-4 w-4" />
      <span>Offline - viewing cached data</span>
    </div>
  )
}
