# Phase 2: Data Integration

This guide covers implementing OAuth flows and sync services for Slack and Atlassian (Jira + Confluence), plus the unified content model and background sync service.

## Overview

By the end of this phase, you will have:
- Slack OAuth 2.0 flow with token storage
- Atlassian Cloud OAuth 2.0 (3LO) with PKCE
- Sync services for channels, messages, issues, and pages
- Unified content model for all sources
- Background sync with progress events

---

## 2.1 Slack Integration

### Register Slack App

1. Go to [api.slack.com/apps](https://api.slack.com/apps)
2. Create a new app "From scratch"
3. Under **OAuth & Permissions**, add redirect URL: `http://localhost:8374/slack/callback`
4. Add the following **User Token Scopes**:
   - `channels:history` - View messages in public channels
   - `channels:read` - View public channels
   - `groups:history` - View messages in private channels
   - `groups:read` - View private channels
   - `im:history` - View DMs
   - `im:read` - View DM list
   - `mpim:history` - View group DMs
   - `mpim:read` - View group DM list
   - `users:read` - View users
   - `search:read` - Search messages

5. Copy your **Client ID** and **Client Secret**

### Slack Auth Module

Create `src-tauri/src/sync/slack.rs`:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::oneshot;
use tokio::net::TcpListener;

use crate::crypto::CryptoService;
use crate::db::Database;

const SLACK_AUTHORIZE_URL: &str = "https://slack.com/oauth/v2/authorize";
const SLACK_TOKEN_URL: &str = "https://slack.com/api/oauth.v2.access";
const SLACK_API_BASE: &str = "https://slack.com/api";
const REDIRECT_PORT: u16 = 8374;

#[derive(Error, Debug)]
pub enum SlackError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("OAuth error: {0}")]
    OAuth(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Crypto error: {0}")]
    Crypto(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackTokens {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
    pub team_id: String,
    pub team_name: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
struct OAuthResponse {
    ok: bool,
    access_token: Option<String>,
    token_type: Option<String>,
    scope: Option<String>,
    team: Option<TeamInfo>,
    authed_user: Option<AuthedUser>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TeamInfo {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct AuthedUser {
    id: String,
    access_token: Option<String>,
    token_type: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlackChannel {
    pub id: String,
    pub name: String,
    pub is_private: bool,
    pub is_im: bool,
    pub is_mpim: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlackMessage {
    pub ts: String,
    pub user: Option<String>,
    pub text: String,
    pub thread_ts: Option<String>,
    pub reply_count: Option<i32>,
}

pub struct SlackClient {
    http: Client,
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
}

impl SlackClient {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            http: Client::new(),
            client_id,
            client_secret,
            access_token: None,
        }
    }
    
    pub fn with_token(mut self, access_token: String) -> Self {
        self.access_token = Some(access_token);
        self
    }
    
    /// Generate OAuth authorization URL
    pub fn get_auth_url(&self, state: &str) -> String {
        let scopes = [
            "channels:history",
            "channels:read", 
            "groups:history",
            "groups:read",
            "im:history",
            "im:read",
            "mpim:history",
            "mpim:read",
            "users:read",
            "search:read",
        ].join(",");
        
        format!(
            "{}?client_id={}&scope={}&redirect_uri=http://localhost:{}/slack/callback&state={}&user_scope={}",
            SLACK_AUTHORIZE_URL,
            self.client_id,
            "", // Bot scopes (empty for user-only)
            REDIRECT_PORT,
            state,
            scopes,
        )
    }
    
    /// Start OAuth flow - opens browser and waits for callback
    pub async fn start_oauth_flow(&self) -> Result<SlackTokens, SlackError> {
        let state = uuid::Uuid::new_v4().to_string();
        let auth_url = self.get_auth_url(&state);
        
        // Open browser
        open::that(&auth_url).map_err(|e| SlackError::OAuth(e.to_string()))?;
        
        // Start local server to receive callback
        let (tx, rx) = oneshot::channel();
        let expected_state = state.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::wait_for_callback(tx, expected_state).await {
                tracing::error!("OAuth callback error: {}", e);
            }
        });
        
        // Wait for authorization code
        let code = rx.await.map_err(|_| SlackError::OAuth("Callback cancelled".into()))?;
        
        // Exchange code for tokens
        self.exchange_code(&code).await
    }
    
    /// Wait for OAuth callback on localhost
    async fn wait_for_callback(tx: oneshot::Sender<String>, expected_state: String) -> Result<(), SlackError> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", REDIRECT_PORT)).await?;
        
        let (mut socket, _) = listener.accept().await?;
        
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        
        let mut buffer = [0; 1024];
        socket.read(&mut buffer).await?;
        
        let request = String::from_utf8_lossy(&buffer);
        
        // Parse code and state from GET request
        if let Some(query_start) = request.find('?') {
            if let Some(query_end) = request[query_start..].find(' ') {
                let query = &request[query_start + 1..query_start + query_end];
                let params: HashMap<_, _> = query
                    .split('&')
                    .filter_map(|p| {
                        let mut parts = p.split('=');
                        Some((parts.next()?, parts.next()?))
                    })
                    .collect();
                
                if let (Some(code), Some(state)) = (params.get("code"), params.get("state")) {
                    if *state == expected_state {
                        // Send success response
                        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authorization successful!</h1><p>You can close this window.</p></body></html>";
                        socket.write_all(response.as_bytes()).await?;
                        
                        let _ = tx.send(code.to_string());
                        return Ok(());
                    }
                }
            }
        }
        
        // Send error response
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authorization failed</h1></body></html>";
        socket.write_all(response.as_bytes()).await?;
        
        Err(SlackError::OAuth("Invalid callback".into()))
    }
    
    /// Exchange authorization code for tokens
    async fn exchange_code(&self, code: &str) -> Result<SlackTokens, SlackError> {
        let response: OAuthResponse = self.http
            .post(SLACK_TOKEN_URL)
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", &format!("http://localhost:{}/slack/callback", REDIRECT_PORT)),
            ])
            .send()
            .await?
            .json()
            .await?;
        
        if !response.ok {
            return Err(SlackError::OAuth(response.error.unwrap_or_default()));
        }
        
        let user = response.authed_user.ok_or_else(|| SlackError::OAuth("No user token".into()))?;
        let team = response.team.ok_or_else(|| SlackError::OAuth("No team info".into()))?;
        
        Ok(SlackTokens {
            access_token: user.access_token.ok_or_else(|| SlackError::OAuth("No access token".into()))?,
            token_type: user.token_type.unwrap_or_else(|| "bearer".into()),
            scope: user.scope.unwrap_or_default(),
            team_id: team.id,
            team_name: team.name,
            user_id: user.id,
        })
    }
    
    /// Fetch list of channels
    pub async fn list_channels(&self) -> Result<Vec<SlackChannel>, SlackError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| SlackError::OAuth("Not authenticated".into()))?;
        
        let mut all_channels = Vec::new();
        let mut cursor: Option<String> = None;
        
        loop {
            let mut params = vec![
                ("types", "public_channel,private_channel,mpim,im"),
                ("limit", "200"),
            ];
            
            let cursor_str;
            if let Some(ref c) = cursor {
                cursor_str = c.clone();
                params.push(("cursor", &cursor_str));
            }
            
            let response: serde_json::Value = self.http
                .get(format!("{}/conversations.list", SLACK_API_BASE))
                .bearer_auth(token)
                .query(&params)
                .send()
                .await?
                .json()
                .await?;
            
            if !response["ok"].as_bool().unwrap_or(false) {
                return Err(SlackError::Api(
                    response["error"].as_str().unwrap_or("Unknown error").to_string()
                ));
            }
            
            if let Some(channels) = response["channels"].as_array() {
                for ch in channels {
                    all_channels.push(SlackChannel {
                        id: ch["id"].as_str().unwrap_or_default().to_string(),
                        name: ch["name"].as_str().unwrap_or_default().to_string(),
                        is_private: ch["is_private"].as_bool().unwrap_or(false),
                        is_im: ch["is_im"].as_bool().unwrap_or(false),
                        is_mpim: ch["is_mpim"].as_bool().unwrap_or(false),
                    });
                }
            }
            
            // Check for pagination
            cursor = response["response_metadata"]["next_cursor"]
                .as_str()
                .filter(|c| !c.is_empty())
                .map(String::from);
            
            if cursor.is_none() {
                break;
            }
        }
        
        Ok(all_channels)
    }
    
    /// Fetch messages from a channel
    pub async fn get_channel_history(
        &self,
        channel_id: &str,
        oldest: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SlackMessage>, SlackError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| SlackError::OAuth("Not authenticated".into()))?;
        
        let mut params = vec![
            ("channel", channel_id),
            ("limit", &limit.to_string()),
        ];
        
        if let Some(ts) = oldest {
            params.push(("oldest", ts));
        }
        
        let response: serde_json::Value = self.http
            .get(format!("{}/conversations.history", SLACK_API_BASE))
            .bearer_auth(token)
            .query(&params)
            .send()
            .await?
            .json()
            .await?;
        
        if !response["ok"].as_bool().unwrap_or(false) {
            return Err(SlackError::Api(
                response["error"].as_str().unwrap_or("Unknown error").to_string()
            ));
        }
        
        let messages = response["messages"]
            .as_array()
            .map(|msgs| {
                msgs.iter()
                    .map(|m| SlackMessage {
                        ts: m["ts"].as_str().unwrap_or_default().to_string(),
                        user: m["user"].as_str().map(String::from),
                        text: m["text"].as_str().unwrap_or_default().to_string(),
                        thread_ts: m["thread_ts"].as_str().map(String::from),
                        reply_count: m["reply_count"].as_i64().map(|n| n as i32),
                    })
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(messages)
    }
    
    /// Fetch thread replies
    pub async fn get_thread_replies(
        &self,
        channel_id: &str,
        thread_ts: &str,
    ) -> Result<Vec<SlackMessage>, SlackError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| SlackError::OAuth("Not authenticated".into()))?;
        
        let response: serde_json::Value = self.http
            .get(format!("{}/conversations.replies", SLACK_API_BASE))
            .bearer_auth(token)
            .query(&[
                ("channel", channel_id),
                ("ts", thread_ts),
            ])
            .send()
            .await?
            .json()
            .await?;
        
        if !response["ok"].as_bool().unwrap_or(false) {
            return Err(SlackError::Api(
                response["error"].as_str().unwrap_or("Unknown error").to_string()
            ));
        }
        
        let messages = response["messages"]
            .as_array()
            .map(|msgs| {
                msgs.iter()
                    .map(|m| SlackMessage {
                        ts: m["ts"].as_str().unwrap_or_default().to_string(),
                        user: m["user"].as_str().map(String::from),
                        text: m["text"].as_str().unwrap_or_default().to_string(),
                        thread_ts: m["thread_ts"].as_str().map(String::from),
                        reply_count: m["reply_count"].as_i64().map(|n| n as i32),
                    })
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(messages)
    }
    
    /// Get user info
    pub async fn get_user_info(&self, user_id: &str) -> Result<serde_json::Value, SlackError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| SlackError::OAuth("Not authenticated".into()))?;
        
        let response: serde_json::Value = self.http
            .get(format!("{}/users.info", SLACK_API_BASE))
            .bearer_auth(token)
            .query(&[("user", user_id)])
            .send()
            .await?
            .json()
            .await?;
        
        if !response["ok"].as_bool().unwrap_or(false) {
            return Err(SlackError::Api(
                response["error"].as_str().unwrap_or("Unknown error").to_string()
            ));
        }
        
        Ok(response["user"].clone())
    }
}

