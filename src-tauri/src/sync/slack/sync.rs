//! Slack synchronization service

use std::sync::Arc;
use super::client::SlackClient;
use super::types::{SlackError, SlackChannel, SlackMessage, SyncResult};
use crate::crypto::CryptoService;
use crate::db::Database;

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
            let cursor = self.get_sync_cursor(&channel.id).await?;
            let messages = self.client
                .get_channel_history(&channel.id, cursor.as_deref(), 100)
                .await?;
            
            for msg in &messages {
                self.store_message(&channel, msg).await?;
                total_items += 1;
                
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
        .bind(created_at)
        .bind(now)
        .bind(&encrypted_body)
        .bind(now)
        .execute(self.db.pool())
        .await?;
        
        Ok(())
    }
}
