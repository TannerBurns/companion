import { useState, useCallback } from 'react'
import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

export type UpdateState = 
  | { status: 'idle' }
  | { status: 'checking' }
  | { status: 'no-update' }
  | { status: 'available'; update: Update }
  | { status: 'downloading'; progress: number; downloaded: number; contentLength: number | null; update: Update }
  | { status: 'ready'; update: Update }
  | { status: 'error'; message: string; update?: Update }

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

export function useUpdater() {
  const [state, setState] = useState<UpdateState>({ status: 'idle' })
  const [dismissedVersion, setDismissedVersion] = useState<string | null>(getDismissedVersion())

  const checkForUpdates = useCallback(async () => {
    try {
      setState({ status: 'checking' })
      const update = await check()
      
      if (update) {
        setState({ status: 'available', update })
      } else {
        setState({ status: 'no-update' })
      }
    } catch (error) {
      console.error('Failed to check for updates:', error)
      
      const errorMessage = error instanceof Error ? error.message : 'Failed to check for updates'
      
      if (errorMessage.includes('Could not fetch a valid release')) {
        setState({ status: 'no-update' })
        return
      }
      
      let friendlyMessage = errorMessage
      if (errorMessage.includes('network') || errorMessage.includes('fetch')) {
        friendlyMessage = 'Unable to connect. Check your internet connection.'
      }
      
      setState({ 
        status: 'error', 
        message: friendlyMessage 
      })
    }
  }, [])

  const downloadAndInstall = useCallback(async (updateToInstall?: Update) => {
    const update = updateToInstall ?? (
      state.status === 'available' ? state.update : 
      state.status === 'error' && state.update ? state.update : 
      null
    )
    if (!update) return
    
    try {
      setState({ status: 'downloading', progress: 0, downloaded: 0, contentLength: null, update })
      
      await update.downloadAndInstall((event: DownloadEvent) => {
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
              const progress = prev.contentLength 
                ? Math.round((newDownloaded / prev.contentLength) * 100)
                : 0
              
              return { 
                status: 'downloading', 
                progress: Math.min(progress, 100), 
                downloaded: newDownloaded,
                contentLength: prev.contentLength,
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
  }, [state])

  const handleRestart = useCallback(async () => {
    try {
      await relaunch()
    } catch (error) {
      console.error('Failed to relaunch:', error)
    }
  }, [])

  const dismiss = useCallback(() => {
    if (state.status === 'available' || state.status === 'downloading' || state.status === 'ready' || (state.status === 'error' && state.update)) {
      const version = 'update' in state && state.update ? state.update.version : null
      if (version) {
        setDismissedVersion(version)
        setDismissedVersionStorage(version)
      }
    }
    setState({ status: 'idle' })
  }, [state])

  const currentVersion = 'update' in state && state.update ? state.update.version : null
  const isDismissed = currentVersion !== null && dismissedVersion === currentVersion

  return {
    state,
    checkForUpdates,
    downloadAndInstall,
    handleRestart,
    dismiss,
    isDismissed,
  }
}