/// Slack sync service
pub struct SlackSyncService {
    client: SlackClient,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
}

impl SlackSyncService {
    pub fn new(client: SlackClient, db: Arc<Database>, crypto: Arc<CryptoService>) -> Self {
        Self { client, db, crypto }
    }
    
    /// Sync all enabled channels
    pub async fn sync_all(&self) -> Result<SyncResult, SlackError> {
        let channels = self.client.list_channels().await?;
        let mut total_items = 0;
        
        for channel in channels {
            // Get last sync cursor for this channel
            let cursor = self.get_sync_cursor(&channel.id).await?;
            
            // Fetch new messages
            let messages = self.client
                .get_channel_history(&channel.id, cursor.as_deref(), 100)
                .await?;
            
            // Store messages as content items
            for msg in &messages {
                self.store_message(&channel, msg).await?;
                total_items += 1;
                
                // Fetch thread replies if this is a thread parent
                if msg.reply_count.map(|c| c > 0).unwrap_or(false) {
                    if let Some(ref thread_ts) = msg.thread_ts {
                        let replies = self.client
                            .get_thread_replies(&channel.id, thread_ts)
                            .await?;
                        
                        for reply in &replies {
                            self.store_message(&channel, reply).await?;
                            total_items += 1;
                        }
                    }
                }
            }
            
            // Update sync cursor
            if let Some(last_msg) = messages.first() {
                self.update_sync_cursor(&channel.id, &last_msg.ts).await?;
            }
        }
        
        Ok(SyncResult {
            source: "slack".to_string(),
            items_synced: total_items,
            errors: vec![],
        })
    }
    
