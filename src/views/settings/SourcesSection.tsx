import { useState, useEffect, useCallback } from 'react'
import {
  Slack,
  FileText,
  Link2,
  Settings,
  RefreshCw,
} from 'lucide-react'
import { useAppStore } from '../../store'
import { SourceCard, SlackChannelSelector } from '../../components'
import { Button } from '../../components/ui/Button'
import { Input } from '../../components/ui/Input'
import { api } from '../../lib/api'

export function SourcesSection() {
  const { slack, setSlackState, showChannelSelector, setShowChannelSelector } = useAppStore()
  const [isConnecting, setIsConnecting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [showSlackSetup, setShowSlackSetup] = useState(false)
  const [slackToken, setSlackToken] = useState('')

  const loadSlackStatus = useCallback(async () => {
    try {
      const status = await api.getSlackConnectionStatus()
      setSlackState({
        connected: status.connected,
        teamId: status.teamId ?? null,
        teamName: status.teamName ?? null,
        userId: status.userId ?? null,
        selectedChannelCount: status.selectedChannelCount,
      })
    } catch (e) {
      console.error('Failed to load Slack status:', e)
    }
  }, [setSlackState])

  useEffect(() => {
    loadSlackStatus()
  }, [loadSlackStatus])

  const handleConnectSlack = async () => {
    if (!slackToken.trim()) {
      setError('Please enter your Slack token')
      return
    }

    setIsConnecting(true)
    setError(null)
    try {
      const tokens = await api.connectSlack(slackToken.trim())
      setSlackState({
        connected: true,
        teamId: tokens.teamId,
        teamName: tokens.teamName,
        userId: tokens.userId,
        selectedChannelCount: 0,
      })
      setShowSlackSetup(false)
      setSlackToken('')
      // Show channel selector after successful connection
      setShowChannelSelector(true)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to connect to Slack')
    } finally {
      setIsConnecting(false)
    }
  }

  const handleDisconnectSlack = async () => {
    try {
      await api.disconnectSlack()
      setSlackState({
        connected: false,
        teamId: null,
        teamName: null,
        userId: null,
        selectedChannelCount: 0,
      })
      setError(null)
    } catch (e) {
      console.error('Failed to disconnect Slack:', e)
    }
  }

  const sources = [
    {
      id: 'slack',
      icon: Slack,
      name: 'Slack',
      description: slack.connected && slack.teamName
        ? `Connected to ${slack.teamName} • ${slack.selectedChannelCount} channels`
        : 'Sync messages and channels from your Slack workspace',
      connected: slack.connected,
      isConnecting: isConnecting,
    },
    {
      id: 'confluence',
      icon: FileText,
      name: 'Confluence',
      description: 'Sync pages and spaces from Atlassian Confluence',
      connected: false,
      isConnecting: false,
      comingSoon: true,
    },
  ]

  const handleConnect = (sourceId: string) => {
    if (sourceId === 'slack') {
      setShowSlackSetup(true)
    }
  }

  const handleDisconnect = (sourceId: string) => {
    if (sourceId === 'slack') {
      handleDisconnectSlack()
    }
  }

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Connected Sources</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Connect your accounts to sync data and generate AI-powered summaries.
        </p>
      </div>

      <div className="space-y-3">
        {sources.map((source) => (
          <SourceCard
            key={source.id}
            icon={source.icon}
            name={source.name}
            description={source.description}
            connected={source.connected}
            isConnecting={source.isConnecting}
            comingSoon={source.comingSoon}
            onConnect={() => handleConnect(source.id)}
            onDisconnect={() => handleDisconnect(source.id)}
          >
            {/* Slack-specific content */}
            {source.id === 'slack' && slack.connected && (
              <button
                onClick={() => setShowChannelSelector(true)}
                className="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-primary-500 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors"
              >
                <Settings className="h-4 w-4" />
                Configure Channels
              </button>
            )}
            {source.id === 'slack' && !slack.connected && (
              <div className="flex items-start gap-3 p-3 bg-muted/50 rounded-lg">
                <Link2 className="h-5 w-5 text-muted-foreground mt-0.5" />
                <div>
                  <h4 className="text-sm font-medium text-foreground">
                    How It Works
                  </h4>
                  <p className="text-sm text-muted-foreground mt-1">
                    Create a Slack app for your workspace, install it, and paste the
                    User OAuth Token. Your token is stored securely on your device.
                  </p>
                </div>
              </div>
            )}
          </SourceCard>
        ))}
      </div>

      {/* Error display */}
      {error && (
        <div className="mt-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400">
          {error}
        </div>
      )}

      {/* Slack Token Setup Modal */}
      {showSlackSetup && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-background border border-border rounded-xl shadow-xl w-full max-w-lg p-6">
            <h3 className="text-lg font-semibold text-foreground mb-2">
              Connect Slack
            </h3>
            <p className="text-sm text-muted-foreground mb-4">
              To connect Slack, create a Slack app and paste your User OAuth Token.
            </p>
            
            <div className="p-4 bg-muted/50 rounded-lg mb-4">
              <h4 className="text-sm font-medium text-foreground mb-2">Setup Steps:</h4>
              <ol className="text-sm text-muted-foreground space-y-2 list-decimal list-inside">
                <li>
                  Go to{' '}
                  <a
                    href="https://api.slack.com/apps"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-primary-500 hover:underline"
                  >
                    api.slack.com/apps
                  </a>
                  {' '}→ Create New App → From scratch
                </li>
                <li>Name it anything (e.g., "Companion"), select your workspace</li>
                <li>Go to <strong>OAuth & Permissions</strong>, add these <strong>User Token Scopes</strong>:
                  <code className="block mt-1 p-2 bg-background rounded text-xs">
                    channels:history, channels:read, groups:history, groups:read, im:history, im:read, mpim:history, mpim:read, users:read
                  </code>
                </li>
                <li>Click <strong>Install to Workspace</strong> and authorize</li>
                <li>Copy the <strong>User OAuth Token</strong> (starts with <code className="text-xs">xoxp-</code>)</li>
              </ol>
            </div>

            {error && (
              <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400">
                {error}
              </div>
            )}

            <div>
              <label className="block text-sm font-medium text-foreground mb-1">
                User OAuth Token
              </label>
              <Input
                type="password"
                value={slackToken}
                onChange={(e) => setSlackToken(e.target.value)}
                placeholder="xoxp-..."
              />
            </div>

            <div className="flex justify-end gap-3 mt-6">
              <Button
                variant="outline"
                onClick={() => {
                  setShowSlackSetup(false)
                  setSlackToken('')
                  setError(null)
                }}
              >
                Cancel
              </Button>
              <Button
                onClick={handleConnectSlack}
                disabled={isConnecting || !slackToken.trim()}
              >
                {isConnecting ? (
                  <RefreshCw className="h-4 w-4 animate-spin" />
                ) : (
                  <Slack className="h-4 w-4" />
                )}
                Connect
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Channel Selector Modal */}
      <SlackChannelSelector
        isOpen={showChannelSelector}
        onClose={() => setShowChannelSelector(false)}
        teamId={slack.teamId ?? ''}
        onSave={loadSlackStatus}
      />
    </div>
  )
}
