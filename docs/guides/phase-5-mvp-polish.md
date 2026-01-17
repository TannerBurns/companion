# Phase 5: MVP Polish

This guide covers the final polish for the MVP including system notifications, offline support, analytics tracking, and macOS build configuration.

## Overview

By the end of this phase, you will have:
- System notifications for digests and important items
- Full offline support with local data
- Analytics tracking for usage patterns
- macOS build ready for distribution

---

## 5.1 System Notifications

### Configure Tauri Notifications

Update `src-tauri/tauri.conf.json` to enable notifications:

```json
{
  "plugins": {
    "notification": {
      "all": true
    }
  }
}
```

### Notification Service

Create `src-tauri/src/notifications.rs`:

```rust
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

pub struct NotificationService {
    app_handle: AppHandle,
}

impl NotificationService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Send notification when daily digest is ready
    pub fn notify_daily_digest(&self, item_count: i32) -> Result<(), String> {
        self.app_handle
            .notification()
            .builder()
            .title("Daily Digest Ready")
            .body(format!("{} new items to review", item_count))
            .show()
            .map_err(|e| e.to_string())
    }

    /// Send notification for high-importance items
    pub fn notify_important_item(&self, title: &str, source: &str) -> Result<(), String> {
        self.app_handle
            .notification()
            .builder()
            .title("Important Update")
            .body(format!("[{}] {}", source, title))
            .show()
            .map_err(|e| e.to_string())
    }

    /// Send notification when sync completes
    pub fn notify_sync_complete(&self, items_synced: i32) -> Result<(), String> {
        if items_synced > 0 {
            self.app_handle
                .notification()
                .builder()
                .title("Sync Complete")
                .body(format!("{} new items synced", items_synced))
                .show()
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}
```

### Frontend Notification Hook

Create `src/hooks/useNotifications.ts`:

```typescript
import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

interface DigestNotification {
  itemCount: number;
  date: string;
}

interface ImportantItemNotification {
  title: string;
  source: string;
  id: string;
}

export function useNotifications(onDigestReady?: (n: DigestNotification) => void) {
  useEffect(() => {
    const unlisten = listen<DigestNotification>('digest:ready', (event) => {
      onDigestReady?.(event.payload);
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, [onDigestReady]);
}
```

---

## 5.2 Offline Support

The app already stores all data locally in SQLite. Enhance with connection status awareness.

### Connection Status Hook

Create `src/hooks/useConnectionStatus.ts`:

```typescript
import { useState, useEffect } from 'react';

export function useConnectionStatus() {
  const [isOnline, setIsOnline] = useState(navigator.onLine);

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  return isOnline;
}
```

### Offline Indicator Component

Create `src/components/OfflineIndicator.tsx`:

```tsx
import { WifiOff } from 'lucide-react';
import { useConnectionStatus } from '../hooks/useConnectionStatus';

export function OfflineIndicator() {
  const isOnline = useConnectionStatus();

  if (isOnline) return null;

  return (
    <div className="fixed bottom-4 left-4 flex items-center gap-2 rounded-lg bg-yellow-100 px-3 py-2 text-sm text-yellow-800 shadow-lg dark:bg-yellow-900 dark:text-yellow-200">
      <WifiOff className="h-4 w-4" />
      <span>Offline - viewing cached data</span>
    </div>
  );
}
```

### Sync Queue for Offline

Create `src-tauri/src/sync/queue.rs`:

```rust
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub id: String,
    pub source: String,
    pub created_at: i64,
}

pub struct SyncQueue {
    queue: Mutex<VecDeque<SyncRequest>>,
}

impl SyncQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub async fn enqueue(&self, request: SyncRequest) {
        let mut queue = self.queue.lock().await;
        queue.push_back(request);
    }

    pub async fn dequeue(&self) -> Option<SyncRequest> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }

    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }

    pub async fn process_all<F, Fut>(&self, mut processor: F)
    where
        F: FnMut(SyncRequest) -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        while let Some(request) = self.dequeue().await {
            if let Err(e) = processor(request.clone()).await {
                tracing::error!("Failed to process sync request: {}", e);
                // Re-queue failed requests
                self.enqueue(request).await;
                break;
            }
        }
    }
}
```

