# Phase 4: Frontend Development

This guide covers building the React frontend with TailwindCSS, including the design system, daily/weekly digest views, drill-down flow, and settings UI.

## Overview

By the end of this phase, you will have:
- TailwindCSS design system with light/dark mode
- Core reusable components
- Daily and weekly digest views
- Drill-down to full analysis and source
- Settings configuration UI

---

## 4.1 Design System Setup

### Core Components Directory Structure

```
src/
├── components/
│   ├── ui/
│   │   ├── Button.tsx
│   │   ├── Card.tsx
│   │   ├── Badge.tsx
│   │   └── Input.tsx
│   ├── ContentCard.tsx
│   ├── CategoryBadge.tsx
│   ├── ImportanceIndicator.tsx
│   ├── SourceIcon.tsx
│   └── DigestSection.tsx
├── views/
│   ├── DailyDigest.tsx
│   ├── WeeklyDigest.tsx
│   ├── ItemDetail.tsx
│   └── Settings.tsx
├── hooks/
│   ├── useDigest.ts
│   ├── useSync.ts
│   └── usePreferences.ts
├── lib/
│   ├── api.ts
│   └── utils.ts
└── App.tsx
```

### Theme Provider

Create `src/lib/theme.tsx`:

```tsx
import { createContext, useContext, useEffect, useState, ReactNode } from 'react';

type Theme = 'light' | 'dark' | 'system';

const ThemeContext = createContext<{
  theme: Theme;
  setTheme: (theme: Theme) => void;
}>({ theme: 'system', setTheme: () => {} });

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<Theme>(() => {
    const stored = localStorage.getItem('theme') as Theme;
    return stored || 'system';
  });

  useEffect(() => {
    const root = document.documentElement;
    const systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    
    if (theme === 'dark' || (theme === 'system' && systemDark)) {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
    
    localStorage.setItem('theme', theme);
  }, [theme]);

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export const useTheme = () => useContext(ThemeContext);
```

---

## 4.2 Core UI Components

Create `src/components/ui/Card.tsx`:

```tsx
import { ReactNode } from 'react';
import { clsx } from 'clsx';

interface CardProps {
  children: ReactNode;
  className?: string;
  onClick?: () => void;
  hoverable?: boolean;
}

export function Card({ children, className, onClick, hoverable }: CardProps) {
  return (
    <div
      className={clsx(
        'rounded-lg border border-border bg-card p-4 shadow-sm',
        hoverable && 'cursor-pointer transition-shadow hover:shadow-md',
        className
      )}
      onClick={onClick}
    >
      {children}
    </div>
  );
}
```

Create `src/components/CategoryBadge.tsx`:

```tsx
import { clsx } from 'clsx';

const categoryColors: Record<string, string> = {
  sales: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
  marketing: 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200',
  product: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
  engineering: 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200',
  research: 'bg-cyan-100 text-cyan-800 dark:bg-cyan-900 dark:text-cyan-200',
  other: 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200',
};

interface CategoryBadgeProps {
  category: string;
  confidence?: number;
}

export function CategoryBadge({ category, confidence }: CategoryBadgeProps) {
  const colorClass = categoryColors[category.toLowerCase()] || categoryColors.other;
  
  return (
    <span className={clsx('inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-medium', colorClass)}>
      {category}
      {confidence !== undefined && confidence < 0.7 && (
        <span className="opacity-60">?</span>
      )}
    </span>
  );
}
```

Create `src/components/SourceIcon.tsx`:

```tsx
import { MessageSquare, FileText, CheckSquare } from 'lucide-react';

interface SourceIconProps {
  source: 'slack' | 'jira' | 'confluence';
  className?: string;
}

export function SourceIcon({ source, className = 'h-4 w-4' }: SourceIconProps) {
  switch (source) {
    case 'slack':
      return <MessageSquare className={className} />;
    case 'jira':
      return <CheckSquare className={className} />;
    case 'confluence':
      return <FileText className={className} />;
  }
}
```

Create `src/components/ImportanceIndicator.tsx`:

```tsx
import { clsx } from 'clsx';

interface ImportanceIndicatorProps {
  score: number;
}

export function ImportanceIndicator({ score }: ImportanceIndicatorProps) {
  const level = score >= 0.8 ? 'high' : score >= 0.5 ? 'medium' : 'low';
  
  return (
    <div className="flex items-center gap-1">
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className={clsx(
            'h-1.5 w-1.5 rounded-full',
            i === 0 && 'bg-current',
            i === 1 && (level !== 'low' ? 'bg-current' : 'bg-gray-300'),
            i === 2 && (level === 'high' ? 'bg-current' : 'bg-gray-300'),
            level === 'high' && 'text-red-500',
            level === 'medium' && 'text-yellow-500',
            level === 'low' && 'text-gray-400'
          )}
        />
      ))}
    </div>
  );
}
```

