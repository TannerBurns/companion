import { describe, it, expect, beforeEach } from 'vitest'
import { useAppStore } from './store'

describe('useAppStore', () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useAppStore.setState({
      currentView: 'daily-digest',
      settingsSection: 'sources',
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
})
