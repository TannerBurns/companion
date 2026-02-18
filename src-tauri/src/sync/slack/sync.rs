//! Slack synchronization service

use super::client::SlackClient;
use super::types::{
    SlackChannel, SlackChannelSelection, SlackError, SlackMessage, SlackUser, SyncResult,
};
use crate::crypto::CryptoService;
use crate::db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;

type ChannelRow = (
    String,
    String,
    i32,
    i32,
    i32,
    String,
    Option<i32>,
    Option<String>,
    i32,
);

const USER_CACHE_TTL_MS: i64 = 24 * 60 * 60 * 1000; // 24 hours
const MAX_CONCURRENT_SYNCS: usize = 2;
const API_CALL_DELAY_MS: u64 = 500;
const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 2000;

fn get_today_start_ts() -> String {
    let now = chrono::Utc::now();
    let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let ts = today_start.and_utc().timestamp();
    format!("{}.000000", ts)
}

#[derive(Clone)]
pub struct SlackSyncService {
    client: SlackClient,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
    team_domain: Option<String>,
}

impl SlackSyncService {
    pub fn new(client: SlackClient, db: Arc<Database>, crypto: Arc<CryptoService>) -> Self {
        Self {
            client,
            db,
            crypto,
            team_domain: None,
        }
    }

    pub fn with_team_domain(mut self, domain: Option<String>) -> Self {
        self.team_domain = domain;
        self
    }