    async fn get_sync_cursor(&self, channel_id: &str) -> Result<Option<String>, SlackError> {
        let result: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT cursor FROM sync_state WHERE source = 'slack' AND resource_id = ?"
        )
        .bind(channel_id)
        .fetch_optional(self.db.pool())
        .await?;
        
        Ok(result.and_then(|r| r.0))
    }
    
    async fn update_sync_cursor(&self, channel_id: &str, cursor: &str) -> Result<(), SlackError> {
        let now = chrono::Utc::now().timestamp();
        
        sqlx::query(
            "INSERT INTO sync_state (id, source, resource_type, resource_id, last_sync_at, cursor, status)
             VALUES (?, 'slack', 'channel', ?, ?, ?, 'complete')
             ON CONFLICT(source, resource_type, resource_id) 
             DO UPDATE SET last_sync_at = ?, cursor = ?, status = 'complete'"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(channel_id)
        .bind(now)
        .bind(cursor)
        .bind(now)
        .bind(cursor)
        .execute(self.db.pool())
        .await?;
        
        Ok(())
    }
    
    async fn store_message(&self, channel: &SlackChannel, msg: &SlackMessage) -> Result<(), SlackError> {
        let now = chrono::Utc::now().timestamp();
        let ts_float: f64 = msg.ts.parse().unwrap_or(0.0);
        let created_at = (ts_float * 1000.0) as i64;
        
        // Encrypt message body
        let encrypted_body = self.crypto
            .encrypt_string(&msg.text)
            .map_err(|e| SlackError::Crypto(e.to_string()))?;
        
        let source_url = format!(
            "https://slack.com/app_redirect?channel={}&message_ts={}",
            channel.id, msg.ts
        );
        
        sqlx::query(
            "INSERT INTO content_items (id, source, source_id, source_url, content_type, title, body, author_id, channel_or_project, parent_id, created_at, updated_at, synced_at)
             VALUES (?, 'slack', ?, ?, 'message', NULL, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source, source_id) DO UPDATE SET body = ?, synced_at = ?"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&msg.ts)
        .bind(&source_url)
        .bind(&encrypted_body)
        .bind(&msg.user)
        .bind(&channel.name)
        .bind(&msg.thread_ts)
        .bind(created_at)
        .bind(created_at)
        .bind(now)
        .bind(&encrypted_body)
        .bind(now)
        .execute(self.db.pool())
        .await?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub source: String,
    pub items_synced: i32,
    pub errors: Vec<String>,
}
```

---

## 2.2 Atlassian Integration

### Register Atlassian App

1. Go to [developer.atlassian.com](https://developer.atlassian.com/console/myapps/)
2. Create a new OAuth 2.0 (3LO) app
3. Add callback URL: `http://localhost:8375/atlassian/callback`
4. Add the following scopes:
   - `read:jira-work` - Read Jira issues
   - `read:jira-user` - Read Jira users
   - `read:confluence-content.all` - Read Confluence content
   - `read:confluence-space.summary` - Read Confluence spaces
5. Copy your **Client ID** and **Client Secret**

### Atlassian Auth Module

Create `src-tauri/src/sync/atlassian.rs`:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::oneshot;
use tokio::net::TcpListener;
use rand::RngCore;

use crate::crypto::CryptoService;
use crate::db::Database;

const ATLASSIAN_AUTHORIZE_URL: &str = "https://auth.atlassian.com/authorize";
const ATLASSIAN_TOKEN_URL: &str = "https://auth.atlassian.com/oauth/token";
const ATLASSIAN_RESOURCES_URL: &str = "https://api.atlassian.com/oauth/token/accessible-resources";
const REDIRECT_PORT: u16 = 8375;

#[derive(Error, Debug)]
pub enum AtlassianError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("OAuth error: {0}")]
    OAuth(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Crypto error: {0}")]
    Crypto(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlassianTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudResource {
    pub id: String,
    pub name: String,
    pub url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub id: String,
    pub key: String,
    pub summary: String,
    pub description: Option<String>,
    pub status: String,
    pub assignee: Option<String>,
    pub reporter: String,
    pub project_key: String,
    pub created: String,
    pub updated: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfluencePage {
    pub id: String,
    pub title: String,
    pub space_key: String,
    pub body: Option<String>,
    pub author: String,
    pub created: String,
    pub updated: String,
    pub url: String,
}

pub struct AtlassianClient {
    http: Client,
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
    cloud_id: Option<String>,
}

impl AtlassianClient {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            http: Client::new(),
            client_id,
            client_secret,
            access_token: None,
            cloud_id: None,
        }
    }
    
    pub fn with_token(mut self, access_token: String, cloud_id: String) -> Self {
        self.access_token = Some(access_token);
        self.cloud_id = Some(cloud_id);
        self
    }
    
    /// Generate PKCE code verifier and challenge
    fn generate_pkce() -> (String, String) {
        let mut verifier_bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut verifier_bytes);
        let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
        
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
        
        (verifier, challenge)
    }
    
    /// Generate OAuth authorization URL with PKCE
    pub fn get_auth_url(&self, state: &str, code_challenge: &str) -> String {
        let scopes = [
            "read:jira-work",
            "read:jira-user", 
            "read:confluence-content.all",
            "read:confluence-space.summary",
            "offline_access",
        ].join(" ");
        
        format!(
            "{}?audience=api.atlassian.com&client_id={}&scope={}&redirect_uri=http://localhost:{}/atlassian/callback&state={}&response_type=code&prompt=consent&code_challenge={}&code_challenge_method=S256",
            ATLASSIAN_AUTHORIZE_URL,
            self.client_id,
            urlencoding::encode(&scopes),
            REDIRECT_PORT,
            state,
            code_challenge,
        )
    }
    
    /// Start OAuth flow with PKCE
    pub async fn start_oauth_flow(&self) -> Result<(AtlassianTokens, Vec<CloudResource>), AtlassianError> {
        let state = uuid::Uuid::new_v4().to_string();
        let (code_verifier, code_challenge) = Self::generate_pkce();
        let auth_url = self.get_auth_url(&state, &code_challenge);
        
        // Open browser
        open::that(&auth_url).map_err(|e| AtlassianError::OAuth(e.to_string()))?;
        
        // Start local server to receive callback
        let (tx, rx) = oneshot::channel();
        let expected_state = state.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::wait_for_callback(tx, expected_state).await {
                tracing::error!("OAuth callback error: {}", e);
            }
        });
        
        // Wait for authorization code
        let code = rx.await.map_err(|_| AtlassianError::OAuth("Callback cancelled".into()))?;
        
        // Exchange code for tokens
        let tokens = self.exchange_code(&code, &code_verifier).await?;
        
        // Get accessible resources
        let resources = self.get_accessible_resources(&tokens.access_token).await?;
        
        Ok((tokens, resources))
    }
    
    /// Wait for OAuth callback
    async fn wait_for_callback(tx: oneshot::Sender<String>, expected_state: String) -> Result<(), AtlassianError> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", REDIRECT_PORT)).await?;
        
        let (mut socket, _) = listener.accept().await?;
        
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        
        let mut buffer = [0; 2048];
        socket.read(&mut buffer).await?;
        
        let request = String::from_utf8_lossy(&buffer);
        
        if let Some(query_start) = request.find('?') {
            if let Some(query_end) = request[query_start..].find(' ') {
                let query = &request[query_start + 1..query_start + query_end];
                let params: HashMap<_, _> = query
                    .split('&')
                    .filter_map(|p| {
                        let mut parts = p.split('=');
                        Some((parts.next()?, parts.next()?))
                    })
                    .collect();
                
                if let (Some(code), Some(state)) = (params.get("code"), params.get("state")) {
                    if *state == expected_state {
                        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authorization successful!</h1><p>You can close this window.</p></body></html>";
                        socket.write_all(response.as_bytes()).await?;
                        
                        let _ = tx.send(code.to_string());
                        return Ok(());
                    }
                }
            }
        }
        
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authorization failed</h1></body></html>";
        socket.write_all(response.as_bytes()).await?;
        
        Err(AtlassianError::OAuth("Invalid callback".into()))
    }
    
    /// Exchange authorization code for tokens
    async fn exchange_code(&self, code: &str, code_verifier: &str) -> Result<AtlassianTokens, AtlassianError> {
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: i64,
            scope: String,
        }
        
        let response: TokenResponse = self.http
            .post(ATLASSIAN_TOKEN_URL)
            .json(&serde_json::json!({
                "grant_type": "authorization_code",
                "client_id": self.client_id,
                "client_secret": self.client_secret,
                "code": code,
                "redirect_uri": format!("http://localhost:{}/atlassian/callback", REDIRECT_PORT),
                "code_verifier": code_verifier,
            }))
            .send()
            .await?
            .json()
            .await?;
        
        Ok(AtlassianTokens {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_in: response.expires_in,
            scope: response.scope,
        })
    }
    
    /// Get accessible cloud resources
    async fn get_accessible_resources(&self, access_token: &str) -> Result<Vec<CloudResource>, AtlassianError> {
        #[derive(Deserialize)]
        struct ResourceResponse {
            id: String,
            name: String,
            url: String,
            scopes: Vec<String>,
        }
        
        let resources: Vec<ResourceResponse> = self.http
            .get(ATLASSIAN_RESOURCES_URL)
            .bearer_auth(access_token)
            .send()
            .await?
            .json()
            .await?;
        
        Ok(resources.into_iter().map(|r| CloudResource {
            id: r.id,
            name: r.name,
            url: r.url,
            scopes: r.scopes,
        }).collect())
    }
    
    /// Search Jira issues using JQL
    pub async fn search_issues(&self, jql: &str, start_at: i32, max_results: i32) -> Result<Vec<JiraIssue>, AtlassianError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| AtlassianError::OAuth("Not authenticated".into()))?;
        let cloud_id = self.cloud_id.as_ref()
            .ok_or_else(|| AtlassianError::OAuth("No cloud instance selected".into()))?;
        
        let url = format!(
            "https://api.atlassian.com/ex/jira/{}/rest/api/3/search",
            cloud_id
        );
        
        let response: serde_json::Value = self.http
            .get(&url)
            .bearer_auth(token)
            .query(&[
                ("jql", jql),
                ("startAt", &start_at.to_string()),
                ("maxResults", &max_results.to_string()),
                ("fields", "summary,description,status,assignee,reporter,project,created,updated"),
            ])
            .send()
            .await?
            .json()
            .await?;
        
        let issues = response["issues"]
            .as_array()
            .map(|issues| {
                issues.iter().filter_map(|i| {
                    let fields = &i["fields"];
                    Some(JiraIssue {
                        id: i["id"].as_str()?.to_string(),
                        key: i["key"].as_str()?.to_string(),
                        summary: fields["summary"].as_str().unwrap_or_default().to_string(),
                        description: fields["description"]["content"][0]["content"][0]["text"]
                            .as_str()
                            .map(String::from),
                        status: fields["status"]["name"].as_str().unwrap_or_default().to_string(),
                        assignee: fields["assignee"]["displayName"].as_str().map(String::from),
                        reporter: fields["reporter"]["displayName"].as_str().unwrap_or_default().to_string(),
                        project_key: fields["project"]["key"].as_str().unwrap_or_default().to_string(),
                        created: fields["created"].as_str().unwrap_or_default().to_string(),
                        updated: fields["updated"].as_str().unwrap_or_default().to_string(),
                        url: format!(
                            "https://{}.atlassian.net/browse/{}",
                            self.cloud_id.as_ref().unwrap_or(&"".to_string()),
                            i["key"].as_str().unwrap_or_default()
                        ),
                    })
                }).collect()
            })
            .unwrap_or_default();
        
        Ok(issues)
    }
    
    /// Search Confluence pages using CQL
    pub async fn search_pages(&self, cql: &str, start: i32, limit: i32) -> Result<Vec<ConfluencePage>, AtlassianError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| AtlassianError::OAuth("Not authenticated".into()))?;
        let cloud_id = self.cloud_id.as_ref()
            .ok_or_else(|| AtlassianError::OAuth("No cloud instance selected".into()))?;
        
        let url = format!(
            "https://api.atlassian.com/ex/confluence/{}/wiki/rest/api/content/search",
            cloud_id
        );
        
        let response: serde_json::Value = self.http
            .get(&url)
            .bearer_auth(token)
            .query(&[
                ("cql", cql),
                ("start", &start.to_string()),
                ("limit", &limit.to_string()),
                ("expand", "body.storage,space,version"),
            ])
            .send()
            .await?
            .json()
            .await?;
        
        let pages = response["results"]
            .as_array()
            .map(|pages| {
                pages.iter().filter_map(|p| {
                    Some(ConfluencePage {
                        id: p["id"].as_str()?.to_string(),
                        title: p["title"].as_str().unwrap_or_default().to_string(),
                        space_key: p["space"]["key"].as_str().unwrap_or_default().to_string(),
                        body: p["body"]["storage"]["value"].as_str().map(String::from),
                        author: p["version"]["by"]["displayName"].as_str().unwrap_or_default().to_string(),
                        created: p["version"]["when"].as_str().unwrap_or_default().to_string(),
                        updated: p["version"]["when"].as_str().unwrap_or_default().to_string(),
                        url: format!(
                            "https://{}.atlassian.net/wiki{}",
                            self.cloud_id.as_ref().unwrap_or(&"".to_string()),
                            p["_links"]["webui"].as_str().unwrap_or_default()
                        ),
                    })
                }).collect()
            })
            .unwrap_or_default();
        
        Ok(pages)
    }
}

