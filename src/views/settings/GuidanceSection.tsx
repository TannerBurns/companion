import { useState } from 'react'
import { MessageSquare } from 'lucide-react'
import { Button } from '../../components/ui/Button'
import { usePreferences } from '../../hooks/usePreferences'

export function GuidanceSection() {
  const { preferences, save, isSaving } = usePreferences()
  const preferenceGuidance = preferences.userGuidance ?? ''

  // Track user edits separately from preference value
  const [editedGuidance, setEditedGuidance] = useState<string | null>(null)

  // Use preference value when not editing, edited value when actively editing
  const localGuidance = editedGuidance ?? preferenceGuidance
  const hasChanges = editedGuidance !== null && editedGuidance !== preferenceGuidance

  const handleChange = (value: string) => {
    setEditedGuidance(value)
  }

  const handleSave = () => {
    const guidance = localGuidance.trim() || undefined
    save({ ...preferences, userGuidance: guidance })
    setEditedGuidance(null)
  }

  const handleCancel = () => {
    setEditedGuidance(null)
  }

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">AI Guidance</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Customize how the AI summarizes and prioritizes your information.
        </p>
      </div>

      <div className="space-y-4">
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-start gap-3 mb-3">
            <MessageSquare className="h-5 w-5 text-muted-foreground mt-0.5" />
            <div>
              <h4 className="font-medium text-foreground">Your Preferences</h4>
              <p className="text-sm text-muted-foreground">
                Tell the AI what matters most to you. This guidance will be used when
                creating summaries and prioritizing information.
              </p>
            </div>
          </div>
          <textarea
            value={localGuidance}
            onChange={e => handleChange(e.target.value)}
            placeholder="e.g., Focus on production issues and incidents. Highlight any mentions of API changes or breaking changes. Prioritize discussions about quarterly goals."
            rows={4}
            className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary-500 resize-none"
          />
          <div className="mt-2 text-xs text-muted-foreground">
            Examples: "Only summarize production-related discussions", "Focus on mentions
            of user X", "Prioritize engineering updates over sales"
          </div>
        </div>

        {hasChanges && (
          <div className="flex items-center gap-2">
            <Button
              onClick={handleSave}
              disabled={isSaving}
              size="sm"
            >
              {isSaving ? 'Saving...' : 'Save Changes'}
            </Button>
            <Button
              variant="outline"
              onClick={handleCancel}
              disabled={isSaving}
              size="sm"
            >
              Cancel
            </Button>
          </div>
        )}
      </div>
    </div>
  )
}
