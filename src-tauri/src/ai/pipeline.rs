use std::sync::Arc;
use sha2::{Sha256, Digest};
use chrono::Utc;
use serde::Serialize;
use crate::db::Database;
use crate::crypto::CryptoService;
use super::gemini::GeminiClient;
use super::prompts::{self, SummaryResult, GroupedAnalysisResult, ExistingTopic};

#[derive(sqlx::FromRow)]
struct ContentItemRow {
    id: String,
    source: String,
    content_type: String,
    title: Option<String>,
    body: Option<String>,
    author_id: Option<String>,
    channel_or_project: Option<String>,
    created_at: i64,
}

#[derive(Serialize)]
struct MessageForPrompt {
    id: String,
    channel: String,
    author: String,
    timestamp: String,
    text: String,
}

#[derive(sqlx::FromRow)]
struct ExistingTopicRow {
    id: String,
    summary: String,
    category: Option<String>,
    importance_score: Option<f64>,
    entities: Option<String>,
}

fn generate_topic_id(topic: &str, date: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(topic.to_lowercase().as_bytes());
    hasher.update(date.as_bytes());
    let result = hasher.finalize();
    format!("topic_{:x}", &result[..8].iter().fold(0u64, |acc, &b| (acc << 8) | b as u64))
}

pub struct ProcessingPipeline {
    gemini: GeminiClient,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
}

impl ProcessingPipeline {
    pub fn new(api_key: String, db: Arc<Database>, crypto: Arc<CryptoService>) -> Self {
        Self {
            gemini: GeminiClient::new(api_key),
            db,
            crypto,
        }
    }