/// Atlassian sync service
pub struct AtlassianSyncService {
    client: AtlassianClient,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
}

impl AtlassianSyncService {
    pub fn new(client: AtlassianClient, db: Arc<Database>, crypto: Arc<CryptoService>) -> Self {
        Self { client, db, crypto }
    }
    
    /// Sync Jira issues updated in the last N days
    pub async fn sync_jira(&self, days: i32) -> Result<i32, AtlassianError> {
        let jql = format!("updated >= -{}d ORDER BY updated DESC", days);
        let mut total = 0;
        let mut start_at = 0;
        
        loop {
            let issues = self.client.search_issues(&jql, start_at, 50).await?;
            
            if issues.is_empty() {
                break;
            }
            
            for issue in &issues {
                self.store_jira_issue(issue).await?;
                total += 1;
            }
            
            start_at += 50;
        }
        
        Ok(total)
    }
    
    /// Sync Confluence pages updated in the last N days
    pub async fn sync_confluence(&self, days: i32) -> Result<i32, AtlassianError> {
        let cql = format!("lastModified >= now('-{}d') ORDER BY lastModified DESC", days);
        let mut total = 0;
        let mut start = 0;
        
        loop {
            let pages = self.client.search_pages(&cql, start, 25).await?;
            
            if pages.is_empty() {
                break;
            }
            
            for page in &pages {
                self.store_confluence_page(page).await?;
                total += 1;
            }
            
            start += 25;
        }
        
        Ok(total)
    }
    
