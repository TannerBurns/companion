import { useState, useEffect, useCallback } from 'react'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { Download, RefreshCw, X, CheckCircle } from 'lucide-react'
import { Button } from './ui'

type UpdateState = 
  | { status: 'idle' }
  | { status: 'checking' }
  | { status: 'available'; update: Update }
  | { status: 'downloading'; progress: number; downloaded: number; contentLength: number | null; update: Update }
  | { status: 'ready'; update: Update }
  | { status: 'error'; message: string; update: Update }

const DISMISSED_VERSION_KEY = 'update-notification-dismissed-version'

function getDismissedVersion(): string | null {
  try {
    return localStorage.getItem(DISMISSED_VERSION_KEY)
  } catch {
    return null
  }
}

function setDismissedVersionStorage(version: string): void {
  try {
    localStorage.setItem(DISMISSED_VERSION_KEY, version)
  } catch {
    // Ignore storage errors
  }
}

export function UpdateNotification() {
  const [state, setState] = useState<UpdateState>({ status: 'idle' })
  // Track the dismissed version in localStorage so it persists across app restarts
  const [dismissedVersion, setDismissedVersion] = useState<string | null>(getDismissedVersion())

  const checkForUpdates = useCallback(async () => {
    try {
      setState({ status: 'checking' })
      const update = await check()
      
      if (update) {
        setState({ status: 'available', update })
      } else {
        setState({ status: 'idle' })
      }
    } catch (error) {
      console.error('Failed to check for updates:', error)
      setState({ status: 'idle' })
    }
  }, [])

  useEffect(() => {
    // Check for updates on mount, with a small delay to not block app startup
    const timer = setTimeout(() => {
      checkForUpdates()
    }, 3000)

    return () => clearTimeout(timer)
  }, [checkForUpdates])

  const handleDownloadAndInstall = async (updateToInstall?: Update) => {
    const update = updateToInstall ?? (state.status === 'available' ? state.update : state.status === 'error' ? state.update : null)
    if (!update) return
    
    try {
      setState({ status: 'downloading', progress: 0, downloaded: 0, contentLength: null, update })
      
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            setState({ 
              status: 'downloading', 
              progress: 0, 
              downloaded: 0, 
              contentLength: event.data.contentLength ?? null, 
              update 
            })
            break
          case 'Progress':
            setState((prev) => {
              if (prev.status !== 'downloading') return prev
              
              const newDownloaded = prev.downloaded + event.data.chunkLength
              const totalLength = prev.contentLength ?? event.data.contentLength
              const progress = totalLength 
                ? Math.round((newDownloaded / totalLength) * 100)
                : 0
              
              return { 
                status: 'downloading', 
                progress: Math.min(progress, 100), 
                downloaded: newDownloaded,
                contentLength: totalLength ?? null,
                update 
              }
            })
            break
          case 'Finished':
            setState({ status: 'ready', update })
            break
        }
      })
    } catch (error) {
      console.error('Failed to download update:', error)
      setState({ 
        status: 'error', 
        message: error instanceof Error ? error.message : 'Failed to download update',
        update
      })
    }
  }

  const handleRestart = async () => {
    try {
      await relaunch()
    } catch (error) {
      console.error('Failed to relaunch:', error)
    }
  }

  const handleDismiss = () => {
    // Store the dismissed version so new versions can still show notifications
    // Persisted to localStorage so it survives app restarts
    if (state.status === 'available' || state.status === 'downloading' || state.status === 'ready' || state.status === 'error') {
      const version = state.update.version
      setDismissedVersion(version)
      setDismissedVersionStorage(version)
    }
  }

  // Don't render if nothing to show
  if (state.status === 'idle' || state.status === 'checking') {
    return null
  }

  // Don't render if this specific version was dismissed
  const currentVersion = 'update' in state ? state.update.version : null
  if (currentVersion && dismissedVersion === currentVersion) {
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
            onClick={handleDismiss}
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
              onClick={handleDownloadAndInstall}
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
              onClick={() => handleDownloadAndInstall(state.update)}
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
