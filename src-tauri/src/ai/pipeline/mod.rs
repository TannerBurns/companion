mod hierarchical;
mod storage;
mod topics;
mod types;

pub use topics::{convert_existing_topics, generate_topic_id, merge_message_ids};
pub use types::{
    ContentItemRow, ExistingTopicRow, MessageForPrompt, SlackUserRow, HISTORICAL_AI_CHUNK_SIZE,
    HIERARCHICAL_CHANNEL_CHUNK_SIZE, HIERARCHICAL_CHANNEL_THRESHOLD, HIERARCHICAL_TOTAL_THRESHOLD,
};

use super::gemini::{GeminiClient, ServiceAccountCredentials};
use super::prompts::{self, ExistingTopic, GroupedAnalysisResult};
use crate::crypto::CryptoService;
use crate::db::Database;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

/// Main AI processing pipeline for content analysis.
pub struct ProcessingPipeline {
    gemini: GeminiClient,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
}

impl ProcessingPipeline {
    /// Create a new ProcessingPipeline.
    ///
    /// The `api_key_or_credentials` parameter can be:
    /// - A plain API key string
    /// - A string prefixed with "SERVICE_ACCOUNT:" followed by JSON credentials
    pub fn new(
        api_key_or_credentials: String,
        db: Arc<Database>,
        crypto: Arc<CryptoService>,
    ) -> Self {
        let gemini = if let Some(json_str) = api_key_or_credentials.strip_prefix("SERVICE_ACCOUNT:")
        {
            match serde_json::from_str::<ServiceAccountCredentials>(json_str) {
                Ok(credentials) => GeminiClient::new_with_service_account(credentials),
                Err(e) => {
                    tracing::error!("Failed to parse service account credentials: {}", e);
                    GeminiClient::new(String::new())
                }
            }
        } else {
            GeminiClient::new(api_key_or_credentials)
        };

        Self { gemini, db, crypto }
    }

    /// Load user guidance from preferences.
    async fn load_user_guidance(&self) -> Option<String> {
        let result: Option<(String,)> =
            sqlx::query_as("SELECT value FROM preferences WHERE key = 'user_preferences'")
                .fetch_optional(self.db.pool())
                .await
                .ok()?;

        result.and_then(|(json,)| {
            let prefs: serde_json::Value = serde_json::from_str(&json).ok()?;
            prefs
                .get("userGuidance")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(String::from)
        })
    }

    /// Load Slack user ID to display name mapping.
    async fn load_user_map(&self) -> Result<HashMap<String, String>, String> {
        let users: Vec<SlackUserRow> =
            sqlx::query_as("SELECT user_id, real_name, display_name FROM slack_users")
                .fetch_all(self.db.pool())
                .await
                .map_err(|e| e.to_string())?;

        let mut map = HashMap::new();
        for user in users {
            let name = user
                .display_name
                .filter(|s| !s.is_empty())
                .or(user.real_name)
                .unwrap_or_else(|| user.user_id.clone());
            map.insert(user.user_id, name);
        }

        Ok(map)
    }

