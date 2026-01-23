import { useEffect } from 'react'
import { Download, RefreshCw, X, CheckCircle } from 'lucide-react'
import { Button } from './ui'
import { useUpdater } from '../hooks/useUpdater'

export function UpdateNotification() {
  const { 
    state, 
    checkForUpdates, 
    downloadAndInstall, 
    handleRestart, 
    dismiss, 
    isDismissed 
  } = useUpdater()

  useEffect(() => {
    // Check for updates on mount, with a small delay to not block app startup
    const timer = setTimeout(() => {
      checkForUpdates()
    }, 3000)

    return () => clearTimeout(timer)
  }, [checkForUpdates])

  if (state.status === 'idle' || state.status === 'checking' || state.status === 'no-update') {
    return null
  }

  if (state.status === 'error' && !state.update) {
    return null
  }

  if (isDismissed) {
    return null
  }

  return (
    <div className="fixed bottom-4 right-4 z-50 w-80 rounded-lg bg-card border border-border shadow-lg overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-primary-500 text-white">
        <div className="flex items-center gap-2">
          <Download className="h-4 w-4" />
          <span className="font-medium">Update Available</span>
        </div>
        {state.status !== 'ready' && (
          <button
            onClick={dismiss}
            className="p-1 hover:bg-primary-600 rounded transition-colors"
            aria-label="Dismiss"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      {/* Content */}
      <div className="p-4">
        {state.status === 'available' && (
          <>
            <p className="text-sm text-muted-foreground mb-3">
              A new version ({state.update.version}) is available. 
              Download and install it to get the latest features and fixes.
            </p>
            <Button 
              onClick={() => downloadAndInstall()}
              className="w-full"
              size="sm"
            >
              <Download className="h-4 w-4" />
              Download & Install
            </Button>
          </>
        )}

        {state.status === 'downloading' && (
          <>
            <p className="text-sm text-muted-foreground mb-3">
              Downloading update...
            </p>
            <div className="w-full bg-muted rounded-full h-2 mb-2">
              <div 
                className="bg-primary-500 h-2 rounded-full transition-all duration-300"
                style={{ width: `${state.progress}%` }}
              />
            </div>
            <p className="text-xs text-muted-foreground text-center">
              {state.progress}%
            </p>
          </>
        )}

        {state.status === 'ready' && (
          <>
            <div className="flex items-center gap-2 text-green-600 dark:text-green-400 mb-3">
              <CheckCircle className="h-4 w-4" />
              <p className="text-sm font-medium">Update ready to install</p>
            </div>
            <p className="text-sm text-muted-foreground mb-3">
              The update has been downloaded. Restart the app to apply changes.
            </p>
            <Button 
              onClick={handleRestart}
              className="w-full"
              size="sm"
            >
              <RefreshCw className="h-4 w-4" />
              Restart Now
            </Button>
          </>
        )}

        {state.status === 'error' && (
          <>
            <p className="text-sm text-red-600 dark:text-red-400 mb-3">
              {state.message}
            </p>
            <Button 
              onClick={() => downloadAndInstall(state.update)}
              variant="outline"
              className="w-full"
              size="sm"
            >
              Try Again
            </Button>
          </>
        )}
      </div>
    </div>
  )
}
