import { describe, it, expect, beforeEach } from 'vitest'
import { useAppStore, type SlackState } from './store'

const initialSlackState: SlackState = {
  connected: false,
  teamId: null,
  teamName: null,
  userId: null,
  selectedChannelCount: 0,
}

describe('useAppStore', () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useAppStore.setState({
      currentView: 'daily-digest',
      settingsSection: 'sources',
      slack: initialSlackState,
      showChannelSelector: false,
      localActivities: [],
      hasUnseenActivity: false,
    })
  })

  describe('initial state', () => {
    it('has daily-digest as default view', () => {
      expect(useAppStore.getState().currentView).toBe('daily-digest')
    })

    it('has sources as default settings section', () => {
      expect(useAppStore.getState().settingsSection).toBe('sources')
    })
  })

  describe('setView', () => {
    it('updates currentView to daily-digest', () => {
      useAppStore.getState().setView('daily-digest')
      expect(useAppStore.getState().currentView).toBe('daily-digest')
    })

    it('updates currentView to weekly-summary', () => {
      useAppStore.getState().setView('weekly-summary')
      expect(useAppStore.getState().currentView).toBe('weekly-summary')
    })

    it('updates currentView to settings', () => {
      useAppStore.getState().setView('settings')
      expect(useAppStore.getState().currentView).toBe('settings')
    })
  })

  describe('setSettingsSection', () => {
    it('updates settingsSection to sources', () => {
      useAppStore.getState().setSettingsSection('sources')
      expect(useAppStore.getState().settingsSection).toBe('sources')
    })

    it('updates settingsSection to api-keys', () => {
      useAppStore.getState().setSettingsSection('api-keys')
      expect(useAppStore.getState().settingsSection).toBe('api-keys')
    })

    it('updates settingsSection to notifications', () => {
      useAppStore.getState().setSettingsSection('notifications')
      expect(useAppStore.getState().settingsSection).toBe('notifications')
    })

    it('updates settingsSection to sync', () => {
      useAppStore.getState().setSettingsSection('sync')
      expect(useAppStore.getState().settingsSection).toBe('sync')
    })

    it('updates settingsSection to appearance', () => {
      useAppStore.getState().setSettingsSection('appearance')
      expect(useAppStore.getState().settingsSection).toBe('appearance')
    })

    it('updates settingsSection to data', () => {
      useAppStore.getState().setSettingsSection('data')
      expect(useAppStore.getState().settingsSection).toBe('data')
    })

    it('updates settingsSection to status', () => {
      useAppStore.getState().setSettingsSection('status')
      expect(useAppStore.getState().settingsSection).toBe('status')
    })

    it('updates settingsSection to about', () => {
      useAppStore.getState().setSettingsSection('about')
      expect(useAppStore.getState().settingsSection).toBe('about')
    })
  })

  describe('state independence', () => {
    it('setView does not affect settingsSection', () => {
      useAppStore.getState().setSettingsSection('appearance')
      useAppStore.getState().setView('settings')
      expect(useAppStore.getState().settingsSection).toBe('appearance')
    })

    it('setSettingsSection does not affect currentView', () => {
      useAppStore.getState().setView('weekly-summary')
      useAppStore.getState().setSettingsSection('notifications')
      expect(useAppStore.getState().currentView).toBe('weekly-summary')
    })
  })

  describe('slack state', () => {
    describe('initial state', () => {
      it('has disconnected slack by default', () => {
        expect(useAppStore.getState().slack.connected).toBe(false)
      })

      it('has null team and user info by default', () => {
        const { slack } = useAppStore.getState()
        expect(slack.teamId).toBeNull()
        expect(slack.teamName).toBeNull()
        expect(slack.userId).toBeNull()
      })

      it('has zero selected channels by default', () => {
        expect(useAppStore.getState().slack.selectedChannelCount).toBe(0)
      })

      it('has channel selector hidden by default', () => {
        expect(useAppStore.getState().showChannelSelector).toBe(false)
      })
    })

    describe('setSlackState', () => {
      it('updates connected status', () => {
        useAppStore.getState().setSlackState({ connected: true })
        expect(useAppStore.getState().slack.connected).toBe(true)
      })

      it('updates team info partially', () => {
        useAppStore.getState().setSlackState({ 
          teamId: 'T123',
          teamName: 'Test Workspace' 
        })
        const { slack } = useAppStore.getState()
        expect(slack.teamId).toBe('T123')
        expect(slack.teamName).toBe('Test Workspace')
        expect(slack.userId).toBeNull() // unchanged
      })

      it('updates full connection state', () => {
        useAppStore.getState().setSlackState({
          connected: true,
          teamId: 'T123',
          teamName: 'Test Workspace',
          userId: 'U456',
          selectedChannelCount: 5,
        })
        const { slack } = useAppStore.getState()
        expect(slack.connected).toBe(true)
        expect(slack.teamId).toBe('T123')
        expect(slack.teamName).toBe('Test Workspace')
        expect(slack.userId).toBe('U456')
        expect(slack.selectedChannelCount).toBe(5)
      })

      it('preserves other slack fields when updating one', () => {
        useAppStore.getState().setSlackState({
          connected: true,
          teamId: 'T123',
          teamName: 'Workspace',
          userId: 'U456',
          selectedChannelCount: 3,
        })
        useAppStore.getState().setSlackState({ selectedChannelCount: 10 })
        const { slack } = useAppStore.getState()
        expect(slack.connected).toBe(true)
        expect(slack.teamId).toBe('T123')
        expect(slack.selectedChannelCount).toBe(10)
      })
    })

    describe('setShowChannelSelector', () => {
      it('shows channel selector', () => {
        useAppStore.getState().setShowChannelSelector(true)
        expect(useAppStore.getState().showChannelSelector).toBe(true)
      })

      it('hides channel selector', () => {
        useAppStore.getState().setShowChannelSelector(true)
        useAppStore.getState().setShowChannelSelector(false)
        expect(useAppStore.getState().showChannelSelector).toBe(false)
      })
    })

    describe('resetSlackState', () => {
      it('resets all slack state to initial values', () => {
        useAppStore.getState().setSlackState({
          connected: true,
          teamId: 'T123',
          teamName: 'Workspace',
          userId: 'U456',
          selectedChannelCount: 5,
        })
        useAppStore.getState().setShowChannelSelector(true)
        
        useAppStore.getState().resetSlackState()
        
        const { slack, showChannelSelector } = useAppStore.getState()
        expect(slack.connected).toBe(false)
        expect(slack.teamId).toBeNull()
        expect(slack.teamName).toBeNull()
        expect(slack.userId).toBeNull()
        expect(slack.selectedChannelCount).toBe(0)
        expect(showChannelSelector).toBe(false)
      })

      it('does not affect other app state', () => {
        useAppStore.getState().setView('settings')
        useAppStore.getState().setSettingsSection('api-keys')
        useAppStore.getState().setSlackState({ connected: true })
        
        useAppStore.getState().resetSlackState()
        
        expect(useAppStore.getState().currentView).toBe('settings')
        expect(useAppStore.getState().settingsSection).toBe('api-keys')
      })
    })

    describe('slack state independence', () => {
      it('setView does not affect slack state', () => {
        useAppStore.getState().setSlackState({ connected: true, teamId: 'T123' })
        useAppStore.getState().setView('weekly-summary')
        expect(useAppStore.getState().slack.connected).toBe(true)
        expect(useAppStore.getState().slack.teamId).toBe('T123')
      })

      it('setSlackState does not affect view state', () => {
        useAppStore.getState().setView('settings')
        useAppStore.getState().setSlackState({ connected: true })
        expect(useAppStore.getState().currentView).toBe('settings')
      })
    })
  })

  describe('local activities', () => {
    describe('initial state', () => {
      it('has empty localActivities by default', () => {
        expect(useAppStore.getState().localActivities).toEqual([])
      })

      it('has hasUnseenActivity false by default', () => {
        expect(useAppStore.getState().hasUnseenActivity).toBe(false)
      })
    })

    describe('addLocalActivity', () => {
      it('adds activity to the list', () => {
        useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Test export',
          status: 'running',
        })
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities).toHaveLength(1)
        expect(localActivities[0].type).toBe('pdf_export')
        expect(localActivities[0].message).toBe('Test export')
        expect(localActivities[0].status).toBe('running')
      })

      it('supports historical_resync activity type', () => {
        useAppStore.getState().addLocalActivity({
          type: 'historical_resync',
          message: 'Resyncing January 25, 2026...',
          status: 'running',
        })
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities).toHaveLength(1)
        expect(localActivities[0].type).toBe('historical_resync')
        expect(localActivities[0].message).toBe('Resyncing January 25, 2026...')
      })

      it('generates unique ID for each activity', () => {
        const id1 = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Export 1',
          status: 'running',
        })
        const id2 = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Export 2',
          status: 'running',
        })
        
        expect(id1).not.toBe(id2)
        expect(id1).toMatch(/^local-\d+-\d+$/)
        expect(id2).toMatch(/^local-\d+-\d+$/)
      })

      it('sets startedAt timestamp', () => {
        const before = Date.now()
        useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Test',
          status: 'running',
        })
        const after = Date.now()
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities[0].startedAt).toBeGreaterThanOrEqual(before)
        expect(localActivities[0].startedAt).toBeLessThanOrEqual(after)
      })

      it('sets completedAt to null and error to null', () => {
        useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Test',
          status: 'running',
        })
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities[0].completedAt).toBeNull()
        expect(localActivities[0].error).toBeNull()
      })

      it('sets hasUnseenActivity to true', () => {
        useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Test',
          status: 'running',
        })
        
        expect(useAppStore.getState().hasUnseenActivity).toBe(true)
      })

      it('prepends new activities to the list', () => {
        useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'First',
          status: 'running',
        })
        useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Second',
          status: 'running',
        })
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities[0].message).toBe('Second')
        expect(localActivities[1].message).toBe('First')
      })
    })

    describe('updateLocalActivity', () => {
      it('updates the correct activity by ID', () => {
        const id = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Original',
          status: 'running',
        })
        
        useAppStore.getState().updateLocalActivity(id, { message: 'Updated' })
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities[0].message).toBe('Updated')
      })

      it('sets completedAt when status changes to completed', () => {
        const id = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Test',
          status: 'running',
        })
        
        const before = Date.now()
        useAppStore.getState().updateLocalActivity(id, { status: 'completed' })
        const after = Date.now()
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities[0].completedAt).toBeGreaterThanOrEqual(before)
        expect(localActivities[0].completedAt).toBeLessThanOrEqual(after)
      })

      it('sets completedAt when status changes to failed', () => {
        const id = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Test',
          status: 'running',
        })
        
        useAppStore.getState().updateLocalActivity(id, { 
          status: 'failed',
          error: 'Something went wrong',
        })
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities[0].completedAt).not.toBeNull()
        expect(localActivities[0].error).toBe('Something went wrong')
      })

      it('does not change completedAt if status is not completed or failed', () => {
        const id = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Test',
          status: 'running',
        })
        
        useAppStore.getState().updateLocalActivity(id, { message: 'Still running' })
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities[0].completedAt).toBeNull()
      })

      it('sets hasUnseenActivity to true', () => {
        const id = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Test',
          status: 'running',
        })
        useAppStore.getState().markActivitySeen()
        
        useAppStore.getState().updateLocalActivity(id, { status: 'completed' })
        
        expect(useAppStore.getState().hasUnseenActivity).toBe(true)
      })

      it('does not affect other activities', () => {
        const id1 = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'First',
          status: 'running',
        })
        useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Second',
          status: 'running',
        })
        
        useAppStore.getState().updateLocalActivity(id1, { status: 'completed' })
        
        const { localActivities } = useAppStore.getState()
        expect(localActivities[1].status).toBe('completed')
        expect(localActivities[0].status).toBe('running')
      })
    })

    describe('clearOldLocalActivities', () => {
      it('removes completed activities older than 5 minutes', () => {
        const id = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Old',
          status: 'running',
        })
        
        // Manually set completedAt to 6 minutes ago
        const sixMinutesAgo = Date.now() - 6 * 60 * 1000
        useAppStore.setState((prev) => ({
          localActivities: prev.localActivities.map((a) =>
            a.id === id ? { ...a, status: 'completed' as const, completedAt: sixMinutesAgo } : a
          ),
        }))
        
        useAppStore.getState().clearOldLocalActivities()
        
        expect(useAppStore.getState().localActivities).toHaveLength(0)
      })

      it('keeps running activities regardless of age', () => {
        useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Running',
          status: 'running',
        })
        
        // Even if startedAt was long ago, running activities are kept
        useAppStore.getState().clearOldLocalActivities()
        
        expect(useAppStore.getState().localActivities).toHaveLength(1)
      })

      it('keeps recent completed activities', () => {
        const id = useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Recent',
          status: 'running',
        })
        useAppStore.getState().updateLocalActivity(id, { status: 'completed' })
        
        useAppStore.getState().clearOldLocalActivities()
        
        expect(useAppStore.getState().localActivities).toHaveLength(1)
      })
    })

    describe('markActivitySeen', () => {
      it('sets hasUnseenActivity to false', () => {
        useAppStore.getState().addLocalActivity({
          type: 'pdf_export',
          message: 'Test',
          status: 'running',
        })
        expect(useAppStore.getState().hasUnseenActivity).toBe(true)
        
        useAppStore.getState().markActivitySeen()
        
        expect(useAppStore.getState().hasUnseenActivity).toBe(false)
      })
    })
  })
})