    async fn store_jira_issue(&self, issue: &JiraIssue) -> Result<(), AtlassianError> {
        let now = chrono::Utc::now().timestamp();
        
        let description = issue.description.as_deref().unwrap_or("");
        let encrypted_body = self.crypto
            .encrypt_string(description)
            .map_err(|e| AtlassianError::Crypto(e.to_string()))?;
        
        sqlx::query(
            "INSERT INTO content_items (id, source, source_id, source_url, content_type, title, body, author, channel_or_project, created_at, updated_at, synced_at)
             VALUES (?, 'jira', ?, ?, 'ticket', ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source, source_id) DO UPDATE SET title = ?, body = ?, synced_at = ?"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&issue.key)
        .bind(&issue.url)
        .bind(&issue.summary)
        .bind(&encrypted_body)
        .bind(&issue.reporter)
        .bind(&issue.project_key)
        .bind(now) // TODO: Parse actual dates
        .bind(now)
        .bind(now)
        .bind(&issue.summary)
        .bind(&encrypted_body)
        .bind(now)
        .execute(self.db.pool())
        .await?;
        
        Ok(())
    }
    
    async fn store_confluence_page(&self, page: &ConfluencePage) -> Result<(), AtlassianError> {
        let now = chrono::Utc::now().timestamp();
        
        let body = page.body.as_deref().unwrap_or("");
        let encrypted_body = self.crypto
            .encrypt_string(body)
            .map_err(|e| AtlassianError::Crypto(e.to_string()))?;
        
        sqlx::query(
            "INSERT INTO content_items (id, source, source_id, source_url, content_type, title, body, author, channel_or_project, created_at, updated_at, synced_at)
             VALUES (?, 'confluence', ?, ?, 'page', ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source, source_id) DO UPDATE SET title = ?, body = ?, synced_at = ?"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&page.id)
        .bind(&page.url)
        .bind(&page.title)
        .bind(&encrypted_body)
        .bind(&page.author)
        .bind(&page.space_key)
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(&page.title)
        .bind(&encrypted_body)
        .bind(now)
        .execute(self.db.pool())
        .await?;
        
        Ok(())
    }
}
```

---

## 2.3 Update Module Exports

Update `src-tauri/src/sync/mod.rs`:

```rust
pub mod slack;
pub mod atlassian;

pub use slack::{SlackClient, SlackSyncService, SlackTokens, SyncResult};
pub use atlassian::{AtlassianClient, AtlassianSyncService, AtlassianTokens, CloudResource};
```

---

## 2.4 Background Sync Service

Add a background sync coordinator. Create `src-tauri/src/sync/background.rs`:

```rust
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tauri::{AppHandle, Manager};
use serde::Serialize;

use super::{SlackClient, SlackSyncService, AtlassianClient, AtlassianSyncService};
use crate::db::Database;
use crate::crypto::CryptoService;

#[derive(Debug, Clone, Serialize)]
pub struct SyncProgress {
    pub source: String,
    pub status: String,
    pub items_synced: i32,
    pub total_items: Option<i32>,
    pub error: Option<String>,
}

pub struct BackgroundSyncService {
    app_handle: AppHandle,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
    interval_minutes: u64,
    is_running: Arc<Mutex<bool>>,
}

impl BackgroundSyncService {
    pub fn new(
        app_handle: AppHandle,
        db: Arc<Database>,
        crypto: Arc<CryptoService>,
        interval_minutes: u64,
    ) -> Self {
        Self {
            app_handle,
            db,
            crypto,
            interval_minutes,
            is_running: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Start the background sync loop
    pub async fn start(&self) {
        let mut is_running = self.is_running.lock().await;
        if *is_running {
            return;
        }
        *is_running = true;
        drop(is_running);
        
        let app_handle = self.app_handle.clone();
        let db = self.db.clone();
        let crypto = self.crypto.clone();
        let interval = self.interval_minutes;
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            loop {
                // Check if still running
                let running = is_running.lock().await;
                if !*running {
                    break;
                }
                drop(running);
                
                // Run sync
                Self::run_sync_cycle(&app_handle, &db, &crypto).await;
                
                // Wait for next interval
                tokio::time::sleep(Duration::from_secs(interval * 60)).await;
            }
        });
    }
    
    /// Stop the background sync loop
    pub async fn stop(&self) {
        let mut is_running = self.is_running.lock().await;
        *is_running = false;
    }
    
    /// Run a single sync cycle
    async fn run_sync_cycle(app_handle: &AppHandle, db: &Database, crypto: &CryptoService) {
        tracing::info!("Starting sync cycle");
        
        // Emit start event
        let _ = app_handle.emit("sync:started", ());
        
        // TODO: Load credentials and create sync services
        // For now, just emit completion
        
        let _ = app_handle.emit("sync:completed", serde_json::json!({
            "items_synced": 0,
            "duration_ms": 0,
        }));
        
        tracing::info!("Sync cycle completed");
    }
    
    /// Emit sync progress event
    fn emit_progress(app_handle: &AppHandle, progress: SyncProgress) {
        let _ = app_handle.emit("sync:progress", progress);
    }
}
```

Update `src-tauri/src/sync/mod.rs`:

```rust
pub mod slack;
pub mod atlassian;
pub mod background;

pub use slack::{SlackClient, SlackSyncService, SlackTokens, SyncResult};
pub use atlassian::{AtlassianClient, AtlassianSyncService, AtlassianTokens, CloudResource};
pub use background::{BackgroundSyncService, SyncProgress};
```

---

## 2.5 Add Sync Commands

Add new commands to `src-tauri/src/commands/mod.rs`:

```rust
// Add these new commands

#[tauri::command]
pub async fn connect_slack(
    state: State<'_, Arc<Mutex<AppState>>>,
    client_id: String,
    client_secret: String,
) -> Result<SlackTokens, String> {
    use crate::sync::SlackClient;
    
    let client = SlackClient::new(client_id, client_secret);
    let tokens = client.start_oauth_flow().await.map_err(|e| e.to_string())?;
    
    // Store tokens
    let state = state.lock().await;
    let encrypted = state.crypto
        .encrypt_string(&serde_json::to_string(&tokens).unwrap())
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at)
         VALUES ('slack', 'slack', ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET encrypted_data = ?, updated_at = ?"
    )
    .bind(&encrypted)
    .bind(now)
    .bind(now)
    .bind(&encrypted)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    Ok(tokens)
}

#[tauri::command]
pub async fn connect_atlassian(
    state: State<'_, Arc<Mutex<AppState>>>,
    client_id: String,
    client_secret: String,
) -> Result<(AtlassianTokens, Vec<CloudResource>), String> {
    use crate::sync::AtlassianClient;
    
    let client = AtlassianClient::new(client_id, client_secret);
    let (tokens, resources) = client.start_oauth_flow().await.map_err(|e| e.to_string())?;
    
    // Store tokens
    let state = state.lock().await;
    let encrypted = state.crypto
        .encrypt_string(&serde_json::to_string(&tokens).unwrap())
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at)
         VALUES ('atlassian', 'atlassian', ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET encrypted_data = ?, updated_at = ?"
    )
    .bind(&encrypted)
    .bind(now)
    .bind(now)
    .bind(&encrypted)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    Ok((tokens, resources))
}

// Update the invoke_handler in main.rs to include these new commands
```

---

## Verification

Test the OAuth flows:

1. **Slack OAuth**:
   ```typescript
   // From frontend console
   await invoke('connect_slack', { 
     clientId: 'your-client-id', 
     clientSecret: 'your-client-secret' 
   });
   ```

2. **Atlassian OAuth**:
   ```typescript
   await invoke('connect_atlassian', { 
     clientId: 'your-client-id', 
     clientSecret: 'your-client-secret' 
   });
   ```

### Checklist

- [ ] Slack OAuth flow opens browser and captures callback
- [ ] Slack tokens are encrypted and stored
- [ ] Can fetch Slack channels and messages
- [ ] Atlassian OAuth flow with PKCE works
- [ ] Can list accessible Atlassian cloud resources
- [ ] Can fetch Jira issues and Confluence pages
- [ ] Messages/issues are stored encrypted in SQLite
- [ ] Sync progress events emit to frontend

---

## Next Steps

Proceed to **Phase 3: AI Processing** to implement Gemini integration and the summarization pipeline.
