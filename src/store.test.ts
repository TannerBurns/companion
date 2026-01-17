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
})