    /// Process daily batch with optional timezone offset.
    ///
    /// `timezone_offset_minutes`: Minutes offset from UTC (positive = west of UTC, e.g., PST = 480)
    /// This matches JavaScript's `Date.getTimezoneOffset()` convention.
    pub async fn process_daily_batch(
        &self,
        timezone_offset_minutes: Option<i32>,
    ) -> Result<i32, String> {
        let offset_minutes = timezone_offset_minutes.unwrap_or(0);

        // Calculate local date and time boundaries
        let now_utc = Utc::now();
        let offset = chrono::FixedOffset::west_opt(offset_minutes * 60)
            .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
        let local_now = now_utc.with_timezone(&offset);
        let today = local_now.date_naive();
        let date_str = today.format("%Y-%m-%d").to_string();

        // Convert local midnight to UTC timestamp
        let local_midnight = today
            .and_hms_opt(0, 0, 0)
            .ok_or("Invalid date")?
            .and_local_timezone(offset)
            .single()
            .ok_or("Ambiguous or invalid local time")?;

        let start_ts = local_midnight.with_timezone(&Utc).timestamp_millis();
        let end_ts = start_ts + 86400 * 1000;

        // Fetch unprocessed content items
        let items: Vec<ContentItemRow> = sqlx::query_as(
            "SELECT ci.id, ci.source, ci.content_type, ci.title, ci.body, 
                    ci.author_id, ci.channel_or_project, ci.source_url, ci.parent_id, ci.created_at
             FROM content_items ci
             LEFT JOIN ai_summaries s ON ci.id = s.content_item_id
             WHERE s.id IS NULL AND ci.created_at >= ? AND ci.created_at < ?
             ORDER BY ci.created_at ASC",
        )
        .bind(start_ts)
        .bind(end_ts)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        if items.is_empty() {
            tracing::info!("No unprocessed items for today");
            return Ok(0);
        }

        tracing::info!("Processing {} items in batch for {}", items.len(), date_str);
        let user_map = self.load_user_map().await.unwrap_or_default();
        let user_guidance = self.load_user_guidance().await;
        if user_guidance.is_some() {
            tracing::info!("User guidance loaded, will apply to AI prompts");
        }

        // Load existing topics for this day
        let existing_topic_rows: Vec<ExistingTopicRow> = sqlx::query_as(
            "SELECT id, summary, category, importance_score, entities
             FROM ai_summaries
             WHERE summary_type = 'group' AND generated_at >= ? AND generated_at < ?
             ORDER BY importance_score DESC",
        )
        .bind(start_ts)
        .bind(end_ts)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        let (mut existing_message_ids_map, existing_topics) =
            convert_existing_topics(&existing_topic_rows);
        tracing::info!(
            "Found {} existing topic groups for today",
            existing_topics.len()
        );

        // Build messages for prompt
        let (messages_for_prompt, _item_ids) =
            self.build_messages_for_prompt(&items, &user_map).await;

        if messages_for_prompt.is_empty() {
            tracing::info!("All items were empty, nothing to process");
            return Ok(0);
        }

        // Decide processing strategy
        let result = if messages_for_prompt.len() >= HIERARCHICAL_TOTAL_THRESHOLD {
            tracing::info!(
                "Using hierarchical summarization for {} messages",
                messages_for_prompt.len()
            );

            let mut messages_by_channel: HashMap<String, Vec<MessageForPrompt>> = HashMap::new();
            for msg in messages_for_prompt {
                messages_by_channel
                    .entry(msg.channel.clone())
                    .or_default()
                    .push(msg);
            }

            hierarchical::process_hierarchical(
                &self.gemini,
                &date_str,
                messages_by_channel,
                user_guidance.as_deref(),
            )
            .await?
        } else {
            self.process_batch_direct(
                &date_str,
                messages_for_prompt,
                &existing_topics,
                user_guidance.as_deref(),
            )
            .await?
        };

        // Store results
        let stored_count = storage::store_results(
            self.db.pool(),
            &result,
            &date_str,
            &mut existing_message_ids_map,
        )
        .await?;

        tracing::info!(
            "Batch processing complete: {} groups (updated/new), {} ungrouped, {} action items",
            result.groups.len(),
            result.ungrouped.len(),
            result.action_items.len()
        );

        Ok(stored_count)
    }

