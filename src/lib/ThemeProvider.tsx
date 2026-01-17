import type { ReactNode } from 'react'
import { useSyncExternalStore, useCallback, useMemo } from 'react'
import { ThemeContext } from './themeContext'

type Theme = 'light' | 'dark' | 'system'

function getSystemTheme(): 'light' | 'dark' {
  if (typeof window === 'undefined') return 'light'
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function getStoredTheme(): Theme {
  if (typeof window === 'undefined') return 'system'
  return (localStorage.getItem('theme') as Theme) || 'system'
}

function resolveTheme(theme: Theme): 'light' | 'dark' {
  return theme === 'system' ? getSystemTheme() : theme
}

function applyThemeToDOM(resolved: 'light' | 'dark') {
  const root = document.documentElement
  if (resolved === 'dark') {
    root.classList.add('dark')
  } else {
    root.classList.remove('dark')
  }
}

let currentTheme: Theme = getStoredTheme()
const listeners = new Set<() => void>()

function subscribe(callback: () => void) {
  listeners.add(callback)
  return () => listeners.delete(callback)
}

function getSnapshot() {
  return currentTheme
}

function setThemeExternal(theme: Theme) {
  currentTheme = theme
  localStorage.setItem('theme', theme)
  applyThemeToDOM(resolveTheme(theme))
  listeners.forEach(l => l())
}

if (typeof window !== 'undefined') {
  applyThemeToDOM(resolveTheme(currentTheme))
  
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (currentTheme === 'system') {
      applyThemeToDOM(getSystemTheme())
      listeners.forEach(l => l())
    }
  })
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const theme = useSyncExternalStore(subscribe, getSnapshot, () => 'system' as Theme)

  const setTheme = useCallback((newTheme: Theme) => {
    setThemeExternal(newTheme)
  }, [])

  const resolvedTheme = resolveTheme(theme)

  const value = useMemo(
    () => ({ theme, setTheme, resolvedTheme }),
    [theme, setTheme, resolvedTheme]
  )

  return (
    <ThemeContext.Provider value={value}>
      {children}
    </ThemeContext.Provider>
  )
}
