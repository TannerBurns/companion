use std::sync::Arc;
use chrono::Utc;
use serde::Serialize;
use crate::db::Database;
use crate::crypto::CryptoService;
use super::gemini::GeminiClient;
use super::prompts::{self, SummaryResult, GroupedAnalysisResult};

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

/// Message format for the batch analysis prompt
#[derive(Serialize)]
struct MessageForPrompt {
    id: String,
    channel: String,
    author: String,
    timestamp: String,
    text: String,
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
        let prompt = prompts::batch_analysis_prompt(&date_str, &messages_json);

        tracing::info!("Sending batch of {} messages to AI for analysis", messages_for_prompt.len());
        let result: GroupedAnalysisResult = self.gemini
            .generate_json(&prompt)
            .await
            .map_err(|e| e.to_string())?;

        let now = Utc::now().timestamp_millis();
        let mut stored_count = 0;

        for group in &result.groups {
            let summary_id = uuid::Uuid::new_v4().to_string();
            
            let entities_json = serde_json::to_string(&serde_json::json!({
                "topic": &group.topic,
                "channels": &group.channels,
                "people": &group.people,
                "message_ids": &group.message_ids
            })).unwrap_or_default();
            
            sqlx::query(
                "INSERT INTO ai_summaries (id, content_item_id, summary_type, summary, highlights, category, category_confidence, importance_score, entities, generated_at)
                 VALUES (?, NULL, 'group', ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&summary_id)
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

        let digest_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO ai_summaries (id, summary_type, summary, highlights, generated_at)
             VALUES (?, 'daily', ?, ?, ?)"
        )
        .bind(&digest_id)
        .bind(&result.daily_summary)
        .bind(serde_json::to_string(&result.key_themes).unwrap_or_default())
        .bind(now)
        .execute(self.db.pool())
        .await
        .map_err(|e| e.to_string())?;

        tracing::info!(
            "Batch processing complete: {} groups, {} ungrouped, {} action items",
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