    async fn get_sync_cursor(&self, channel_id: &str) -> Result<Option<String>, SlackError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT cursor FROM sync_state 
             WHERE source = 'slack' AND resource_type = 'channel' AND resource_id = ?",
        )
        .bind(channel_id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| r.0))
    }

    async fn should_refresh_user_cache(&self) -> Result<bool, SlackError> {
        let row: Option<(i64,)> = sqlx::query_as("SELECT MAX(updated_at) FROM slack_users")
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
        let row: Option<(String,)> =
            sqlx::query_as("SELECT team_id FROM slack_selected_channels WHERE enabled = 1 LIMIT 1")
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

        let channels: Vec<SlackChannelSelection> = rows
            .into_iter()
            .map(|row| SlackChannelSelection {
                channel_id: row.0,
                channel_name: row.1,
                is_private: row.2 != 0,
                is_im: row.3 != 0,
                is_mpim: row.4 != 0,
                team_id: row.5,
                member_count: row.6,
                purpose: row.7,
                enabled: row.8 != 0,
            })
            .collect();

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
        tracing::debug!(
            "Syncing channel: {} ({})",
            channel.channel_name,
            channel.channel_id
        );

        let mut items_synced = 0;

        let oldest = self
            .get_sync_cursor(&channel.channel_id)
            .await?
            .or_else(|| Some(get_today_start_ts()));
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
            let response = self
                .fetch_with_retry(|| async {
                    self.client
                        .get_channel_history(
                            &channel.channel_id,
                            oldest.as_deref(),
                            None, // No upper bound for incremental sync
                            api_cursor.as_deref(),
                            100,
                        )
                        .await
                })
                .await?;

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
                    let replies = self
                        .fetch_with_retry(|| async {
                            self.client
                                .get_thread_replies(&channel.channel_id, thread_ts)
                                .await
                        })
                        .await?;

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

        tracing::debug!(
            "Synced {} messages from channel {}",
            items_synced,
            channel.channel_name
        );
        Ok(items_synced)
    }

    /// Sync messages for a specific historical date. Does not update the sync cursor.
    pub async fn sync_historical_day(
        &self,
        date_str: &str,
        timezone_offset_minutes: i32,
    ) -> Result<SyncResult, SlackError> {
        let target_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| SlackError::Api(format!("Invalid date format: {}", e)))?;

        let offset = chrono::FixedOffset::west_opt(timezone_offset_minutes * 60)
            .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());

        let local_midnight = target_date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| SlackError::Api("Invalid date".to_string()))?
            .and_local_timezone(offset)
            .single()
            .ok_or_else(|| SlackError::Api("Ambiguous or invalid local time".to_string()))?;

        let start_ts_secs = local_midnight.with_timezone(&chrono::Utc).timestamp();
        let end_ts_secs = start_ts_secs + 86400;

        // Slack API uses exclusive bounds
        let oldest_ts = format!("{}.999999", start_ts_secs - 1);
        let latest_ts = format!("{}.000000", end_ts_secs);

        tracing::info!(
            "Syncing historical day {} (offset {}min): oldest={}, latest={}",
            date_str,
            timezone_offset_minutes,
            oldest_ts,
            latest_ts
        );

        let selected_channels = self.get_enabled_channels().await?;

        if selected_channels.is_empty() {
            tracing::info!("No channels selected for sync");
            return Ok(SyncResult {
                source: "slack".to_string(),
                items_synced: 0,
                errors: vec![],
            });
        }

        let mut total_items = 0;
        let mut errors = Vec::new();

        for channel in selected_channels {
            match self
                .sync_channel_range(&channel, &oldest_ts, &latest_ts)
                .await
            {
                Ok(count) => {
                    total_items += count;
                }
                Err(e) => {
                    tracing::error!(
                        "Error syncing channel {} for {}: {}",
                        channel.channel_name,
                        date_str,
                        e
                    );
                    errors.push(format!("{}: {}", channel.channel_name, e));
                }
            }

            sleep(Duration::from_millis(API_CALL_DELAY_MS)).await;
        }

        tracing::info!(
            "Historical sync for {} complete: {} items",
            date_str,
            total_items
        );

        Ok(SyncResult {
            source: "slack".to_string(),
            items_synced: total_items,
            errors,
        })
    }

    /// Sync a channel within a specific timestamp range. Does not update the sync cursor.
    async fn sync_channel_range(
        &self,
        channel: &SlackChannelSelection,
        oldest: &str,
        latest: &str,
    ) -> Result<i32, SlackError> {
        tracing::debug!(
            "Syncing channel {} ({}) range: {} to {}",
            channel.channel_name,
            channel.channel_id,
            oldest,
            latest
        );

        let mut items_synced = 0;
        let mut api_cursor: Option<String> = None;

        let slack_channel = SlackChannel {
            id: channel.channel_id.clone(),
            name: channel.channel_name.clone(),
            is_private: channel.is_private,
            is_im: channel.is_im,
            is_mpim: channel.is_mpim,
            user: None,
            member_count: channel.member_count,
            purpose: channel.purpose.clone(),
            topic: None,
        };

        loop {
            let response = self
                .fetch_with_retry(|| async {
                    self.client
                        .get_channel_history(
                            &channel.channel_id,
                            Some(oldest),
                            Some(latest),
                            api_cursor.as_deref(),
                            100,
                        )
                        .await
                })
                .await?;

            tracing::debug!(
                "Got {} messages from channel {} in range (has_more: {})",
                response.messages.len(),
                channel.channel_name,
                response.has_more
            );

            for msg in &response.messages {
                self.store_message(&slack_channel, msg).await?;
                items_synced += 1;

                if msg.reply_count.map(|c| c > 0).unwrap_or(false) {
                    sleep(Duration::from_millis(API_CALL_DELAY_MS)).await;
                    let thread_ts = msg.thread_ts.as_ref().unwrap_or(&msg.ts);
                    let replies = self
                        .fetch_with_retry(|| async {
                            self.client
                                .get_thread_replies(&channel.channel_id, thread_ts)
                                .await
                        })
                        .await?;

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

        tracing::debug!(
            "Synced {} messages from channel {} in range",
            items_synced,
            channel.channel_name
        );
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

    async fn store_message(
        &self,
        channel: &SlackChannel,
        msg: &SlackMessage,
    ) -> Result<(), SlackError> {
        let now = chrono::Utc::now().timestamp_millis();
        let ts_float: f64 = msg.ts.parse().unwrap_or(0.0);
        let created_at = (ts_float * 1000.0) as i64;
        let encrypted_body = self
            .crypto
            .encrypt_string(&msg.text)
            .map_err(|e| SlackError::Crypto(e.to_string()))?;

        let source_url = if let Some(ref domain) = self.team_domain {
            // e.g., https://acme-corp.slack.com/archives/C04KQBBPPLN/p1769203754053419
            let permalink_ts = format!("p{}", msg.ts.replace('.', ""));
            format!(
                "https://{}.slack.com/archives/{}/{}",
                domain, channel.id, permalink_ts
            )
        } else {
            format!(
                "https://slack.com/app_redirect?channel={}&message_ts={}",
                channel.id, msg.ts
            )
        };

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_get_today_start_ts_format() {
        let ts = get_today_start_ts();
        assert!(
            ts.ends_with(".000000"),
            "should end with .000000, got: {}",
            ts
        );
    }

    #[test]
    fn test_get_today_start_ts_is_parseable() {
        let ts = get_today_start_ts();
        let parsed: f64 = ts.parse().expect("should be parseable as f64");
        assert!(parsed > 0.0, "should be a positive timestamp");
    }

    #[test]
    fn test_get_today_start_ts_is_midnight_utc() {
        let ts = get_today_start_ts();
        let seconds: i64 = ts.split('.').next().unwrap().parse().unwrap();
        let datetime = chrono::DateTime::from_timestamp(seconds, 0).unwrap();

        assert_eq!(datetime.time().hour(), 0);
        assert_eq!(datetime.time().minute(), 0);
        assert_eq!(datetime.time().second(), 0);
    }

    #[test]
    fn test_get_today_start_ts_is_today() {
        let ts = get_today_start_ts();
        let seconds: i64 = ts.split('.').next().unwrap().parse().unwrap();
        let ts_date = chrono::DateTime::from_timestamp(seconds, 0)
            .unwrap()
            .date_naive();
        let today = chrono::Utc::now().date_naive();

        assert_eq!(ts_date, today, "timestamp should be for today's date");
    }

    #[test]
    fn test_get_today_start_ts_reasonable_range() {
        let ts = get_today_start_ts();
        let seconds: i64 = ts.split('.').next().unwrap().parse().unwrap();

        let year_2020 = 1577836800_i64;
        let year_2100 = 4102444800_i64;

        assert!(seconds > year_2020, "timestamp should be after 2020");
        assert!(seconds < year_2100, "timestamp should be before 2100");
    }

    #[test]
    fn test_historical_date_parsing() {
        let date_str = "2026-01-25";
        let target_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
        assert!(target_date.is_ok());

        let date = target_date.unwrap();
        assert_eq!(date.year(), 2026);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 25);
    }

    #[test]
    fn test_historical_date_parsing_invalid() {
        let invalid_dates = ["2026/01/25", "01-25-2026", "not-a-date", ""];
        for date_str in invalid_dates {
            let result = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
            assert!(result.is_err(), "should reject invalid date: {}", date_str);
        }
    }

    #[test]
    fn test_historical_timestamp_bounds_utc() {
        use chrono::{NaiveDate, Utc};

        let date_str = "2026-01-25";
        let target_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
        let offset = chrono::FixedOffset::west_opt(0).unwrap(); // UTC

        let local_midnight = target_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(offset)
            .single()
            .unwrap();

        let start_ts_secs = local_midnight.with_timezone(&Utc).timestamp();
        let end_ts_secs = start_ts_secs + 86400;

        // Verify the bounds span exactly 24 hours
        assert_eq!(end_ts_secs - start_ts_secs, 86400);

        // Verify the Slack API format
        let oldest_ts = format!("{}.999999", start_ts_secs - 1);
        let latest_ts = format!("{}.000000", end_ts_secs);

        assert!(oldest_ts.ends_with(".999999"));
        assert!(latest_ts.ends_with(".000000"));
    }

    #[test]
    fn test_historical_timestamp_bounds_pst() {
        use chrono::{NaiveDate, Utc};

        let date_str = "2026-01-25";
        let target_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
        // PST is UTC-8, which is 480 minutes west
        let offset = chrono::FixedOffset::west_opt(480 * 60).unwrap();

        let local_midnight = target_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(offset)
            .single()
            .unwrap();

        let start_ts_secs = local_midnight.with_timezone(&Utc).timestamp();

        // PST midnight on 2026-01-25 should be 08:00 UTC on 2026-01-25
        let utc_datetime = chrono::DateTime::from_timestamp(start_ts_secs, 0).unwrap();
        assert_eq!(utc_datetime.hour(), 8);
        assert_eq!(utc_datetime.day(), 25);
    }

    #[test]
    fn test_historical_timestamp_bounds_est() {
        use chrono::{NaiveDate, Utc};

        let date_str = "2026-01-25";
        let target_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
        // EST is UTC-5, which is 300 minutes west
        let offset = chrono::FixedOffset::west_opt(300 * 60).unwrap();

        let local_midnight = target_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(offset)
            .single()
            .unwrap();

        let start_ts_secs = local_midnight.with_timezone(&Utc).timestamp();

        // EST midnight on 2026-01-25 should be 05:00 UTC on 2026-01-25
        let utc_datetime = chrono::DateTime::from_timestamp(start_ts_secs, 0).unwrap();
        assert_eq!(utc_datetime.hour(), 5);
        assert_eq!(utc_datetime.day(), 25);
    }

    #[test]
    fn test_historical_timestamp_negative_offset() {
        use chrono::{NaiveDate, Utc};

        let date_str = "2026-01-25";
        let target_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
        // Timezone east of UTC (e.g., UTC+5:30 India) - negative offset in JS convention
        // But FixedOffset::west_opt with negative value means east
        let offset = chrono::FixedOffset::west_opt(-330 * 60).unwrap(); // UTC+5:30

        let local_midnight = target_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(offset)
            .single()
            .unwrap();

        let start_ts_secs = local_midnight.with_timezone(&Utc).timestamp();

        // IST midnight on 2026-01-25 should be 18:30 UTC on 2026-01-24
        let utc_datetime = chrono::DateTime::from_timestamp(start_ts_secs, 0).unwrap();
        assert_eq!(utc_datetime.hour(), 18);
        assert_eq!(utc_datetime.minute(), 30);
        assert_eq!(utc_datetime.day(), 24); // Previous day in UTC
    }
}