Create `src/components/ContentCard.tsx`:

```tsx
import { useState } from 'react';
import { ExternalLink, ChevronDown, ChevronUp } from 'lucide-react';
import { Card } from './ui/Card';
import { CategoryBadge } from './CategoryBadge';
import { SourceIcon } from './SourceIcon';
import { ImportanceIndicator } from './ImportanceIndicator';

interface ContentCardProps {
  item: {
    id: string;
    title: string;
    summary: string;
    highlights?: string[];
    category: string;
    categoryConfidence?: number;
    source: 'slack' | 'jira' | 'confluence';
    sourceUrl?: string;
    importanceScore: number;
    createdAt: number;
  };
  onViewDetail?: (id: string) => void;
}

export function ContentCard({ item, onViewDetail }: ContentCardProps) {
  const [expanded, setExpanded] = useState(false);
  
  return (
    <Card hoverable className="group">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2 text-muted-foreground">
          <SourceIcon source={item.source} />
          <span className="text-xs">
            {new Date(item.createdAt).toLocaleDateString()}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <ImportanceIndicator score={item.importanceScore} />
          <CategoryBadge category={item.category} confidence={item.categoryConfidence} />
        </div>
      </div>
      
      <h3 className="mt-2 font-medium text-foreground line-clamp-2">
        {item.title || item.summary.slice(0, 60)}
      </h3>
      
      <p className="mt-1 text-sm text-muted-foreground line-clamp-2">
        {item.summary}
      </p>
      
      {expanded && item.highlights && (
        <ul className="mt-3 space-y-1">
          {item.highlights.map((h, i) => (
            <li key={i} className="flex items-start gap-2 text-sm text-muted-foreground">
              <span className="text-primary">•</span>
              {h}
            </li>
          ))}
        </ul>
      )}
      
      <div className="mt-3 flex items-center justify-between">
        <button
          onClick={() => setExpanded(!expanded)}
          className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
        >
          {expanded ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
          {expanded ? 'Less' : 'More'}
        </button>
        
        <div className="flex items-center gap-2">
          {item.sourceUrl && (
            <a
              href={item.sourceUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 text-xs text-primary hover:underline"
              onClick={(e) => e.stopPropagation()}
            >
              <ExternalLink className="h-3 w-3" />
              View Source
            </a>
          )}
        </div>
      </div>
    </Card>
  );
}
```

---

## 4.3 API Hooks

Create `src/lib/api.ts`:

```tsx
import { invoke } from '@tauri-apps/api/core';

export interface DigestItem {
  id: string;
  title: string;
  summary: string;
  highlights?: string[];
  category: string;
  categoryConfidence?: number;
  source: 'slack' | 'jira' | 'confluence';
  sourceUrl?: string;
  importanceScore: number;
  createdAt: number;
}

export interface DigestResponse {
  date: string;
  items: DigestItem[];
  categories: CategorySummary[];
}

export interface CategorySummary {
  name: string;
  count: number;
  topItems: DigestItem[];
}

export interface SyncStatus {
  isSyncing: boolean;
  lastSyncAt?: number;
  sources: SourceStatus[];
}

export interface SourceStatus {
  name: string;
  status: string;
  itemsSynced: number;
  lastError?: string;
}

export interface Preferences {
  syncIntervalMinutes: number;
  enabledSources: string[];
  enabledCategories: string[];
  notificationsEnabled: boolean;
}

export const api = {
  getDailyDigest: (date?: string) => 
    invoke<DigestResponse>('get_daily_digest', { date }),
  
  getWeeklyDigest: (weekStart?: string) => 
    invoke<DigestResponse>('get_weekly_digest', { weekStart }),
  
  startSync: (sources?: string[]) => 
    invoke<void>('start_sync', { sources }),
  
  getSyncStatus: () => 
    invoke<SyncStatus>('get_sync_status'),
  
  getPreferences: () => 
    invoke<Preferences>('get_preferences'),
  
  savePreferences: (preferences: Preferences) => 
    invoke<void>('save_preferences', { preferences }),
  
  saveApiKey: (service: string, apiKey: string) => 
    invoke<void>('save_api_key', { service, apiKey }),
  
  connectSlack: (clientId: string, clientSecret: string) =>
    invoke<any>('connect_slack', { clientId, clientSecret }),
  
  connectAtlassian: (clientId: string, clientSecret: string) =>
    invoke<any>('connect_atlassian', { clientId, clientSecret }),
};
```