    /// Build messages for prompt from content items.
    async fn build_messages_for_prompt(
        &self,
        items: &[ContentItemRow],
        user_map: &HashMap<String, String>,
    ) -> (Vec<MessageForPrompt>, Vec<String>) {
        let mut messages_for_prompt: Vec<MessageForPrompt> = Vec::new();
        let mut item_ids: Vec<String> = Vec::new();

        for item in items {
            let text = match &item.body {
                Some(encrypted) => self
                    .crypto
                    .decrypt_string(encrypted)
                    .unwrap_or_else(|_| "[decryption failed]".to_string()),
                None => String::new(),
            };

            if text.trim().is_empty() {
                continue;
            }

            let timestamp = chrono::DateTime::from_timestamp_millis(item.created_at)
                .map(|dt| dt.format("%H:%M").to_string())
                .unwrap_or_default();

            let author_name = item
                .author_id
                .as_ref()
                .and_then(|id| user_map.get(id))
                .cloned()
                .unwrap_or_else(|| {
                    item.author_id
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string())
                });

            messages_for_prompt.push(MessageForPrompt {
                id: item.id.clone(),
                channel: item
                    .channel_or_project
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                author: author_name,
                timestamp,
                text,
                url: item.source_url.clone(),
                thread_id: item.parent_id.clone(),
            });
            item_ids.push(item.id.clone());
        }

