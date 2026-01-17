//! Slack synchronization service

use std::sync::Arc;
use chrono::Utc;
use super::client::SlackClient;
use super::types::{SlackError, SlackChannel, SlackMessage, SyncResult, SlackChannelSelection};
use crate::crypto::CryptoService;
use crate::db::Database;

type ChannelRow = (String, String, i32, i32, i32, String, Option<i32>, Option<String>, i32);

pub struct SlackSyncService {
    client: SlackClient,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
}

impl SlackSyncService {
    pub fn new(client: SlackClient, db: Arc<Database>, crypto: Arc<CryptoService>) -> Self {
        Self { client, db, crypto }
    }
    
    fn get_today_start_timestamp() -> f64 {
        let today = Utc::now().date_naive();
        let start_of_day = today.and_hms_opt(0, 0, 0).unwrap();
        start_of_day.and_utc().timestamp() as f64
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
    
    /// Sync only selected and enabled channels
    pub async fn sync_all(&self) -> Result<SyncResult, SlackError> {
        let selected_channels = self.get_enabled_channels().await?;
        tracing::debug!("Found {} enabled channels to sync", selected_channels.len());
        
        let mut total_items = 0;
        let mut errors = Vec::new();
        
        if selected_channels.is_empty() {
            tracing::info!("No channels selected for sync, skipping Slack sync");
            return Ok(SyncResult {
                source: "slack".to_string(),
                items_synced: 0,
                errors: vec![],
            });
        }
        
        let today_start = Self::get_today_start_timestamp();
        
        for channel in selected_channels {
            match self.sync_channel(&channel, today_start).await {
                Ok(count) => {
                    total_items += count;
                }
                Err(e) => {
                    tracing::error!("Error syncing channel {}: {}", channel.channel_name, e);
                    errors.push(format!("{}: {}", channel.channel_name, e));
                }
            }
        }
        
        Ok(SyncResult {
            source: "slack".to_string(),
            items_synced: total_items,
            errors,
        })
    }
    
    /// Sync a single channel with date-aware cursor logic
    async fn sync_channel(&self, channel: &SlackChannelSelection, _today_start: f64) -> Result<i32, SlackError> {
        tracing::debug!("Syncing channel: {} ({})", channel.channel_name, channel.channel_id);
        
        let mut items_synced = 0;
        
        // For now, always fetch recent messages without cursor filter
        // TODO: Restore cursor logic for incremental sync
        let oldest: Option<String> = None;
        
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
        
        let messages = self.client
            .get_channel_history(&channel.channel_id, oldest.as_deref(), 100)
            .await?;
        tracing::debug!("Got {} messages from channel {}", messages.len(), channel.channel_name);
        
        for msg in &messages {
            self.store_message(&slack_channel, msg).await?;
            items_synced += 1;
            
            // Fetch thread replies if this is a parent message with replies
            if msg.reply_count.map(|c| c > 0).unwrap_or(false) {
                if let Some(ref thread_ts) = msg.thread_ts {
                    let replies = self.client
                        .get_thread_replies(&channel.channel_id, thread_ts)
                        .await?;
                    
                    for reply in &replies {
                        self.store_message(&slack_channel, reply).await?;
                        items_synced += 1;
                    }
                }
            }
        }
        
        // Update cursor to latest message timestamp
        if let Some(last_msg) = messages.first() {
            self.update_sync_cursor(&channel.channel_id, &last_msg.ts).await?;
        }
        
        tracing::debug!("Synced {} messages from channel {}", items_synced, channel.channel_name);
        Ok(items_synced)
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