Create `src/hooks/useDigest.ts`:

```tsx
import { useQuery } from '@tanstack/react-query';
import { api } from '../lib/api';

export function useDailyDigest(date?: string) {
  return useQuery({
    queryKey: ['daily-digest', date],
    queryFn: () => api.getDailyDigest(date),
    staleTime: 1000 * 60 * 5, // 5 minutes
  });
}

export function useWeeklyDigest(weekStart?: string) {
  return useQuery({
    queryKey: ['weekly-digest', weekStart],
    queryFn: () => api.getWeeklyDigest(weekStart),
    staleTime: 1000 * 60 * 15, // 15 minutes
  });
}
```

---

## 4.4 Digest Views

Create `src/views/DailyDigest.tsx`:

```tsx
import { useState } from 'react';
import { format, subDays, addDays } from 'date-fns';
import { ChevronLeft, ChevronRight, RefreshCw } from 'lucide-react';
import { useDailyDigest } from '../hooks/useDigest';
import { ContentCard } from '../components/ContentCard';
import { CategoryBadge } from '../components/CategoryBadge';
import { api } from '../lib/api';

const CATEGORIES = ['all', 'engineering', 'product', 'sales', 'marketing', 'research', 'other'];

export function DailyDigest() {
  const [date, setDate] = useState(new Date());
  const [filter, setFilter] = useState('all');
  const [syncing, setSyncing] = useState(false);
  
  const dateStr = format(date, 'yyyy-MM-dd');
  const { data, isLoading, refetch } = useDailyDigest(dateStr);
  
  const filteredItems = data?.items.filter(
    item => filter === 'all' || item.category.toLowerCase() === filter
  ) ?? [];
  
  const handleSync = async () => {
    setSyncing(true);
    try {
      await api.startSync();
      await refetch();
    } finally {
      setSyncing(false);
    }
  };
  
  return (
    <div className="mx-auto max-w-4xl p-6">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <button
            onClick={() => setDate(d => subDays(d, 1))}
            className="rounded-lg p-2 hover:bg-muted"
          >
            <ChevronLeft className="h-5 w-5" />
          </button>
          
          <h1 className="text-2xl font-semibold">
            Daily Digest - {format(date, 'MMMM d, yyyy')}
          </h1>
          
          <button
            onClick={() => setDate(d => addDays(d, 1))}
            className="rounded-lg p-2 hover:bg-muted"
            disabled={format(date, 'yyyy-MM-dd') === format(new Date(), 'yyyy-MM-dd')}
          >
            <ChevronRight className="h-5 w-5" />
          </button>
        </div>
        
        <button
          onClick={handleSync}
          disabled={syncing}
          className="flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-white hover:bg-primary/90 disabled:opacity-50"
        >
          <RefreshCw className={`h-4 w-4 ${syncing ? 'animate-spin' : ''}`} />
          Sync
        </button>
      </div>
      
      {/* Category Filter */}
      <div className="mb-6 flex flex-wrap gap-2">
        {CATEGORIES.map(cat => (
          <button
            key={cat}
            onClick={() => setFilter(cat)}
            className={`rounded-full px-3 py-1 text-sm font-medium transition-colors ${
              filter === cat
                ? 'bg-primary text-white'
                : 'bg-muted text-muted-foreground hover:bg-muted/80'
            }`}
          >
            {cat.charAt(0).toUpperCase() + cat.slice(1)}
            {cat !== 'all' && data?.categories.find(c => c.name.toLowerCase() === cat) && (
              <span className="ml-1 opacity-70">
                ({data.categories.find(c => c.name.toLowerCase() === cat)?.count})
              </span>
            )}
          </button>
        ))}
      </div>
      
      {/* Content */}
      {isLoading ? (
        <div className="flex h-64 items-center justify-center">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        </div>
      ) : filteredItems.length === 0 ? (
        <div className="flex h-64 flex-col items-center justify-center text-muted-foreground">
          <p>No items for this day</p>
          <button
            onClick={handleSync}
            className="mt-2 text-primary hover:underline"
          >
            Sync now
          </button>
        </div>
      ) : (
        <div className="grid gap-4">
          {filteredItems.map(item => (
            <ContentCard key={item.id} item={item} />
          ))}
        </div>
      )}
    </div>
  );
}
```

Create `src/views/WeeklyDigest.tsx`:

