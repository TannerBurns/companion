# Phase 1: Project Foundation

This guide covers the initial setup of the Companion MVP, including Tauri project initialization, Rust dependencies, database schema, and the encryption layer.

## Overview

By the end of this phase, you will have:
- A fully configured Tauri + Vite + React + TypeScript project
- SQLite database with sqlx migrations
- Secure credential storage using OS keychain + AES-256-GCM
- Project structure ready for feature development

---

## 1.1 Initialize Tauri Project

### Prerequisites

Ensure you have the following installed:
- **Rust** (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js** (18+): Use nvm or download from nodejs.org
- **pnpm** (recommended): `npm install -g pnpm`

### Create Project

```bash
# Create the Tauri + Vite + React + TypeScript project
pnpm create tauri-app companion --template react-ts

cd companion

# Install frontend dependencies
pnpm install
```

### Install Additional Frontend Dependencies

```bash
# Core UI and state management
pnpm add @tanstack/react-query zustand

# Styling
pnpm add -D tailwindcss postcss autoprefixer
pnpm add @headlessui/react lucide-react clsx

# Tauri API
pnpm add @tauri-apps/api @tauri-apps/plugin-notification @tauri-apps/plugin-shell

# Date handling
pnpm add date-fns

# Dev dependencies
pnpm add -D @types/node
```

### Configure TailwindCSS

```bash
npx tailwindcss init -p
```

Update `tailwind.config.js`:

```javascript
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Custom color palette
        primary: {
          50: '#f0f9ff',
          100: '#e0f2fe',
          200: '#bae6fd',
          300: '#7dd3fc',
          400: '#38bdf8',
          500: '#0ea5e9',
          600: '#0284c7',
          700: '#0369a1',
          800: '#075985',
          900: '#0c4a6e',
        },
      },
    },
  },
  plugins: [],
}
```

Update `src/index.css`:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 222.2 84% 4.9%;
    --card: 0 0% 100%;
    --card-foreground: 222.2 84% 4.9%;
    --muted: 210 40% 96.1%;
    --muted-foreground: 215.4 16.3% 46.9%;
    --accent: 210 40% 96.1%;
    --accent-foreground: 222.2 47.4% 11.2%;
    --border: 214.3 31.8% 91.4%;
  }

  .dark {
    --background: 222.2 84% 4.9%;
    --foreground: 210 40% 98%;
    --card: 222.2 84% 4.9%;
    --card-foreground: 210 40% 98%;
    --muted: 217.2 32.6% 17.5%;
    --muted-foreground: 215 20.2% 65.1%;
    --accent: 217.2 32.6% 17.5%;
    --accent-foreground: 210 40% 98%;
    --border: 217.2 32.6% 17.5%;
  }
}

body {
  @apply bg-background text-foreground;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
}
```

---

## 1.2 Configure Rust Dependencies

Update `src-tauri/Cargo.toml`:

```toml
[package]
name = "companion"
version = "0.1.0"
description = "AI-powered work companion for Slack and Atlassian"
authors = ["Your Name"]
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
# Tauri
tauri = { version = "2", features = ["devtools"] }
tauri-plugin-notification = "2"
tauri-plugin-shell = "2"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate"] }

# HTTP client
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Encryption
aes-gcm = "0.10"
rand = "0.8"

# OS Keychain
keyring = { version = "3", features = ["apple-native", "windows-native", "linux-native"] }

# Utilities
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]
```

### Project Structure

Create the Rust module structure:

```bash
cd src-tauri/src

# Create module directories
mkdir -p commands db sync ai crypto

# Create module files
touch commands/mod.rs
touch db/mod.rs db/schema.rs db/migrations.rs
touch sync/mod.rs sync/slack.rs sync/atlassian.rs
touch ai/mod.rs ai/gemini.rs ai/prompts.rs
touch crypto/mod.rs
```

Update `src-tauri/src/main.rs`:

```rust
// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod crypto;
mod db;
mod ai;
mod sync;

use db::Database;
use crypto::CryptoService;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct AppState {
    pub db: Database,
    pub crypto: CryptoService,
}

