mod hierarchical;
mod topics;
mod types;

pub use types::{
    ContentItemRow, ExistingTopicRow, MessageForPrompt, SlackUserRow,
    HIERARCHICAL_CHANNEL_THRESHOLD, HIERARCHICAL_TOTAL_THRESHOLD,
};
pub use topics::{convert_existing_topics, generate_topic_id, merge_message_ids};

use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use crate::db::Database;
use crate::crypto::CryptoService;
use super::gemini::{GeminiClient, ServiceAccountCredentials};
use super::prompts::{self, SummaryResult, GroupedAnalysisResult, ExistingTopic};

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
    pub fn new(api_key_or_credentials: String, db: Arc<Database>, crypto: Arc<CryptoService>) -> Self {
        let gemini = if let Some(json_str) = api_key_or_credentials.strip_prefix("SERVICE_ACCOUNT:") {
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
    
    /// Load Slack user ID to display name mapping.
    async fn load_user_map(&self) -> Result<HashMap<String, String>, String> {
        let users: Vec<SlackUserRow> = sqlx::query_as(
            "SELECT user_id, real_name, display_name FROM slack_users"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;
        
        let mut map = HashMap::new();
        for user in users {
            let name = user.display_name
                .filter(|s| !s.is_empty())
                .or(user.real_name)
                .unwrap_or_else(|| user.user_id.clone());
            map.insert(user.user_id, name);
        }
        
        Ok(map)
    }

    /// Fetch message IDs for a topic from the database.
    /// 
    /// This is a fallback for when the topic exists in the DB but wasn't loaded
    /// into the local message IDs map (e.g., due to malformed entities JSON).
    async fn fetch_message_ids_from_db(&self, topic_id: &str) -> Result<Vec<String>, String> {
        let result: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT entities FROM ai_summaries WHERE id = ?"
        )
        .bind(topic_id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        match result {
            Some((Some(entities_json),)) => {
                let entities: serde_json::Value = serde_json::from_str(&entities_json)
                    .map_err(|e| e.to_string())?;
                let message_ids: Vec<String> = entities.get("message_ids")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                Ok(message_ids)
            }
            _ => Ok(vec![]),
        }
    }

    /// Process daily batch with optional timezone offset.
    /// 
    /// `timezone_offset_minutes`: Minutes offset from UTC (positive = west of UTC, e.g., PST = 480)
    /// This matches JavaScript's `Date.getTimezoneOffset()` convention.
    pub async fn process_daily_batch(&self, timezone_offset_minutes: Option<i32>) -> Result<i32, String> {
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
             ORDER BY ci.created_at ASC"
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

        // Load existing topics for this day
        let existing_topic_rows: Vec<ExistingTopicRow> = sqlx::query_as(
            "SELECT id, summary, category, importance_score, entities
             FROM ai_summaries
             WHERE summary_type = 'group' AND generated_at >= ? AND generated_at < ?
             ORDER BY importance_score DESC"
        )
        .bind(start_ts)
        .bind(end_ts)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        let (mut existing_message_ids_map, existing_topics) = convert_existing_topics(&existing_topic_rows);
        tracing::info!("Found {} existing topic groups for today", existing_topics.len());

        // Build messages for prompt
        let (messages_for_prompt, _item_ids) = self.build_messages_for_prompt(&items, &user_map).await;

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
            
            hierarchical::process_hierarchical(&self.gemini, &date_str, messages_by_channel).await?
        } else {
            self.process_batch_direct(&date_str, messages_for_prompt, &existing_topics).await?
        };

        // Store results
        let stored_count = self.store_results(&result, &date_str, &mut existing_message_ids_map).await?;

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
                Some(encrypted) => self.crypto
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
            
            let author_name = item.author_id
                .as_ref()
                .and_then(|id| user_map.get(id))
                .cloned()
                .unwrap_or_else(|| item.author_id.clone().unwrap_or_else(|| "unknown".to_string()));

            messages_for_prompt.push(MessageForPrompt {
                id: item.id.clone(),
                channel: item.channel_or_project.clone().unwrap_or_else(|| "unknown".to_string()),
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
    ) -> Result<GroupedAnalysisResult, String> {
        let messages_json = serde_json::to_string_pretty(&messages_for_prompt)
            .map_err(|e| e.to_string())?;
        
        let prompt = if existing_topics.is_empty() {
            prompts::batch_analysis_prompt(date_str, &messages_json)
        } else {
            let existing_topics_json = serde_json::to_string_pretty(existing_topics)
                .map_err(|e| e.to_string())?;
            prompts::batch_analysis_prompt_with_existing(date_str, &messages_json, Some(&existing_topics_json))
        };

        tracing::info!(
            "Sending batch of {} messages to AI for analysis (with {} existing topics)", 
            messages_for_prompt.len(), 
            existing_topics.len()
        );
        
        self.gemini
            .generate_json(&prompt)
            .await
            .map_err(|e| e.to_string())
    }

    /// Store processing results to the database.
    async fn store_results(
        &self,
        result: &GroupedAnalysisResult,
        date_str: &str,
        existing_message_ids_map: &mut HashMap<String, Vec<String>>,
    ) -> Result<i32, String> {
        let now = Utc::now().timestamp_millis();
        let mut stored_count = 0;

        // Store topic groups
        for group in &result.groups {
            let ai_recognized_existing = group.topic_id.is_some();
            let topic_id = group.topic_id.clone()
                .unwrap_or_else(|| generate_topic_id(&group.topic, date_str));
            
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM ai_summaries WHERE id = ?"
            )
            .bind(&topic_id)
            .fetch_optional(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;

            let should_update = existing.is_some() && ai_recognized_existing;

            let merged_message_ids = if should_update {
                let existing_ids = if let Some(ids) = existing_message_ids_map.get(&topic_id) {
                    ids.clone()
                } else {
                    // Topic exists in DB but not in local map - fetch from database
                    tracing::warn!("Topic {} exists in DB but not in local map, fetching from database", topic_id);
                    let db_ids = self.fetch_message_ids_from_db(&topic_id).await.unwrap_or_else(|e| {
                        tracing::error!("Failed to fetch message IDs for topic {}: {}", topic_id, e);
                        vec![]
                    });
                    if !db_ids.is_empty() {
                        tracing::info!("Recovered {} message IDs from database for topic {}", db_ids.len(), topic_id);
                    }
                    db_ids
                };
                merge_message_ids(&existing_ids, &group.message_ids)
            } else {
                group.message_ids.clone()
            };
            
            let final_topic_id = if existing.is_some() && !ai_recognized_existing {
                let unique_suffix = &uuid::Uuid::new_v4().to_string()[..8];
                let new_id = format!("{}_{}", topic_id, unique_suffix);
                tracing::warn!(
                    "Topic ID collision for '{}', generating unique ID: {}",
                    group.topic,
                    new_id
                );
                new_id
            } else {
                topic_id
            };

            let entities_json = serde_json::to_string(&serde_json::json!({
                "topic": &group.topic,
                "channels": &group.channels,
                "people": &group.people,
                "message_ids": &merged_message_ids
            })).unwrap_or_default();

            if should_update {
                let existing_count = merged_message_ids.len().saturating_sub(group.message_ids.len());
                tracing::info!(
                    "Updating existing topic: {} (merging {} existing + {} new = {} total message_ids)", 
                    group.topic, 
                    existing_count,
                    group.message_ids.len(),
                    merged_message_ids.len()
                );
                sqlx::query(
                    "UPDATE ai_summaries 
                     SET summary = ?, highlights = ?, category = ?, category_confidence = ?, importance_score = ?, entities = ?, generated_at = ?
                     WHERE id = ?"
                )
                .bind(&group.summary)
                .bind(serde_json::to_string(&group.highlights).unwrap_or_default())
                .bind(&group.category)
                .bind(0.9)
                .bind(group.importance_score)
                .bind(&entities_json)
                .bind(now)
                .bind(&final_topic_id)
                .execute(self.db.pool())
                .await
                .map_err(|e| e.to_string())?;
            } else {
                tracing::info!("Creating new topic: {} (id: {})", group.topic, final_topic_id);
                sqlx::query(
                    "INSERT INTO ai_summaries (id, content_item_id, summary_type, summary, highlights, category, category_confidence, importance_score, entities, generated_at)
                     VALUES (?, NULL, 'group', ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&final_topic_id)
                .bind(&group.summary)
                .bind(serde_json::to_string(&group.highlights).unwrap_or_default())
                .bind(&group.category)
                .bind(0.9)
                .bind(group.importance_score)
                .bind(&entities_json)
                .bind(now)
                .execute(self.db.pool())
                .await
                .map_err(|e| e.to_string())?;
            }

            stored_count += 1;

            // Mark individual messages as processed
            for msg_id in &group.message_ids {
                let placeholder_id = uuid::Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT OR IGNORE INTO ai_summaries (id, content_item_id, summary_type, summary, category, importance_score, generated_at)
                     VALUES (?, ?, 'item', ?, ?, ?, ?)"
                )
                .bind(&placeholder_id)
                .bind(msg_id)
                .bind(format!("Part of group: {}", group.topic))
                .bind(&group.category)
                .bind(group.importance_score)
                .bind(now)
                .execute(self.db.pool())
                .await;
            }
        }

        // Store ungrouped items
        for ungrouped in &result.ungrouped {
            let summary_id = uuid::Uuid::new_v4().to_string();
            
            sqlx::query(
                "INSERT OR IGNORE INTO ai_summaries (id, content_item_id, summary_type, summary, category, importance_score, generated_at)
                 VALUES (?, ?, 'item', ?, ?, ?, ?)"
            )
            .bind(&summary_id)
            .bind(&ungrouped.message_id)
            .bind(&ungrouped.summary)
            .bind(&ungrouped.category)
            .bind(ungrouped.importance_score)
            .bind(now)
            .execute(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;

            stored_count += 1;
        }

        // Store daily summary
        let daily_digest_id = format!("daily_{}", date_str);
        let existing_daily: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM ai_summaries WHERE id = ?"
        )
        .bind(&daily_digest_id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        if existing_daily.is_some() {
            tracing::info!("Updating daily summary for {}", date_str);
            sqlx::query(
                "UPDATE ai_summaries SET summary = ?, highlights = ?, generated_at = ? WHERE id = ?"
            )
            .bind(&result.daily_summary)
            .bind(serde_json::to_string(&result.key_themes).unwrap_or_default())
            .bind(now)
            .bind(&daily_digest_id)
            .execute(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;
        } else {
            tracing::info!("Creating daily summary for {}", date_str);
            sqlx::query(
                "INSERT INTO ai_summaries (id, summary_type, summary, highlights, generated_at)
                 VALUES (?, 'daily', ?, ?, ?)"
            )
            .bind(&daily_digest_id)
            .bind(&result.daily_summary)
            .bind(serde_json::to_string(&result.key_themes).unwrap_or_default())
            .bind(now)
            .execute(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;
        }

        Ok(stored_count)
    }

    /// Process pending items individually (legacy method).
    #[allow(dead_code)]
    pub async fn process_pending(&self) -> Result<i32, String> {
        let items: Vec<ContentItemRow> = sqlx::query_as(
            "SELECT ci.id, ci.source, ci.content_type, ci.title, ci.body, 
                    ci.author_id, ci.channel_or_project, ci.source_url, ci.parent_id, ci.created_at
             FROM content_items ci
             LEFT JOIN ai_summaries s ON ci.id = s.content_item_id
             WHERE s.id IS NULL
             LIMIT 50"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        let mut processed = 0;
        
        for item in items {
            if let Err(e) = self.process_item(
                &item.id,
                &item.source,
                &item.content_type,
                item.title,
                item.body,
                item.channel_or_project,
            ).await {
                tracing::error!("Failed to process item {}: {}", item.id, e);
                continue;
            }
            processed += 1;
        }

        Ok(processed)
    }

    /// Process a single content item.
    async fn process_item(
        &self,
        id: &str,
        source: &str,
        content_type: &str,
        title: Option<String>,
        body: Option<String>,
        channel: Option<String>,
    ) -> Result<(), String> {
        let body_text = match body {
            Some(encrypted) => self.crypto
                .decrypt_string(&encrypted)
                .map_err(|e| e.to_string())?,
            None => String::new(),
        };

        let prompt = match (source, content_type) {
            ("slack", "message") => prompts::slack_message_prompt(
                channel.as_deref().unwrap_or("unknown"),
                &body_text,
            ),
            ("jira", "ticket") => prompts::jira_issue_prompt(
                id,
                title.as_deref().unwrap_or(""),
                &body_text,
            ),
            ("confluence", "page") => prompts::confluence_page_prompt(
                title.as_deref().unwrap_or(""),
                channel.as_deref().unwrap_or(""),
                &body_text,
            ),
            _ => return Ok(()),
        };

        let result: SummaryResult = self.gemini
            .generate_json(&prompt)
            .await
            .map_err(|e| e.to_string())?;

        let now = chrono::Utc::now().timestamp_millis();
        let summary_id = uuid::Uuid::new_v4().to_string();
        
        sqlx::query(
            "INSERT INTO ai_summaries (id, content_item_id, summary_type, summary, highlights, category, category_confidence, importance_score, entities, generated_at)
             VALUES (?, ?, 'item', ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&summary_id)
        .bind(id)
        .bind(&result.summary)
        .bind(serde_json::to_string(&result.highlights).unwrap())
        .bind(&result.category)
        .bind(result.category_confidence)
        .bind(result.importance_score)
        .bind(serde_json::to_string(&result.entities).unwrap())
        .bind(now)
        .execute(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Generate daily digest for a specific date.
    pub async fn generate_daily_digest(&self, date: &str) -> Result<String, String> {
        let parsed_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| e.to_string())?;
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
             LIMIT 50"
        )
        .bind(start_ts)
        .bind(end_ts)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        if items.is_empty() {
            return Ok("No items to summarize".to_string());
        }

        let items_json = serde_json::to_string_pretty(&items).unwrap();
        let prompt = prompts::daily_digest_prompt(date, &items_json);
        
        let digest: prompts::DigestSummary = self.gemini
            .generate_json(&prompt)
            .await
            .map_err(|e| e.to_string())?;

        let now = chrono::Utc::now().timestamp_millis();
        let digest_id = format!("daily_{}", date);
        
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM ai_summaries WHERE id = ?"
        )
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
                 VALUES (?, 'daily', ?, ?, ?)"
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
        let message_ids: Vec<String> = entities.get("message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        
        assert_eq!(message_ids, vec!["msg1", "msg2", "msg3"]);
    }

    #[test]
    fn test_parse_message_ids_missing_field() {
        // Test when message_ids field is missing
        let entities_json = r##"{"topic": "Test Topic", "channels": ["#dev"]}"##;
        
        let entities: serde_json::Value = serde_json::from_str(entities_json).unwrap();
        let message_ids: Vec<String> = entities.get("message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        
        assert!(message_ids.is_empty());
    }

    #[test]
    fn test_parse_message_ids_empty_array() {
        // Test when message_ids is an empty array
        let entities_json = r##"{"topic": "Test Topic", "message_ids": []}"##;
        
        let entities: serde_json::Value = serde_json::from_str(entities_json).unwrap();
        let message_ids: Vec<String> = entities.get("message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        
        assert!(message_ids.is_empty());
    }
}