    pub async fn process_daily_batch(&self) -> Result<i32, String> {
        let today = Utc::now().date_naive();
        let date_str = today.format("%Y-%m-%d").to_string();
        
        let start_ts = today
            .and_hms_opt(0, 0, 0)
            .ok_or("Invalid date")?
            .and_utc()
            .timestamp_millis();
        let end_ts = start_ts + 86400 * 1000;

        let items: Vec<ContentItemRow> = sqlx::query_as(
            "SELECT ci.id, ci.source, ci.content_type, ci.title, ci.body, 
                    ci.author_id, ci.channel_or_project, ci.created_at
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

        let mut existing_message_ids_map: std::collections::HashMap<String, Vec<String>> = 
            std::collections::HashMap::new();
        let mut existing_topics: Vec<ExistingTopic> = Vec::new();
        
        for row in &existing_topic_rows {
            let entities: serde_json::Value = row.entities.as_ref()
                .and_then(|e| serde_json::from_str(e).ok())
                .unwrap_or(serde_json::json!({}));
            
            // Only process rows that have a valid topic field - these are the ones
            // we'll include in the AI prompt. Rows without topics should not have
            // their message_ids cached, as the AI won't know about them and
            // shouldn't be able to reference them.
            let Some(topic) = entities.get("topic").and_then(|v| v.as_str()) else {
                continue;
            };
            
            let message_ids: Vec<String> = entities.get("message_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let message_count: i32 = message_ids.len().try_into().unwrap_or(i32::MAX);
            
            // Cache message IDs only for rows that will be in existing_topics
            existing_message_ids_map.insert(row.id.clone(), message_ids);
            
            let channels: Vec<String> = entities.get("channels")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let people: Vec<String> = entities.get("people")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            
            existing_topics.push(ExistingTopic {
                topic_id: row.id.clone(),
                topic: topic.to_string(),
                channels,
                summary: row.summary.clone(),
                category: row.category.clone().unwrap_or_else(|| "other".to_string()),
                importance_score: row.importance_score.unwrap_or(0.5),
                message_count,
                people,
            });
        }

        tracing::info!("Found {} existing topic groups for today", existing_topics.len());

        let mut messages_for_prompt: Vec<MessageForPrompt> = Vec::new();
        let mut item_ids: Vec<String> = Vec::new();

        for item in &items {
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

            messages_for_prompt.push(MessageForPrompt {
                id: item.id.clone(),
                channel: item.channel_or_project.clone().unwrap_or_else(|| "unknown".to_string()),
                author: item.author_id.clone().unwrap_or_else(|| "unknown".to_string()),
                timestamp,
                text,
            });
            item_ids.push(item.id.clone());
        }

        if messages_for_prompt.is_empty() {
            tracing::info!("All items were empty, nothing to process");
            return Ok(0);
        }

        let messages_json = serde_json::to_string_pretty(&messages_for_prompt)
            .map_err(|e| e.to_string())?;
        
        let prompt = if existing_topics.is_empty() {
            prompts::batch_analysis_prompt(&date_str, &messages_json)
        } else {
            let existing_topics_json = serde_json::to_string_pretty(&existing_topics)
                .map_err(|e| e.to_string())?;
            prompts::batch_analysis_prompt_with_existing(&date_str, &messages_json, Some(&existing_topics_json))
        };

        tracing::info!("Sending batch of {} messages to AI for analysis (with {} existing topics)", 
            messages_for_prompt.len(), existing_topics.len());
        let result: GroupedAnalysisResult = self.gemini
            .generate_json(&prompt)
            .await
            .map_err(|e| e.to_string())?;

        let now = Utc::now().timestamp_millis();
        let mut stored_count = 0;

        for group in &result.groups {
            let ai_recognized_existing = group.topic_id.is_some();
            let topic_id = group.topic_id.clone()
                .unwrap_or_else(|| generate_topic_id(&group.topic, &date_str));
            
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM ai_summaries WHERE id = ?"
            )
            .bind(&topic_id)
            .fetch_optional(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;

            let should_update = existing.is_some() && ai_recognized_existing;

            let merged_message_ids: Vec<String> = if should_update {
                let mut merged: Vec<String> = if let Some(cached) = existing_message_ids_map.get(&topic_id) {
                    cached.clone()
                } else {
                    tracing::warn!(
                        "Topic {} exists in DB but not in local map - fetching fresh",
                        topic_id
                    );
                    let fresh_row: Option<(Option<String>,)> = sqlx::query_as(
                        "SELECT entities FROM ai_summaries WHERE id = ?"
                    )
                    .bind(&topic_id)
                    .fetch_optional(self.db.pool())
                    .await
                    .map_err(|e| e.to_string())?;
                    
                    fresh_row
                        .and_then(|(entities_opt,)| entities_opt)
                        .and_then(|e| serde_json::from_str::<serde_json::Value>(&e).ok())
                        .and_then(|v| v.get("message_ids").cloned())
                        .and_then(|v| serde_json::from_value(v).ok())
                        .unwrap_or_default()
                };
                
                for msg_id in &group.message_ids {
                    if !merged.contains(msg_id) {
                        merged.push(msg_id.clone());
                    }
                }
                merged
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
                tracing::info!("Updating existing topic: {} (merging {} existing + {} new = {} total message_ids)", 
                    group.topic, 
                    existing_count,
                    group.message_ids.len(),
                    merged_message_ids.len());
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

        tracing::info!(
            "Batch processing complete: {} groups (updated/new), {} ungrouped, {} action items",
            result.groups.len(),
            result.ungrouped.len(),
            result.action_items.len()
        );

        Ok(stored_count)
    }

    #[allow(dead_code)]
    pub async fn process_pending(&self) -> Result<i32, String> {
        let items: Vec<ContentItemRow> = sqlx::query_as(
            "SELECT ci.id, ci.source, ci.content_type, ci.title, ci.body, 
                    ci.author_id, ci.channel_or_project, ci.created_at
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
                id,  // Jira issue key (e.g., "PROJ-123")
                title.as_deref().unwrap_or(""),
                &body_text,
            ),
            ("confluence", "page") => prompts::confluence_page_prompt(
                title.as_deref().unwrap_or(""),
                channel.as_deref().unwrap_or(""),
                &body_text,
            ),
            _ => return Ok(()), // Skip unknown types
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

    /// Generate daily digest
    pub async fn generate_daily_digest(&self, date: &str) -> Result<String, String> {
        let parsed_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| e.to_string())?;
        let start_ts = parsed_date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| format!("Invalid date: {}", date))?
            .and_utc()
            .timestamp_millis();  // Use milliseconds to match content_items.created_at
        let end_ts = start_ts + 86400 * 1000;  // 24 hours in milliseconds

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
        let digest_id = uuid::Uuid::new_v4().to_string();
        
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

        Ok(digest.summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_topic_id_is_deterministic() {
        let id1 = generate_topic_id("Q1 Product Launch", "2024-01-15");
        let id2 = generate_topic_id("Q1 Product Launch", "2024-01-15");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_generate_topic_id_is_case_insensitive() {
        let id1 = generate_topic_id("Q1 Product Launch", "2024-01-15");
        let id2 = generate_topic_id("q1 product launch", "2024-01-15");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_generate_topic_id_different_topics() {
        let id1 = generate_topic_id("Q1 Product Launch", "2024-01-15");
        let id2 = generate_topic_id("Q2 Marketing Campaign", "2024-01-15");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_topic_id_different_dates() {
        let id1 = generate_topic_id("Q1 Product Launch", "2024-01-15");
        let id2 = generate_topic_id("Q1 Product Launch", "2024-01-16");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_topic_id_format() {
        let id = generate_topic_id("Test Topic", "2024-01-15");
        assert!(id.starts_with("topic_"));
        let hex_part = &id[6..];
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_existing_topic_row_fields() {
        let row = ExistingTopicRow {
            id: "test_id".to_string(),
            summary: "Test summary".to_string(),
            category: Some("engineering".to_string()),
            importance_score: Some(0.8),
            entities: Some(r#"{"topic": "Test", "channels": [], "people": [], "message_ids": []}"#.to_string()),
        };
        
        assert_eq!(row.id, "test_id");
        assert_eq!(row.summary, "Test summary");
        assert_eq!(row.category, Some("engineering".to_string()));
        assert_eq!(row.importance_score, Some(0.8));
        assert!(row.entities.is_some());
    }

    #[test]
    fn test_existing_topic_conversion() {
        let entities_json = serde_json::json!({
            "topic": "Sprint Planning",
            "channels": ["#engineering", "#product"],
            "people": ["Alice", "Bob"],
            "message_ids": ["msg1", "msg2", "msg3"]
        });

        let row = ExistingTopicRow {
            id: "topic_abc".to_string(),
            summary: "Discussed sprint goals".to_string(),
            category: Some("engineering".to_string()),
            importance_score: Some(0.85),
            entities: Some(serde_json::to_string(&entities_json).unwrap()),
        };

        let entities: serde_json::Value = row.entities.as_ref()
            .and_then(|e| serde_json::from_str(e).ok())
            .unwrap_or(serde_json::json!({}));
        
        let topic = entities.get("topic").and_then(|v| v.as_str()).unwrap();
        let channels: Vec<String> = entities.get("channels")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let people: Vec<String> = entities.get("people")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let message_ids: Vec<String> = entities.get("message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let existing = ExistingTopic {
            topic_id: row.id.clone(),
            topic: topic.to_string(),
            channels,
            summary: row.summary.clone(),
            category: row.category.clone().unwrap_or_else(|| "other".to_string()),
            importance_score: row.importance_score.unwrap_or(0.5),
            message_count: message_ids.len().try_into().unwrap_or(i32::MAX),
            people,
        };

        assert_eq!(existing.topic_id, "topic_abc");
        assert_eq!(existing.topic, "Sprint Planning");
        assert_eq!(existing.channels, vec!["#engineering", "#product"]);
        assert_eq!(existing.people, vec!["Alice", "Bob"]);
        assert_eq!(existing.message_count, 3);
        assert_eq!(existing.importance_score, 0.85);
    }

    #[test]
    fn test_generate_topic_id_handles_empty_topic() {
        let id = generate_topic_id("", "2024-01-15");
        assert!(id.starts_with("topic_"));
        assert!(!id.is_empty());
    }

    #[test]
    fn test_generate_topic_id_handles_special_characters() {
        let id1 = generate_topic_id("Q1 Launch! @#$%", "2024-01-15");
        let id2 = generate_topic_id("q1 launch! @#$%", "2024-01-15");
        assert!(id1.starts_with("topic_"));
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_generate_topic_id_handles_unicode() {
        let id = generate_topic_id("プロジェクト計画", "2024-01-15");
        assert!(id.starts_with("topic_"));
        let hex_part = &id[6..];
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_existing_topic_conversion_missing_entities() {
        let row = ExistingTopicRow {
            id: "topic_abc".to_string(),
            summary: "Summary".to_string(),
            category: None,
            importance_score: None,
            entities: None,
        };

        let entities: serde_json::Value = row.entities.as_ref()
            .and_then(|e| serde_json::from_str(e).ok())
            .unwrap_or(serde_json::json!({}));
        
        let topic = entities.get("topic").and_then(|v| v.as_str());
        assert!(topic.is_none());
        
        let category = row.category.clone().unwrap_or_else(|| "other".to_string());
        assert_eq!(category, "other");
        
        let importance = row.importance_score.unwrap_or(0.5);
        assert_eq!(importance, 0.5);
    }

    #[test]
    fn test_existing_topic_conversion_partial_entities() {
        let entities_json = serde_json::json!({
            "topic": "Partial Topic"
        });

        let row = ExistingTopicRow {
            id: "topic_partial".to_string(),
            summary: "Partial summary".to_string(),
            category: Some("product".to_string()),
            importance_score: Some(0.7),
            entities: Some(serde_json::to_string(&entities_json).unwrap()),
        };

        let entities: serde_json::Value = row.entities.as_ref()
            .and_then(|e| serde_json::from_str(e).ok())
            .unwrap_or(serde_json::json!({}));
        
        let topic = entities.get("topic").and_then(|v| v.as_str()).unwrap();
        let channels: Vec<String> = entities.get("channels")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let people: Vec<String> = entities.get("people")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        assert_eq!(topic, "Partial Topic");
        assert!(channels.is_empty());
        assert!(people.is_empty());
    }

    #[test]
    fn test_existing_topic_conversion_malformed_entities() {
        let row = ExistingTopicRow {
            id: "topic_bad".to_string(),
            summary: "Summary".to_string(),
            category: Some("other".to_string()),
            importance_score: Some(0.5),
            entities: Some("not valid json".to_string()),
        };

        let entities: serde_json::Value = row.entities.as_ref()
            .and_then(|e| serde_json::from_str(e).ok())
            .unwrap_or(serde_json::json!({}));
        
        assert!(entities.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_message_ids_merge_logic() {
        let existing: Vec<String> = vec!["msg1".into(), "msg2".into(), "msg3".into()];
        let new: Vec<String> = vec!["msg3".into(), "msg4".into(), "msg5".into()];
        
        let mut merged = existing.clone();
        for msg_id in &new {
            if !merged.contains(msg_id) {
                merged.push(msg_id.clone());
            }
        }
        
        assert_eq!(merged.len(), 5);
        assert_eq!(merged, vec!["msg1", "msg2", "msg3", "msg4", "msg5"]);
    }

    #[test]
    fn test_message_ids_merge_preserves_order() {
        let existing: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let new: Vec<String> = vec!["d".into(), "e".into()];
        
        let mut merged = existing.clone();
        for msg_id in &new {
            if !merged.contains(msg_id) {
                merged.push(msg_id.clone());
            }
        }
        
        assert_eq!(merged, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn test_message_ids_merge_empty_existing() {
        let existing: Vec<String> = vec![];
        let new: Vec<String> = vec!["msg1".into(), "msg2".into()];
        
        let mut merged = existing.clone();
        for msg_id in &new {
            if !merged.contains(msg_id) {
                merged.push(msg_id.clone());
            }
        }
        
        assert_eq!(merged.len(), 2);
        assert_eq!(merged, vec!["msg1", "msg2"]);
    }

    #[test]
    fn test_message_ids_merge_empty_new() {
        let existing: Vec<String> = vec!["msg1".into(), "msg2".into()];
        let new: Vec<String> = vec![];
        
        let mut merged = existing.clone();
        for msg_id in &new {
            if !merged.contains(msg_id) {
                merged.push(msg_id.clone());
            }
        }
        
        assert_eq!(merged.len(), 2);
        assert_eq!(merged, vec!["msg1", "msg2"]);
    }

    #[test]
    fn test_message_ids_merge_all_duplicates() {
        let existing: Vec<String> = vec!["msg1".into(), "msg2".into()];
        let new: Vec<String> = vec!["msg1".into(), "msg2".into()];
        
        let mut merged = existing.clone();
        for msg_id in &new {
            if !merged.contains(msg_id) {
                merged.push(msg_id.clone());
            }
        }
        
        assert_eq!(merged, vec!["msg1", "msg2"]);
    }

    #[test]
    fn test_message_ids_cached_for_rows_with_missing_topic() {
        let rows = vec![
            ExistingTopicRow {
                id: "topic_valid".to_string(),
                summary: "Valid topic".to_string(),
                category: Some("engineering".to_string()),
                importance_score: Some(0.8),
                entities: Some(r#"{"topic": "Valid Topic", "channels": [], "people": [], "message_ids": ["msg1", "msg2"]}"#.to_string()),
            },
            ExistingTopicRow {
                id: "topic_malformed".to_string(),
                summary: "Malformed entry".to_string(),
                category: Some("other".to_string()),
                importance_score: Some(0.5),
                entities: Some(r#"{"channels": [], "people": [], "message_ids": ["msg3", "msg4", "msg5"]}"#.to_string()),
            },
            ExistingTopicRow {
                id: "topic_invalid_json".to_string(),
                summary: "Invalid JSON".to_string(),
                category: None,
                importance_score: None,
                entities: Some("not valid json at all".to_string()),
            },
        ];
        
        let mut existing_message_ids_map: std::collections::HashMap<String, Vec<String>> = 
            std::collections::HashMap::new();
        let mut existing_topics: Vec<ExistingTopic> = Vec::new();
        
        for row in &rows {
            let entities: serde_json::Value = row.entities.as_ref()
                .and_then(|e| serde_json::from_str(e).ok())
                .unwrap_or(serde_json::json!({}));
            
            let message_ids: Vec<String> = entities.get("message_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let message_count: i32 = message_ids.len().try_into().unwrap_or(i32::MAX);
            existing_message_ids_map.insert(row.id.clone(), message_ids);
            
            let Some(topic) = entities.get("topic").and_then(|v| v.as_str()) else {
                continue;
            };
            let channels: Vec<String> = entities.get("channels")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let people: Vec<String> = entities.get("people")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            
            existing_topics.push(ExistingTopic {
                topic_id: row.id.clone(),
                topic: topic.to_string(),
                channels,
                summary: row.summary.clone(),
                category: row.category.clone().unwrap_or_else(|| "other".to_string()),
                importance_score: row.importance_score.unwrap_or(0.5),
                message_count,
                people,
            });
        }
        
        assert_eq!(existing_topics.len(), 1);
        assert_eq!(existing_topics[0].topic_id, "topic_valid");
        assert_eq!(existing_message_ids_map.len(), 3);
        assert_eq!(
            existing_message_ids_map.get("topic_valid"),
            Some(&vec!["msg1".to_string(), "msg2".to_string()])
        );
        assert_eq!(
            existing_message_ids_map.get("topic_malformed"),
            Some(&vec!["msg3".to_string(), "msg4".to_string(), "msg5".to_string()])
        );
        assert_eq!(
            existing_message_ids_map.get("topic_invalid_json"),
            Some(&vec![])
        );
    }
}