fn main() {
    // Initialize logging
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
                // Initialize database
                let db = Database::new(&app_handle).await
                    .expect("Failed to initialize database");
                
                // Initialize crypto service
                let crypto = CryptoService::new()
                    .expect("Failed to initialize crypto service");
                
                // Store state
                app.manage(Arc::new(Mutex::new(AppState { db, crypto })));
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 1.3 Database Schema and Migrations

### Create Migration Files

Create `src-tauri/migrations/` directory and add the initial migration:

```bash
mkdir -p src-tauri/migrations
```

Create `src-tauri/migrations/001_initial_schema.sql`:

```sql
-- Unified content items from all sources
CREATE TABLE IF NOT EXISTS content_items (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,           -- 'slack', 'jira', 'confluence'
    source_id TEXT NOT NULL,        -- Original ID from source
    source_url TEXT,                -- Deep link back to source
    content_type TEXT NOT NULL,     -- 'message', 'ticket', 'page', 'comment'
    title TEXT,
    body TEXT,                       -- Raw content (encrypted)
    author TEXT,
    author_id TEXT,
    channel_or_project TEXT,        -- Slack channel / Jira project / Confluence space
    parent_id TEXT,                 -- For threading
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    synced_at INTEGER NOT NULL,
    metadata TEXT,                  -- JSON blob for source-specific data
    UNIQUE(source, source_id)
);

-- AI-generated summaries and analysis
CREATE TABLE IF NOT EXISTS ai_summaries (
    id TEXT PRIMARY KEY,
    content_item_id TEXT,
    summary_type TEXT NOT NULL,     -- 'item', 'daily', 'weekly'
    summary TEXT NOT NULL,
    highlights TEXT,                -- JSON array of key points
    category TEXT,                  -- 'sales', 'marketing', 'product', 'engineering', 'research'
    category_confidence REAL,
    importance_score REAL,
    entities TEXT,                  -- JSON: people, projects, topics
    generated_at INTEGER NOT NULL,
    user_override_category TEXT,    -- If user recategorized
    FOREIGN KEY (content_item_id) REFERENCES content_items(id) ON DELETE CASCADE
);

-- Sync state tracking
CREATE TABLE IF NOT EXISTS sync_state (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    resource_type TEXT NOT NULL,    -- 'channel', 'project', 'space'
    resource_id TEXT NOT NULL,
    last_sync_at INTEGER,
    cursor TEXT,                    -- Pagination cursor for incremental sync
    status TEXT,                    -- 'pending', 'syncing', 'complete', 'error'
    error_message TEXT,
    UNIQUE(source, resource_type, resource_id)
);

-- User preferences
CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL             -- JSON values
);

-- Analytics/audit log
CREATE TABLE IF NOT EXISTS analytics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,       -- 'view', 'click', 'ai_request', 'sync'
    event_data TEXT,                -- JSON
    created_at INTEGER NOT NULL
);

-- Credentials (encrypted)
CREATE TABLE IF NOT EXISTS credentials (
    id TEXT PRIMARY KEY,
    service TEXT NOT NULL,          -- 'slack', 'atlassian', 'gemini'
    encrypted_data TEXT NOT NULL,   -- AES-GCM encrypted token data
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_content_items_source ON content_items(source);
CREATE INDEX IF NOT EXISTS idx_content_items_created_at ON content_items(created_at);
CREATE INDEX IF NOT EXISTS idx_content_items_channel ON content_items(channel_or_project);
CREATE INDEX IF NOT EXISTS idx_ai_summaries_type ON ai_summaries(summary_type);
CREATE INDEX IF NOT EXISTS idx_ai_summaries_category ON ai_summaries(category);
CREATE INDEX IF NOT EXISTS idx_ai_summaries_generated_at ON ai_summaries(generated_at);
CREATE INDEX IF NOT EXISTS idx_sync_state_source ON sync_state(source);
CREATE INDEX IF NOT EXISTS idx_analytics_event_type ON analytics(event_type);
CREATE INDEX IF NOT EXISTS idx_analytics_created_at ON analytics(created_at);
```

### Database Module

Create `src-tauri/src/db/mod.rs`:

```rust
pub mod schema;
mod migrations;

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(app_handle: &AppHandle) -> Result<Self, DbError> {
        // Get app data directory
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .expect("Failed to get app data dir");
        
        // Ensure directory exists
        std::fs::create_dir_all(&app_dir)?;
        
        let db_path = app_dir.join("companion.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        
        tracing::info!("Opening database at: {}", db_path.display());
        
        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;
        
        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;
        
        tracing::info!("Database initialized successfully");
        
        Ok(Self { pool })
    }
    
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
```

Create `src-tauri/src/db/schema.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Source {
    Slack,
    Jira,
    Confluence,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum ContentType {
    Message,
    Ticket,
    Page,
    Comment,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum SummaryType {
    Item,
    Daily,
    Weekly,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Category {
    Sales,
    Marketing,
    Product,
    Engineering,
    Research,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum SyncStatus {
    Pending,
    Syncing,
    Complete,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContentItem {
    pub id: String,
    pub source: String,
    pub source_id: String,
    pub source_url: Option<String>,
    pub content_type: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub author: Option<String>,
    pub author_id: Option<String>,
    pub channel_or_project: Option<String>,
    pub parent_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub synced_at: i64,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AiSummary {
    pub id: String,
    pub content_item_id: Option<String>,
    pub summary_type: String,
    pub summary: String,
    pub highlights: Option<String>,
    pub category: Option<String>,
    pub category_confidence: Option<f64>,
    pub importance_score: Option<f64>,
    pub entities: Option<String>,
    pub generated_at: i64,
    pub user_override_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncState {
    pub id: String,
    pub source: String,
    pub resource_type: String,
    pub resource_id: String,
    pub last_sync_at: Option<i64>,
    pub cursor: Option<String>,
    pub status: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Preference {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Credential {
    pub id: String,
    pub service: String,
    pub encrypted_data: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnalyticsEvent {
    pub id: i64,
    pub event_type: String,
    pub event_data: Option<String>,
    pub created_at: i64,
}
```

Create `src-tauri/src/db/migrations.rs`:

```rust
// This module is a placeholder for any runtime migration helpers
// SQLx handles migrations via the migrate! macro
```

---

## 1.4 Encryption Layer

Create `src-tauri/src/crypto/mod.rs`:

```rust
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use keyring::Entry;
use rand::RngCore;
use thiserror::Error;

const SERVICE_NAME: &str = "companion-app";
const MASTER_KEY_NAME: &str = "master-encryption-key";
const NONCE_SIZE: usize = 12;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    
    #[error("Encryption error")]
    Encryption,
    
    #[error("Decryption error")]
    Decryption,
    
    #[error("Invalid key length")]
    InvalidKeyLength,
    
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    /// Create a new CryptoService, generating or retrieving the master key from OS keychain
    pub fn new() -> Result<Self, CryptoError> {
        let master_key = Self::get_or_create_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(&master_key)
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        
        Ok(Self { cipher })
    }
    
    /// Get existing master key or create a new one
    fn get_or_create_master_key() -> Result<[u8; 32], CryptoError> {
        let entry = Entry::new(SERVICE_NAME, MASTER_KEY_NAME)?;
        
        match entry.get_password() {
            Ok(key_b64) => {
                // Decode existing key
                let key_bytes = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &key_b64
                )?;
                
                let mut key = [0u8; 32];
                if key_bytes.len() != 32 {
                    return Err(CryptoError::InvalidKeyLength);
                }
                key.copy_from_slice(&key_bytes);
                
                tracing::info!("Retrieved master key from keychain");
                Ok(key)
            }
            Err(keyring::Error::NoEntry) => {
                // Generate new key
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                
                // Store in keychain
                let key_b64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &key
                );
                entry.set_password(&key_b64)?;
                
                tracing::info!("Generated and stored new master key in keychain");
                Ok(key)
            }
            Err(e) => Err(CryptoError::Keyring(e)),
        }
    }
    
    /// Encrypt plaintext, returning base64-encoded ciphertext with prepended nonce
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, CryptoError> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::Encryption)?;
        
        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        // Base64 encode
        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &result
        ))
    }
    
    /// Decrypt base64-encoded ciphertext (with prepended nonce)
    pub fn decrypt(&self, ciphertext_b64: &str) -> Result<Vec<u8>, CryptoError> {
        // Base64 decode
        let data = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            ciphertext_b64
        )?;
        
        if data.len() < NONCE_SIZE {
            return Err(CryptoError::Decryption);
        }
        
        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Decrypt
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::Decryption)
    }
    
    /// Encrypt a string value
    pub fn encrypt_string(&self, plaintext: &str) -> Result<String, CryptoError> {
        self.encrypt(plaintext.as_bytes())
    }
    
    /// Decrypt to a string value
    pub fn decrypt_string(&self, ciphertext_b64: &str) -> Result<String, CryptoError> {
        let bytes = self.decrypt(ciphertext_b64)?;
        String::from_utf8(bytes).map_err(|_| CryptoError::Decryption)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let crypto = CryptoService::new().unwrap();
        let plaintext = "Hello, World!";
        
        let encrypted = crypto.encrypt_string(plaintext).unwrap();
        let decrypted = crypto.decrypt_string(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }
}
```

---

## 1.5 Command Stubs

Create `src-tauri/src/commands/mod.rs`:

```rust
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub source: String,
    pub source_url: Option<String>,
    pub importance_score: f64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestResponse {
    pub date: String,
    pub items: Vec<DigestItem>,
    pub categories: Vec<CategorySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySummary {
    pub name: String,
    pub count: i32,
    pub top_items: Vec<DigestItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub is_syncing: bool,
    pub last_sync_at: Option<i64>,
    pub sources: Vec<SourceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatus {
    pub name: String,
    pub status: String,
    pub items_synced: i32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub sync_interval_minutes: i32,
    pub enabled_sources: Vec<String>,
    pub enabled_categories: Vec<String>,
    pub notifications_enabled: bool,
}

#[tauri::command]
pub async fn get_daily_digest(
    state: State<'_, Arc<Mutex<AppState>>>,
    date: Option<String>,
) -> Result<DigestResponse, String> {
    // TODO: Implement in Phase 3
    Ok(DigestResponse {
        date: date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
        items: vec![],
        categories: vec![],
    })
}

#[tauri::command]
pub async fn get_weekly_digest(
    state: State<'_, Arc<Mutex<AppState>>>,
    week_start: Option<String>,
) -> Result<DigestResponse, String> {
    // TODO: Implement in Phase 3
    Ok(DigestResponse {
        date: week_start.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
        items: vec![],
        categories: vec![],
    })
}

#[tauri::command]
pub async fn start_sync(
    state: State<'_, Arc<Mutex<AppState>>>,
    sources: Option<Vec<String>>,
) -> Result<(), String> {
    // TODO: Implement in Phase 2
    tracing::info!("Sync requested for sources: {:?}", sources);
    Ok(())
}

#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SyncStatus, String> {
    // TODO: Implement in Phase 2
    Ok(SyncStatus {
        is_syncing: false,
        last_sync_at: None,
        sources: vec![],
    })
}

#[tauri::command]
pub async fn save_api_key(
    state: State<'_, Arc<Mutex<AppState>>>,
    service: String,
    api_key: String,
) -> Result<(), String> {
    let state = state.lock().await;
    
    // Encrypt the API key
    let encrypted = state.crypto
        .encrypt_string(&api_key)
        .map_err(|e| e.to_string())?;
    
    // Store in database
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at) 
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET encrypted_data = ?, updated_at = ?"
    )
    .bind(&service)
    .bind(&service)
    .bind(&encrypted)
    .bind(now)
    .bind(now)
    .bind(&encrypted)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    tracing::info!("Saved encrypted API key for service: {}", service);
    Ok(())
}

#[tauri::command]
pub async fn get_preferences(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Preferences, String> {
    // TODO: Load from database
    Ok(Preferences {
        sync_interval_minutes: 15,
        enabled_sources: vec!["slack".to_string(), "jira".to_string(), "confluence".to_string()],
        enabled_categories: vec![
            "sales".to_string(),
            "marketing".to_string(),
            "product".to_string(),
            "engineering".to_string(),
            "research".to_string(),
        ],
        notifications_enabled: true,
    })
}

#[tauri::command]
pub async fn save_preferences(
    state: State<'_, Arc<Mutex<AppState>>>,
    preferences: Preferences,
) -> Result<(), String> {
    let state = state.lock().await;
    let prefs_json = serde_json::to_string(&preferences).map_err(|e| e.to_string())?;
    
    sqlx::query("INSERT OR REPLACE INTO preferences (key, value) VALUES ('user_preferences', ?)")
        .bind(&prefs_json)
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
```

---

## 1.6 Remaining Module Stubs

Create placeholder modules for the remaining Rust code:

`src-tauri/src/sync/mod.rs`:

```rust
pub mod slack;
pub mod atlassian;

// Will be implemented in Phase 2
```

`src-tauri/src/sync/slack.rs`:

```rust
// Slack OAuth and sync implementation
// See Phase 2 guide for implementation details
```

`src-tauri/src/sync/atlassian.rs`:

```rust
// Atlassian OAuth and sync implementation
// See Phase 2 guide for implementation details
```

`src-tauri/src/ai/mod.rs`:

```rust
pub mod gemini;
pub mod prompts;

// Will be implemented in Phase 3
```

`src-tauri/src/ai/gemini.rs`:

```rust
// Gemini API client implementation
// See Phase 3 guide for implementation details
```

`src-tauri/src/ai/prompts.rs`:

```rust
// Prompt templates for AI processing
// See Phase 3 guide for implementation details
```

---

## Verification

After completing Phase 1, verify your setup:

```bash
# Build the Rust backend
cd src-tauri
cargo build

# Run frontend dev server
cd ..
pnpm dev

# Run the full Tauri app in development
pnpm tauri dev
```

### Checklist

- [ ] Tauri project initializes without errors
- [ ] SQLite database is created in app data directory
- [ ] Migrations run successfully on first launch
- [ ] Master encryption key is stored in OS keychain
- [ ] Basic Tauri commands respond (can test via browser console)
- [ ] TailwindCSS styling works in the React app

---

## Next Steps

Proceed to **Phase 2: Data Integration** to implement Slack and Atlassian OAuth flows and sync services.
