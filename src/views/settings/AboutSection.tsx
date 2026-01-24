import { useState, useEffect } from 'react'
import { getVersion, getName } from '@tauri-apps/api/app'
import { clsx } from 'clsx'
import {
  Info,
  Download,
  CheckCircle,
  RefreshCw,
} from 'lucide-react'
import { Button } from '../../components/ui/Button'
import { useUpdater } from '../../hooks/useUpdater'

export function AboutSection() {
  const [appName, setAppName] = useState<string>('')
  const [appVersion, setAppVersion] = useState<string>('')
  const { 
    state, 
    checkForUpdates, 
    downloadAndInstall, 
    handleRestart 
  } = useUpdater()

  useEffect(() => {
    getName().then(setAppName).catch(() => setAppName('Companion'))
    getVersion().then(setAppVersion).catch(() => setAppVersion('Unknown'))
  }, [])

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">About</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Application information and updates.
        </p>
      </div>

      <div className="space-y-4">
        {/* App Info */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
              <Info className="h-5 w-5 text-primary-500" />
            </div>
            <div>
              <h4 className="font-medium text-foreground">{appName || 'Companion'}</h4>
              <p className="text-sm text-muted-foreground">
                Version {appVersion || 'Loading...'}
              </p>
            </div>
          </div>
        </div>

        {/* Check for Updates */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Download className={clsx(
                'h-5 w-5 text-muted-foreground',
                state.status === 'checking' && 'animate-pulse'
              )} />
              <div>
                <h4 className="font-medium text-foreground">Software Updates</h4>
                <p className="text-sm text-muted-foreground">
                  {state.status === 'idle' && 'Check for available updates'}
                  {state.status === 'checking' && 'Checking for updates...'}
                  {state.status === 'no-update' && 'You are running the latest version'}
                  {state.status === 'available' && `Version ${state.update.version} is available`}
                  {state.status === 'downloading' && `Downloading update... ${state.progress}%`}
                  {state.status === 'ready' && 'Update ready to install'}
                  {state.status === 'error' && state.message}
                </p>
              </div>
            </div>
            <div className="flex-shrink-0">
              {(state.status === 'idle' || state.status === 'no-update' || state.status === 'error') && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={checkForUpdates}
                >
                  <RefreshCw className="h-4 w-4" />
                  Check for Updates
                </Button>
              )}
              {state.status === 'checking' && (
                <Button
                  variant="outline"
                  size="sm"
                  disabled
                >
                  <RefreshCw className="h-4 w-4 animate-spin" />
                  Checking...
                </Button>
              )}
              {state.status === 'available' && (
                <Button
                  size="sm"
                  onClick={() => downloadAndInstall()}
                >
                  <Download className="h-4 w-4" />
                  Download & Install
                </Button>
              )}
              {state.status === 'downloading' && (
                <Button
                  variant="outline"
                  size="sm"
                  disabled
                >
                  <RefreshCw className="h-4 w-4 animate-spin" />
                  Downloading...
                </Button>
              )}
              {state.status === 'ready' && (
                <Button
                  size="sm"
                  onClick={handleRestart}
                >
                  <RefreshCw className="h-4 w-4" />
                  Restart Now
                </Button>
              )}
            </div>
          </div>

          {/* Download Progress Bar */}
          {state.status === 'downloading' && (
            <div className="mt-4">
              <div className="w-full bg-muted rounded-full h-2">
                <div 
                  className="bg-primary-500 h-2 rounded-full transition-all duration-300"
                  style={{ width: `${state.progress}%` }}
                />
              </div>
            </div>
          )}

          {/* Update Ready */}
          {state.status === 'ready' && (
            <div className="mt-4 flex items-center gap-2 text-green-600 dark:text-green-400">
              <CheckCircle className="h-4 w-4" />
              <p className="text-sm">Update downloaded and ready to install</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