```tsx
import { useState } from 'react';
import { format, startOfWeek, subWeeks, addWeeks } from 'date-fns';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { useWeeklyDigest } from '../hooks/useDigest';
import { ContentCard } from '../components/ContentCard';

export function WeeklyDigest() {
  const [weekStart, setWeekStart] = useState(() => startOfWeek(new Date(), { weekStartsOn: 1 }));
  
  const weekStartStr = format(weekStart, 'yyyy-MM-dd');
  const { data, isLoading } = useWeeklyDigest(weekStartStr);
  
  return (
    <div className="mx-auto max-w-4xl p-6">
      <div className="mb-6 flex items-center gap-4">
        <button
          onClick={() => setWeekStart(d => subWeeks(d, 1))}
          className="rounded-lg p-2 hover:bg-muted"
        >
          <ChevronLeft className="h-5 w-5" />
        </button>
        
        <h1 className="text-2xl font-semibold">
          Week of {format(weekStart, 'MMMM d, yyyy')}
        </h1>
        
        <button
          onClick={() => setWeekStart(d => addWeeks(d, 1))}
          className="rounded-lg p-2 hover:bg-muted"
        >
          <ChevronRight className="h-5 w-5" />
        </button>
      </div>
      
      {isLoading ? (
        <div className="flex h-64 items-center justify-center">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        </div>
      ) : (
        <>
          {/* Category breakdown */}
          <div className="mb-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {data?.categories.map(cat => (
              <div key={cat.name} className="rounded-lg border border-border bg-card p-4">
                <div className="flex items-center justify-between">
                  <span className="font-medium capitalize">{cat.name}</span>
                  <span className="text-2xl font-bold text-primary">{cat.count}</span>
                </div>
              </div>
            ))}
          </div>
          
          {/* Top items */}
          <h2 className="mb-4 text-lg font-semibold">Top Items This Week</h2>
          <div className="grid gap-4">
            {data?.items.slice(0, 10).map(item => (
              <ContentCard key={item.id} item={item} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}
```

---

## 4.5 Settings View

Create `src/views/Settings.tsx`:

```tsx
import { useState, useEffect } from 'react';
import { Save, Key, RefreshCw } from 'lucide-react';
import { api, Preferences } from '../lib/api';
import { useTheme } from '../lib/theme';

export function Settings() {
  const { theme, setTheme } = useTheme();
  const [prefs, setPrefs] = useState<Preferences | null>(null);
  const [saving, setSaving] = useState(false);
  const [apiKeys, setApiKeys] = useState({ slack: '', atlassian: '', gemini: '' });
  
  useEffect(() => {
    api.getPreferences().then(setPrefs);
  }, []);
  
  const handleSave = async () => {
    if (!prefs) return;
    setSaving(true);
    try {
      await api.savePreferences(prefs);
      
      if (apiKeys.gemini) {
        await api.saveApiKey('gemini', apiKeys.gemini);
        setApiKeys(k => ({ ...k, gemini: '' }));
      }
    } finally {
      setSaving(false);
    }
  };
  
  if (!prefs) return <div className="p-6">Loading...</div>;
  
  return (
    <div className="mx-auto max-w-2xl p-6">
      <h1 className="mb-6 text-2xl font-semibold">Settings</h1>
      
      {/* Theme */}
      <section className="mb-8">
        <h2 className="mb-4 text-lg font-medium">Appearance</h2>
        <div className="flex gap-2">
          {(['light', 'dark', 'system'] as const).map(t => (
            <button
              key={t}
              onClick={() => setTheme(t)}
              className={`rounded-lg px-4 py-2 text-sm font-medium ${
                theme === t ? 'bg-primary text-white' : 'bg-muted'
              }`}
            >
              {t.charAt(0).toUpperCase() + t.slice(1)}
            </button>
          ))}
        </div>
      </section>
      
      {/* API Keys */}
      <section className="mb-8">
        <h2 className="mb-4 text-lg font-medium">API Keys</h2>
        
        <div className="space-y-4">
          <div>
            <label className="mb-1 block text-sm font-medium">Gemini API Key</label>
            <div className="flex gap-2">
              <input
                type="password"
                value={apiKeys.gemini}
                onChange={e => setApiKeys(k => ({ ...k, gemini: e.target.value }))}
                placeholder="Enter your Gemini API key"
                className="flex-1 rounded-lg border border-border bg-background px-3 py-2 text-sm"
              />
              <button className="rounded-lg bg-muted p-2 hover:bg-muted/80">
                <Key className="h-4 w-4" />
              </button>
            </div>
          </div>
          
          <div className="rounded-lg border border-border p-4">
            <h3 className="mb-2 font-medium">Connected Services</h3>
            <div className="space-y-2 text-sm">
              <div className="flex items-center justify-between">
                <span>Slack</span>
                <button className="text-primary hover:underline">Connect</button>
              </div>
              <div className="flex items-center justify-between">
                <span>Atlassian</span>
                <button className="text-primary hover:underline">Connect</button>
              </div>
            </div>
          </div>
        </div>
      </section>
      
      {/* Sync Settings */}
      <section className="mb-8">
        <h2 className="mb-4 text-lg font-medium">Sync</h2>
        
        <div className="space-y-4">
          <div>
            <label className="mb-1 block text-sm font-medium">Sync Interval</label>
            <select
              value={prefs.syncIntervalMinutes}
              onChange={e => setPrefs({ ...prefs, syncIntervalMinutes: +e.target.value })}
              className="rounded-lg border border-border bg-background px-3 py-2 text-sm"
            >
              <option value={5}>Every 5 minutes</option>
              <option value={15}>Every 15 minutes</option>
              <option value={30}>Every 30 minutes</option>
              <option value={60}>Every hour</option>
            </select>
          </div>
          
          <div>
            <label className="mb-2 block text-sm font-medium">Enabled Sources</label>
            <div className="flex flex-wrap gap-2">
              {['slack', 'jira', 'confluence'].map(source => (
                <label key={source} className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={prefs.enabledSources.includes(source)}
                    onChange={e => {
                      const sources = e.target.checked
                        ? [...prefs.enabledSources, source]
                        : prefs.enabledSources.filter(s => s !== source);
                      setPrefs({ ...prefs, enabledSources: sources });
                    }}
                    className="h-4 w-4 rounded border-border"
                  />
                  <span className="text-sm capitalize">{source}</span>
                </label>
              ))}
            </div>
          </div>
          
          <div>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={prefs.notificationsEnabled}
                onChange={e => setPrefs({ ...prefs, notificationsEnabled: e.target.checked })}
                className="h-4 w-4 rounded border-border"
              />
              <span className="text-sm">Enable desktop notifications</span>
            </label>
          </div>
        </div>
      </section>
      
      {/* Save Button */}
      <button
        onClick={handleSave}
        disabled={saving}
        className="flex items-center gap-2 rounded-lg bg-primary px-6 py-2 font-medium text-white hover:bg-primary/90 disabled:opacity-50"
      >
        {saving ? <RefreshCw className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
        Save Settings
      </button>
    </div>
  );
}
```