        (messages_for_prompt, item_ids)
    }

    /// Process batch directly without hierarchical summarization.
    async fn process_batch_direct(
        &self,
        date_str: &str,
        messages_for_prompt: Vec<MessageForPrompt>,
        existing_topics: &[ExistingTopic],
        user_guidance: Option<&str>,
    ) -> Result<GroupedAnalysisResult, String> {
        let messages_json =
            serde_json::to_string_pretty(&messages_for_prompt).map_err(|e| e.to_string())?;

        let prompt = if existing_topics.is_empty() {
            prompts::batch_analysis_prompt_with_existing(
                date_str,
                &messages_json,
                None,
                user_guidance,
            )
        } else {
            let existing_topics_json =
                serde_json::to_string_pretty(existing_topics).map_err(|e| e.to_string())?;
            prompts::batch_analysis_prompt_with_existing(
                date_str,
                &messages_json,
                Some(&existing_topics_json),
                user_guidance,
            )
        };

        tracing::info!(
            "Sending batch of {} messages to AI for analysis (with {} existing topics, guidance: {})",
            messages_for_prompt.len(),
            existing_topics.len(),
            user_guidance.is_some()
        );

        self.gemini
            .generate_json(&prompt)
            .await
            .map_err(|e| e.to_string())
    }

    /// Process batch for a specific date.
    pub async fn process_batch_for_date(
        &self,
        date_str: &str,
        timezone_offset_minutes: i32,
    ) -> Result<i32, String> {
        let target_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| format!("Invalid date format: {}", e))?;

        let offset = chrono::FixedOffset::west_opt(timezone_offset_minutes * 60)
            .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());

        let local_midnight = target_date
            .and_hms_opt(0, 0, 0)
            .ok_or("Invalid date")?
            .and_local_timezone(offset)
            .single()
            .ok_or("Ambiguous or invalid local time")?;

        let start_ts = local_midnight.with_timezone(&Utc).timestamp_millis();
        let end_ts = start_ts + 86400 * 1000;

        let user_map = self.load_user_map().await.unwrap_or_default();
        let user_guidance = self.load_user_guidance().await;
        let mut total_stored = 0;
        let mut chunk_index = 0;
        let mut cursor: Option<(i64, String)> = None;

        loop {
            let items: Vec<ContentItemRow> = if let Some((cursor_created_at, ref cursor_id)) = cursor
            {
                sqlx::query_as(
                    "SELECT ci.id, ci.source, ci.content_type, ci.title, ci.body, 
                            ci.author_id, ci.channel_or_project, ci.source_url, ci.parent_id, ci.created_at
                     FROM content_items ci
                     LEFT JOIN ai_summaries s ON ci.id = s.content_item_id
                     WHERE s.id IS NULL
                       AND ci.created_at >= ? AND ci.created_at < ?
                       AND (ci.created_at > ? OR (ci.created_at = ? AND ci.id > ?))
                     ORDER BY ci.created_at ASC, ci.id ASC
                     LIMIT ?",
                )
                .bind(start_ts)
                .bind(end_ts)
                .bind(cursor_created_at)
                .bind(cursor_created_at)
                .bind(cursor_id)
                .bind(HISTORICAL_AI_CHUNK_SIZE)
                .fetch_all(self.db.pool())
                .await
                .map_err(|e| e.to_string())?
            } else {
                sqlx::query_as(
                    "SELECT ci.id, ci.source, ci.content_type, ci.title, ci.body, 
                            ci.author_id, ci.channel_or_project, ci.source_url, ci.parent_id, ci.created_at
                     FROM content_items ci
                     LEFT JOIN ai_summaries s ON ci.id = s.content_item_id
                     WHERE s.id IS NULL AND ci.created_at >= ? AND ci.created_at < ?
                     ORDER BY ci.created_at ASC, ci.id ASC
                     LIMIT ?",
                )
                .bind(start_ts)
                .bind(end_ts)
                .bind(HISTORICAL_AI_CHUNK_SIZE)
                .fetch_all(self.db.pool())
                .await
                .map_err(|e| e.to_string())?
            };

            if items.is_empty() {
                if chunk_index == 0 {
                    tracing::info!("No unprocessed items for {}", date_str);
                }
                break;
            }

            cursor = items.last().map(|item| (item.created_at, item.id.clone()));
            chunk_index += 1;
            tracing::info!(
                "Processing historical AI chunk {} for {} ({} items)",
                chunk_index,
                date_str,
                items.len()
            );

            let existing_topic_rows: Vec<ExistingTopicRow> = sqlx::query_as(
                "SELECT id, summary, category, importance_score, entities
                 FROM ai_summaries
                 WHERE summary_type = 'group' AND generated_at >= ? AND generated_at < ?
                 ORDER BY importance_score DESC",
            )
            .bind(start_ts)
            .bind(end_ts)
            .fetch_all(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;

            let (mut existing_message_ids_map, existing_topics) =
                convert_existing_topics(&existing_topic_rows);

            let (messages_for_prompt, _item_ids) =
                self.build_messages_for_prompt(&items, &user_map).await;

            if messages_for_prompt.is_empty() {
                tracing::warn!(
                    "Historical AI chunk {} for {} had no message text after decrypt/filter, skipping",
                    chunk_index,
                    date_str
                );
                continue;
            }

            let result = if messages_for_prompt.len() >= HIERARCHICAL_TOTAL_THRESHOLD {
                tracing::info!(
                    "Using hierarchical summarization for {} messages",
                    messages_for_prompt.len()
                );

                let mut messages_by_channel: HashMap<String, Vec<MessageForPrompt>> = HashMap::new();
                for msg in messages_for_prompt {
                    messages_by_channel
                        .entry(msg.channel.clone())
                        .or_default()
                        .push(msg);
                }

                hierarchical::process_hierarchical(
                    &self.gemini,
                    date_str,
                    messages_by_channel,
                    user_guidance.as_deref(),
                )
                .await?
            } else {
                self.process_batch_direct(
                    date_str,
                    messages_for_prompt,
                    &existing_topics,
                    user_guidance.as_deref(),
                )
                .await?
            };

            let stored_count = storage::store_results(
                self.db.pool(),
                &result,
                date_str,
                &mut existing_message_ids_map,
            )
            .await?;

            total_stored += stored_count;
            tracing::info!(
                "Historical AI chunk {} complete: {} groups, {} ungrouped, {} action items, stored {}",
                chunk_index,
                result.groups.len(),
                result.ungrouped.len(),
                result.action_items.len(),
                stored_count
            );
        }

        Ok(total_stored)
    }

    /// Generate daily digest for a specific date.
    pub async fn generate_daily_digest(&self, date: &str) -> Result<String, String> {
        let parsed_date =
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|e| e.to_string())?;
        let start_ts = parsed_date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| format!("Invalid date: {}", date))?
            .and_utc()
            .timestamp_millis();
        let end_ts = start_ts + 86400 * 1000;

        let items: Vec<(String, String, Option<String>, f64)> = sqlx::query_as(
            "SELECT s.summary, s.category, s.highlights, s.importance_score
             FROM ai_summaries s
             JOIN content_items c ON s.content_item_id = c.id
             WHERE c.created_at >= ? AND c.created_at < ? AND s.summary_type = 'item'
             ORDER BY s.importance_score DESC
             LIMIT 50",
        )
        .bind(start_ts)
        .bind(end_ts)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        if items.is_empty() {
            return Ok("No items to summarize".to_string());
        }

        let user_guidance = self.load_user_guidance().await;
        let items_json = serde_json::to_string_pretty(&items).unwrap();
        let prompt = prompts::daily_digest_prompt(date, &items_json, user_guidance.as_deref());

        let digest: prompts::DigestSummary = self
            .gemini
            .generate_json(&prompt)
            .await
            .map_err(|e| e.to_string())?;

        let now = chrono::Utc::now().timestamp_millis();
        let digest_id = format!("daily_{}", date);

        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM ai_summaries WHERE id = ?")
                .bind(&digest_id)
                .fetch_optional(self.db.pool())
                .await
                .map_err(|e| e.to_string())?;

        if existing.is_some() {
            sqlx::query(
                "UPDATE ai_summaries SET summary = ?, highlights = ?, generated_at = ? WHERE id = ?"
            )
            .bind(&digest.summary)
            .bind(serde_json::to_string(&digest.key_themes).unwrap())
            .bind(now)
            .bind(&digest_id)
            .execute(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;
        } else {
            sqlx::query(
                "INSERT INTO ai_summaries (id, summary_type, summary, highlights, generated_at)
                 VALUES (?, 'daily', ?, ?, ?)",
            )
            .bind(&digest_id)
            .bind(&digest.summary)
            .bind(serde_json::to_string(&digest.key_themes).unwrap())
            .bind(now)
            .execute(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;
        }

        Ok(digest.summary)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_service_account_credential_parsing() {
        // Test that SERVICE_ACCOUNT: prefix is detected
        let creds = "SERVICE_ACCOUNT:{\"client_email\":\"test@example.com\"}";
        assert!(creds.starts_with("SERVICE_ACCOUNT:"));

        let json_str = creds.strip_prefix("SERVICE_ACCOUNT:").unwrap();
        assert!(json_str.contains("client_email"));
    }

    #[test]
    fn test_plain_api_key_detection() {
        let key = "AIzaSy1234567890abcdef";
        assert!(!key.starts_with("SERVICE_ACCOUNT:"));
    }

    #[test]
    fn test_parse_message_ids_from_entities_json() {
        // Test the JSON parsing logic used in fetch_message_ids_from_db
        let entities_json = r##"{"topic": "Test Topic", "channels": ["#dev"], "message_ids": ["msg1", "msg2", "msg3"]}"##;

        let entities: serde_json::Value = serde_json::from_str(entities_json).unwrap();
        let message_ids: Vec<String> = entities
            .get("message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        assert_eq!(message_ids, vec!["msg1", "msg2", "msg3"]);
    }

    #[test]
    fn test_parse_message_ids_missing_field() {
        // Test when message_ids field is missing
        let entities_json = r##"{"topic": "Test Topic", "channels": ["#dev"]}"##;

        let entities: serde_json::Value = serde_json::from_str(entities_json).unwrap();
        let message_ids: Vec<String> = entities
            .get("message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        assert!(message_ids.is_empty());
    }

    #[test]
    fn test_parse_message_ids_empty_array() {
        // Test when message_ids is an empty array
        let entities_json = r##"{"topic": "Test Topic", "message_ids": []}"##;

        let entities: serde_json::Value = serde_json::from_str(entities_json).unwrap();
        let message_ids: Vec<String> = entities
            .get("message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        assert!(message_ids.is_empty());
    }
}
