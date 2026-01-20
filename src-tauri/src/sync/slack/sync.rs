//! Slack synchronization service

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use super::client::SlackClient;
use super::types::{SlackError, SlackChannel, SlackMessage, SlackUser, SyncResult, SlackChannelSelection};
use crate::crypto::CryptoService;
use crate::db::Database;

type ChannelRow = (String, String, i32, i32, i32, String, Option<i32>, Option<String>, i32);

const USER_CACHE_TTL_MS: i64 = 24 * 60 * 60 * 1000; // 24 hours
const MAX_CONCURRENT_SYNCS: usize = 2;
const API_CALL_DELAY_MS: u64 = 500;
const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 2000;

#[derive(Clone)]
pub struct SlackSyncService {
    client: SlackClient,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
}

impl SlackSyncService {
    pub fn new(client: SlackClient, db: Arc<Database>, crypto: Arc<CryptoService>) -> Self {
        Self { client, db, crypto }
    }
    
    async fn get_sync_cursor(&self, channel_id: &str) -> Result<Option<String>, SlackError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT cursor FROM sync_state 
             WHERE source = 'slack' AND resource_type = 'channel' AND resource_id = ?"
        )
        .bind(channel_id)
        .fetch_optional(self.db.pool())
        .await?;
        
        Ok(row.map(|r| r.0))
    }
    
    async fn should_refresh_user_cache(&self) -> Result<bool, SlackError> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT MAX(updated_at) FROM slack_users"
        )
        .fetch_optional(self.db.pool())
        .await?;
        
        let now = chrono::Utc::now().timestamp_millis();
        match row {
            Some((last_update,)) => Ok(now - last_update > USER_CACHE_TTL_MS),
            None => Ok(true), // No users cached yet
        }
    }
    
    async fn store_users(&self, users: &[SlackUser], team_id: &str) -> Result<(), SlackError> {
        let now = chrono::Utc::now().timestamp_millis();
        
        for user in users {
            sqlx::query(
                "INSERT INTO slack_users (user_id, team_id, username, real_name, display_name, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?)
                 ON CONFLICT(user_id) DO UPDATE SET 
                    username = excluded.username,
                    real_name = excluded.real_name,
                    display_name = excluded.display_name,
                    updated_at = excluded.updated_at"
            )
            .bind(&user.id)
            .bind(team_id)
            .bind(&user.name)
            .bind(&user.real_name)
            .bind(&user.display_name)
            .bind(now)
            .execute(self.db.pool())
            .await?;
        }
        
        Ok(())
    }
    
    async fn get_team_id(&self) -> Result<Option<String>, SlackError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT team_id FROM slack_selected_channels WHERE enabled = 1 LIMIT 1"
        )
        .fetch_optional(self.db.pool())
        .await?;
        
        Ok(row.map(|r| r.0))
    }
    
    async fn get_enabled_channels(&self) -> Result<Vec<SlackChannelSelection>, SlackError> {
        let rows: Vec<ChannelRow> = sqlx::query_as(
            "SELECT channel_id, channel_name, is_private, is_im, is_mpim, team_id, member_count, purpose, enabled 
             FROM slack_selected_channels WHERE enabled = 1"
        )
        .fetch_all(self.db.pool())
        .await?;
        
        let channels: Vec<SlackChannelSelection> = rows.into_iter().map(|row| {
            SlackChannelSelection {
                channel_id: row.0,
                channel_name: row.1,
                is_private: row.2 != 0,
                is_im: row.3 != 0,
                is_mpim: row.4 != 0,
                team_id: row.5,
                member_count: row.6,
                purpose: row.7,
                enabled: row.8 != 0,
            }
        }).collect();
        
        Ok(channels)
    }
    
    pub async fn sync_all(&self) -> Result<SyncResult, SlackError> {
        let selected_channels = self.get_enabled_channels().await?;
        tracing::debug!("Found {} enabled channels to sync", selected_channels.len());
        
        let mut errors = Vec::new();
        
        if selected_channels.is_empty() {
            tracing::info!("No channels selected for sync, skipping Slack sync");
            return Ok(SyncResult {
                source: "slack".to_string(),
                items_synced: 0,
                errors: vec![],
            });
        }
        
        if self.should_refresh_user_cache().await? {
            if let Some(team_id) = self.get_team_id().await? {
                tracing::info!("Refreshing Slack user cache");
                match self.client.list_users().await {
                    Ok(users) => {
                        tracing::info!("Fetched {} users from Slack", users.len());
                        if let Err(e) = self.store_users(&users, &team_id).await {
                            tracing::error!("Failed to store users: {}", e);
                            errors.push(format!("User cache: {}", e));
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch users: {}", e);
                        errors.push(format!("User fetch: {}", e));
                    }
                }
            }
        }
        
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_SYNCS));
        let mut handles = Vec::new();
        
        for channel in selected_channels {
            let sem = semaphore.clone();
            let service = self.clone();
            
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("Semaphore closed unexpectedly");
                let result = service.sync_channel(&channel).await;
                (channel.channel_name.clone(), result)
            }));
        }
        
        let mut total_items = 0;
        for handle in handles {
            match handle.await {
                Ok((_channel_name, Ok(count))) => {
                    total_items += count;
                }
                Ok((channel_name, Err(e))) => {
                    tracing::error!("Error syncing channel {}: {}", channel_name, e);
                    errors.push(format!("{}: {}", channel_name, e));
                }
                Err(e) => {
                    tracing::error!("Task join error: {}", e);
                    errors.push(format!("Task error: {}", e));
                }
            }
        }
        
        Ok(SyncResult {
            source: "slack".to_string(),
            items_synced: total_items,
            errors,
        })
    }
    
    async fn sync_channel(&self, channel: &SlackChannelSelection) -> Result<i32, SlackError> {
        tracing::debug!("Syncing channel: {} ({})", channel.channel_name, channel.channel_id);
        
        let mut items_synced = 0;
        
        let oldest = self.get_sync_cursor(&channel.channel_id).await?;
        let mut newest_ts: Option<String> = None;
        let mut api_cursor: Option<String> = None;
        
        let slack_channel = SlackChannel {
            id: channel.channel_id.clone(),
            name: channel.channel_name.clone(),
            is_private: channel.is_private,
            is_im: channel.is_im,
            is_mpim: channel.is_mpim,
            user: None, // Not needed for sync, only for display
            member_count: channel.member_count,
            purpose: channel.purpose.clone(),
            topic: None,
        };
        
        loop {
            let response = self.fetch_with_retry(|| async {
                self.client
                    .get_channel_history(
                        &channel.channel_id,
                        oldest.as_deref(),
                        api_cursor.as_deref(),
                        100,
                    )
                    .await
            }).await?;
            
            tracing::debug!(
                "Got {} messages from channel {} (has_more: {})",
                response.messages.len(),
                channel.channel_name,
                response.has_more
            );
            
            // Slack returns messages in reverse chronological order
            if newest_ts.is_none() {
                newest_ts = response.messages.first().map(|m| m.ts.clone());
            }
            
            for msg in &response.messages {
                self.store_message(&slack_channel, msg).await?;
                items_synced += 1;
                
                if msg.reply_count.map(|c| c > 0).unwrap_or(false) {
                    sleep(Duration::from_millis(API_CALL_DELAY_MS)).await;
                    let thread_ts = msg.thread_ts.as_ref().unwrap_or(&msg.ts);
                    let replies = self.fetch_with_retry(|| async {
                        self.client
                            .get_thread_replies(&channel.channel_id, thread_ts)
                            .await
                    }).await?;
                    
                    for reply in replies.iter().skip(1) {
                        self.store_message(&slack_channel, reply).await?;
                        items_synced += 1;
                    }
                }
            }
            
            if !response.has_more {
                break;
            }
            
            api_cursor = response.next_cursor;
            if api_cursor.is_none() {
                break;
            }
            
            sleep(Duration::from_millis(API_CALL_DELAY_MS)).await;
        }
        
        if let Some(ts) = newest_ts {
            self.update_sync_cursor(&channel.channel_id, &ts).await?;
        }
        
        tracing::debug!("Synced {} messages from channel {}", items_synced, channel.channel_name);
        Ok(items_synced)
    }
    
    async fn fetch_with_retry<F, Fut, T>(&self, f: F) -> Result<T, SlackError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, SlackError>>,
    {
        let mut retries = 0;
        
        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(SlackError::Api(ref msg)) if msg.contains("429") && retries < MAX_RETRIES => {
                    retries += 1;
                    let delay = RETRY_BASE_DELAY_MS * (1 << retries); // Exponential backoff
                    tracing::warn!(
                        "Rate limited (429), retry {}/{} after {}ms",
                        retries,
                        MAX_RETRIES,
                        delay
                    );
                    sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    async fn update_sync_cursor(&self, channel_id: &str, cursor: &str) -> Result<(), SlackError> {
        let now = chrono::Utc::now().timestamp_millis();
        
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
        let now = chrono::Utc::now().timestamp_millis();
        let ts_float: f64 = msg.ts.parse().unwrap_or(0.0);
        let created_at = (ts_float * 1000.0) as i64;
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
        .bind(now)
        .bind(now)
        .bind(&encrypted_body)
        .bind(now)
        .execute(self.db.pool())
        .await?;
        
        Ok(())
    }
}