---

## 4.6 App Layout and Routing

Update `src/App.tsx`:

```tsx
import { useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Calendar, CalendarDays, Settings as SettingsIcon } from 'lucide-react';
import { ThemeProvider } from './lib/theme';
import { DailyDigest } from './views/DailyDigest';
import { WeeklyDigest } from './views/WeeklyDigest';
import { Settings } from './views/Settings';

const queryClient = new QueryClient();

type View = 'daily' | 'weekly' | 'settings';

export default function App() {
  const [view, setView] = useState<View>('daily');
  
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <div className="flex h-screen bg-background">
          {/* Sidebar */}
          <nav className="flex w-16 flex-col items-center gap-4 border-r border-border bg-card py-4">
            <button
              onClick={() => setView('daily')}
              className={`rounded-lg p-3 ${view === 'daily' ? 'bg-primary text-white' : 'text-muted-foreground hover:bg-muted'}`}
              title="Daily Digest"
            >
              <Calendar className="h-5 w-5" />
            </button>
            <button
              onClick={() => setView('weekly')}
              className={`rounded-lg p-3 ${view === 'weekly' ? 'bg-primary text-white' : 'text-muted-foreground hover:bg-muted'}`}
              title="Weekly Digest"
            >
              <CalendarDays className="h-5 w-5" />
            </button>
            <div className="flex-1" />
            <button
              onClick={() => setView('settings')}
              className={`rounded-lg p-3 ${view === 'settings' ? 'bg-primary text-white' : 'text-muted-foreground hover:bg-muted'}`}
              title="Settings"
            >
              <SettingsIcon className="h-5 w-5" />
            </button>
          </nav>
          
          {/* Main Content */}
          <main className="flex-1 overflow-auto">
            {view === 'daily' && <DailyDigest />}
            {view === 'weekly' && <WeeklyDigest />}
            {view === 'settings' && <Settings />}
          </main>
        </div>
      </ThemeProvider>
    </QueryClientProvider>
  );
}
```

---

## Verification

- [ ] App renders with sidebar navigation
- [ ] Daily digest view loads and displays items
- [ ] Category filtering works
- [ ] Weekly digest shows category breakdown
- [ ] Settings view saves preferences
- [ ] Light/dark mode toggle works
- [ ] Content cards expand to show highlights

---

## Next Steps

Proceed to **Phase 5: MVP Polish** to add notifications, offline support, and analytics.