---

## 5.3 Analytics Tracking

### Analytics Service

Create `src-tauri/src/analytics.rs`:

```rust
use crate::db::Database;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub event_type: String,
    pub event_data: serde_json::Value,
}

pub struct AnalyticsService {
    db: Arc<Database>,
}

impl AnalyticsService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn track(&self, event: AnalyticsEvent) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        
        sqlx::query(
            "INSERT INTO analytics (event_type, event_data, created_at) VALUES (?, ?, ?)"
        )
        .bind(&event.event_type)
        .bind(serde_json::to_string(&event.event_data).unwrap())
        .bind(now)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    pub async fn track_view(&self, view_name: &str) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent {
            event_type: "view".to_string(),
            event_data: serde_json::json!({ "view": view_name }),
        }).await
    }

    pub async fn track_sync(&self, source: &str, items: i32, duration_ms: i64) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent {
            event_type: "sync".to_string(),
            event_data: serde_json::json!({
                "source": source,
                "items_synced": items,
                "duration_ms": duration_ms
            }),
        }).await
    }

    pub async fn track_ai_request(&self, model: &str, tokens: i32, latency_ms: i64) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent {
            event_type: "ai_request".to_string(),
            event_data: serde_json::json!({
                "model": model,
                "tokens": tokens,
                "latency_ms": latency_ms
            }),
        }).await
    }

    pub async fn track_source_click(&self, source: &str, item_id: &str) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent {
            event_type: "source_click".to_string(),
            event_data: serde_json::json!({
                "source": source,
                "item_id": item_id
            }),
        }).await
    }

    /// Get usage summary for a time period
    pub async fn get_summary(&self, days: i32) -> Result<UsageSummary, sqlx::Error> {
        let since = chrono::Utc::now().timestamp() - (days as i64 * 86400);

        let total_syncs: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics WHERE event_type = 'sync' AND created_at >= ?"
        )
        .bind(since)
        .fetch_one(self.db.pool())
        .await?;

        let total_ai_requests: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics WHERE event_type = 'ai_request' AND created_at >= ?"
        )
        .bind(since)
        .fetch_one(self.db.pool())
        .await?;

        let total_views: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics WHERE event_type = 'view' AND created_at >= ?"
        )
        .bind(since)
        .fetch_one(self.db.pool())
        .await?;

        Ok(UsageSummary {
            total_syncs: total_syncs.0 as i32,
            total_ai_requests: total_ai_requests.0 as i32,
            total_views: total_views.0 as i32,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    pub total_syncs: i32,
    pub total_ai_requests: i32,
    pub total_views: i32,
}
```

### Analytics Command

Add to `src-tauri/src/commands/mod.rs`:

```rust
#[tauri::command]
pub async fn track_event(
    state: State<'_, Arc<Mutex<AppState>>>,
    event_type: String,
    event_data: serde_json::Value,
) -> Result<(), String> {
    let state = state.lock().await;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO analytics (event_type, event_data, created_at) VALUES (?, ?, ?)"
    )
    .bind(&event_type)
    .bind(serde_json::to_string(&event_data).unwrap())
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_analytics_summary(
    state: State<'_, Arc<Mutex<AppState>>>,
    days: i32,
) -> Result<serde_json::Value, String> {
    let state = state.lock().await;
    let since = chrono::Utc::now().timestamp() - (days as i64 * 86400);

    let counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT event_type, COUNT(*) FROM analytics WHERE created_at >= ? GROUP BY event_type"
    )
    .bind(since)
    .fetch_all(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;

    let summary: std::collections::HashMap<_, _> = counts.into_iter().collect();
    Ok(serde_json::json!(summary))
}
```

---

## 5.4 macOS Build Configuration

### Update Tauri Config

Update `src-tauri/tauri.conf.json`:

