import { useState, useEffect } from 'react'
import { clsx } from 'clsx'
import {
  FileText,
  Key,
  Save,
  RefreshCw,
  Check,
} from 'lucide-react'
import { Button } from '../components/ui/Button'
import { Input } from '../components/ui/Input'
import { useApiKey } from '../hooks/usePreferences'
import { api } from '../lib/api'

// Custom Google icon component
function GoogleIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
      <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" />
      <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" />
      <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" />
    </svg>
  )
}

export { GoogleIcon }

type GeminiAuthType = 'api_key' | 'service_account' | 'none'

export function GeminiSettings() {
  const [geminiKey, setGeminiKey] = useState('')
  const [currentAuthType, setCurrentAuthType] = useState<GeminiAuthType>('none')
  const [isLoadingAuthType, setIsLoadingAuthType] = useState(true)
  const [jsonFileName, setJsonFileName] = useState<string | null>(null)
  const [jsonContent, setJsonContent] = useState<string | null>(null)
  const [vertexRegion, setVertexRegion] = useState('global')
  const [isVerifying, setIsVerifying] = useState(false)
  const [verifyResult, setVerifyResult] = useState<{ success: boolean; message: string } | null>(null)
  const [isSavingCredentials, setIsSavingCredentials] = useState(false)
  const [saveCredentialsSuccess, setSaveCredentialsSuccess] = useState(false)
  const { hasKey, isLoading, saveApiKey, isSaving, isSuccess } = useApiKey('gemini')

  useEffect(() => {
    const loadAuthType = async () => {
      try {
        const type = await api.getGeminiAuthType()
        setCurrentAuthType(type)
      } catch (e) {
        console.error('Failed to load auth type:', e)
      } finally {
        setIsLoadingAuthType(false)
      }
    }
    loadAuthType()
  }, [])

  const handleSaveGemini = () => {
    if (geminiKey.trim()) {
      saveApiKey(geminiKey.trim())
      setGeminiKey('')
      setCurrentAuthType('api_key')
      setVerifyResult(null)
    }
  }

  const handleFileUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file) return

    const reader = new FileReader()
    reader.onload = (e) => {
      const content = e.target?.result as string
      try {
        const parsed = JSON.parse(content)
        if (!parsed.client_email || !parsed.private_key || !parsed.project_id) {
          setVerifyResult({ success: false, message: 'Invalid service account JSON: missing required fields' })
          return
        }
        setJsonContent(content)
        setJsonFileName(file.name)
        setVerifyResult(null)
        setSaveCredentialsSuccess(false)
      } catch {
        setVerifyResult({ success: false, message: 'Invalid JSON file' })
      }
    }
    reader.readAsText(file)
  }

  const handleSaveCredentials = async () => {
    if (!jsonContent) return

    setIsSavingCredentials(true)
    setSaveCredentialsSuccess(false)
    setVerifyResult(null)
    try {
      await api.saveGeminiCredentials(jsonContent, vertexRegion || undefined)
      setCurrentAuthType('service_account')
      setSaveCredentialsSuccess(true)
      setTimeout(() => setSaveCredentialsSuccess(false), 3000)
    } catch (e) {
      setVerifyResult({ success: false, message: e instanceof Error ? e.message : 'Failed to save credentials' })
    } finally {
      setIsSavingCredentials(false)
    }
  }

  const handleVerifyConnection = async () => {
    setIsVerifying(true)
    setVerifyResult(null)
    try {
      await api.verifyGeminiConnection()
      setVerifyResult({ success: true, message: 'Connection verified successfully!' })
    } catch (e) {
      setVerifyResult({ success: false, message: e instanceof Error ? e.message : 'Verification failed' })
    } finally {
      setIsVerifying(false)
    }
  }

  const isConfigured = currentAuthType !== 'none' || hasKey

  return (
    <div>
      <div className="mb-6">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-semibold text-foreground">Gemini</h3>
            <p className="text-sm text-muted-foreground mt-1">
              Configure Google Gemini for AI-powered summarization and categorization.
            </p>
          </div>
          {!isLoadingAuthType && !isLoading && (
            <div className="flex items-center gap-1.5">
              <div className={clsx(
                'h-2 w-2 rounded-full',
                isConfigured ? 'bg-green-500' : 'bg-yellow-500'
              )} />
              <span className={clsx(
                'text-sm',
                isConfigured 
                  ? 'text-green-600 dark:text-green-400'
                  : 'text-yellow-600 dark:text-yellow-400'
              )}>
                {currentAuthType === 'service_account' 
                  ? 'Service Account Active' 
                  : currentAuthType === 'api_key' || hasKey
                    ? 'API Key Active'
                    : 'Not configured'}
              </span>
            </div>
          )}
        </div>
      </div>

      <div className="space-y-6">
        {/* AI Studio API Key Section */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <Key className="h-4 w-4 text-muted-foreground" />
              <h4 className="font-medium text-foreground">Google AI Studio</h4>
            </div>
            {(currentAuthType === 'api_key' || (hasKey && currentAuthType !== 'service_account')) && (
              <div className="flex items-center gap-1.5">
                <div className="h-2 w-2 rounded-full bg-green-500" />
                <span className="text-xs text-green-600 dark:text-green-400">Active</span>
              </div>
            )}
          </div>
          <p className="text-sm text-muted-foreground mb-4">
            Use an API key from Google AI Studio. Best for personal projects and quick setup.
          </p>
          
          <div className="flex gap-2">
            <Input
              type="password"
              value={geminiKey}
              onChange={e => setGeminiKey(e.target.value)}
              placeholder={hasKey || currentAuthType === 'api_key' ? "Enter new key to replace existing" : "Enter your Gemini API key"}
              className="flex-1"
            />
            <Button
              onClick={handleSaveGemini}
              disabled={isSaving || !geminiKey.trim()}
            >
              {isSaving ? (
                <RefreshCw className="h-4 w-4 animate-spin" />
              ) : isSuccess ? (
                <Check className="h-4 w-4" />
              ) : (
                <Save className="h-4 w-4" />
              )}
              {hasKey || currentAuthType === 'api_key' ? 'Update' : 'Save'}
            </Button>
          </div>
          {isSuccess && (
            <p className="mt-2 text-sm text-green-600 dark:text-green-400">
              API key saved successfully!
            </p>
          )}
          <p className="mt-3 text-xs text-muted-foreground">
            Get your API key from{' '}
            <a
              href="https://aistudio.google.com/app/apikey"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary-500 hover:underline"
            >
              Google AI Studio
            </a>
          </p>
        </div>

        {/* Divider */}
        <div className="relative">
          <div className="absolute inset-0 flex items-center">
            <div className="w-full border-t border-border" />
          </div>
          <div className="relative flex justify-center text-xs uppercase">
            <span className="bg-background px-2 text-muted-foreground">or</span>
          </div>
        </div>

        {/* Vertex AI Service Account Section */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <FileText className="h-4 w-4 text-muted-foreground" />
              <h4 className="font-medium text-foreground">Google Cloud Vertex AI</h4>
            </div>
            {currentAuthType === 'service_account' && (
              <div className="flex items-center gap-1.5">
                <div className="h-2 w-2 rounded-full bg-green-500" />
                <span className="text-xs text-green-600 dark:text-green-400">Active</span>
              </div>
            )}
          </div>
          <p className="text-sm text-muted-foreground mb-4">
            Use a service account JSON for Vertex AI. Best for enterprise and production use.
          </p>

          <div className="space-y-3">
            <div className="flex gap-2 items-center">
              <label className="flex-1">
                <input
                  type="file"
                  accept=".json"
                  onChange={handleFileUpload}
                  className="hidden"
                />
                <div className="flex items-center gap-2 px-4 py-2 border border-border rounded-lg cursor-pointer hover:bg-muted/50 transition-colors">
                  <FileText className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm text-foreground">
                    {jsonFileName || 'Choose service account JSON file...'}
                  </span>
                </div>
              </label>
            </div>
            <div className="flex gap-2 items-end">
              <div className="flex-1">
                <label className="block text-xs font-medium text-muted-foreground mb-1">
                  Vertex AI Region
                </label>
                <Input
                  type="text"
                  value={vertexRegion}
                  onChange={e => setVertexRegion(e.target.value)}
                  placeholder="global"
                  className="w-full"
                />
              </div>
              <Button
                onClick={handleSaveCredentials}
                disabled={isSavingCredentials || !jsonContent}
              >
                {isSavingCredentials ? (
                  <RefreshCw className="h-4 w-4 animate-spin" />
                ) : saveCredentialsSuccess ? (
                  <Check className="h-4 w-4" />
                ) : (
                  <Save className="h-4 w-4" />
                )}
                {currentAuthType === 'service_account' ? 'Update' : 'Save'}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Use "global" for the non-regional endpoint, or specify a region (us-central1, europe-west1, etc.)
            </p>
          </div>
          {saveCredentialsSuccess && (
            <p className="mt-2 text-sm text-green-600 dark:text-green-400">
              Service account credentials saved successfully!
            </p>
          )}
          <p className="mt-3 text-xs text-muted-foreground">
            Download your service account JSON from the{' '}
            <a
              href="https://console.cloud.google.com/iam-admin/serviceaccounts"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary-500 hover:underline"
            >
              Google Cloud Console
            </a>
          </p>
        </div>

        {/* Verify Connection */}
        {isConfigured && (
          <div className="p-4 bg-card border border-border rounded-lg">
            <div className="flex items-center justify-between">
              <div>
                <h4 className="font-medium text-foreground">Test Connection</h4>
                <p className="text-sm text-muted-foreground">
                  Verify that your credentials are working correctly.
                </p>
              </div>
              <Button
                variant="outline"
                onClick={handleVerifyConnection}
                disabled={isVerifying}
              >
                {isVerifying ? (
                  <RefreshCw className="h-4 w-4 animate-spin" />
                ) : (
                  <Check className="h-4 w-4" />
                )}
                Verify
              </Button>
            </div>

            {verifyResult && (
              <div className={clsx(
                'mt-3 p-3 rounded-lg text-sm',
                verifyResult.success
                  ? 'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 text-green-600 dark:text-green-400'
                  : 'bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-600 dark:text-red-400'
              )}>
                {verifyResult.message}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