```json
{
  "productName": "Companion",
  "version": "0.1.0",
  "identifier": "com.companion.app",
  "build": {
    "beforeBuildCommand": "pnpm build",
    "beforeDevCommand": "pnpm dev",
    "frontendDist": "../dist",
    "devUrl": "http://localhost:5173"
  },
  "app": {
    "windows": [
      {
        "title": "Companion",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "macOS": {
      "minimumSystemVersion": "10.15",
      "exceptionDomain": "",
      "signingIdentity": null,
      "providerShortName": null,
      "entitlements": null
    }
  }
}
```

### Create App Icons

Create icons in `src-tauri/icons/`:

```bash
# Generate icons from a 1024x1024 source image
# Use a tool like iconutil on macOS or online converters

mkdir -p src-tauri/icons
# Place your icon files:
# - 32x32.png
# - 128x128.png
# - 128x128@2x.png
# - icon.icns (macOS)
# - icon.ico (Windows)
```

### Build Commands

```bash
# Development
pnpm tauri dev

# Build for production
pnpm tauri build

# Build for specific target
pnpm tauri build --target aarch64-apple-darwin  # Apple Silicon
pnpm tauri build --target x86_64-apple-darwin   # Intel Mac
```

### Code Signing (Optional for Distribution)

For App Store or notarized distribution, add to environment:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export APPLE_ID="your@email.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAMID"
```

---

## 5.5 Final Integration

### Update Main Entry Point

Update `src-tauri/src/main.rs` with all services:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod crypto;
mod db;
mod ai;
mod sync;
mod notifications;
mod analytics;

use db::Database;
use crypto::CryptoService;
use notifications::NotificationService;
use analytics::AnalyticsService;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct AppState {
    pub db: Database,
    pub crypto: CryptoService,
    pub notifications: NotificationService,
    pub analytics: AnalyticsService,
}

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            tauri::async_runtime::block_on(async {
                let db = Database::new(&app_handle).await
                    .expect("Failed to initialize database");
                
                let crypto = CryptoService::new()
                    .expect("Failed to initialize crypto service");
                
                let db_arc = Arc::new(db);
                let notifications = NotificationService::new(app_handle.clone());
                let analytics = AnalyticsService::new(db_arc.clone());
                
                app.manage(Arc::new(Mutex::new(AppState {
                    db: Database::new(&app_handle).await.unwrap(),
                    crypto,
                    notifications,
                    analytics,
                })));
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_daily_digest,
            commands::get_weekly_digest,
            commands::start_sync,
            commands::get_sync_status,
            commands::save_api_key,
            commands::get_preferences,
            commands::save_preferences,
            commands::connect_slack,
            commands::connect_atlassian,
            commands::track_event,
            commands::get_analytics_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Add Offline Indicator to App

Update `src/App.tsx`:

```tsx
import { OfflineIndicator } from './components/OfflineIndicator';

// Add inside the main div, after </main>:
<OfflineIndicator />
```

---

## Verification Checklist

### Notifications
- [ ] Daily digest notification appears
- [ ] Important item notifications work
- [ ] Sync completion notifies when items found

### Offline Support
- [ ] App works without internet connection
- [ ] Offline indicator shows when disconnected
- [ ] Cached data displays correctly
- [ ] Sync resumes when back online

### Analytics
- [ ] View events tracked
- [ ] Sync events tracked
- [ ] AI request events tracked
- [ ] Analytics summary returns data

### Build
- [ ] `pnpm tauri dev` runs successfully
- [ ] `pnpm tauri build` produces .dmg
- [ ] App launches from Applications folder
- [ ] All features work in production build

---

## MVP Complete

Congratulations! You now have a fully functional Companion MVP with:

1. **Data Integration**: Slack and Atlassian OAuth with incremental sync
2. **AI Processing**: Gemini-powered summarization and categorization
3. **Frontend**: Modern React UI with daily/weekly digests
4. **Security**: Encrypted credential storage with OS keychain
5. **Polish**: Notifications, offline support, and analytics

### Next Steps (Post-MVP)
- Custom categories and user training
- Windows/Linux builds
- Real-time webhooks for instant updates
- Export and sharing features
- Advanced search capabilities
