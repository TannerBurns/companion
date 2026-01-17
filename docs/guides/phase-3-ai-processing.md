# Phase 3: AI Processing

This guide covers Gemini API integration, prompt templates, the processing pipeline, and digest generation.

## Overview

By the end of this phase, you will have:
- Gemini API client with tool/function calling
- Specialized prompts for each content type
- Processing pipeline for summarization
- Daily and weekly digest aggregation

---

## 3.1 Gemini Client

Create `src-tauri/src/ai/gemini.rs`:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Error, Debug)]
pub enum GeminiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
    FunctionCall { function_call: FunctionCall },
    FunctionResponse { function_response: FunctionResponse },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<i32>,
    pub response_mime_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: Content,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: i32,
    pub total_token_count: i32,
}

pub struct GeminiClient {
    http: Client,
    api_key: String,
    model: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            http: Client::new(),
            api_key,
            model: "gemini-1.5-flash".to_string(),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, GeminiError> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_URL, self.model, self.api_key
        );

        let response = self.http
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(GeminiError::Api(error_text));
        }

        Ok(response.json().await?)
    }

    /// Simple text generation
    pub async fn generate_text(&self, prompt: &str) -> Result<String, GeminiError> {
        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: prompt.to_string() }],
            }],
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.7),
                max_output_tokens: Some(2048),
                response_mime_type: None,
            }),
        };

        let response = self.generate(request).await?;
        
        response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| GeminiError::Parse("No text in response".into()))
    }

    /// Generate with JSON output
    pub async fn generate_json<T: for<'de> Deserialize<'de>>(
        &self,
        prompt: &str,
    ) -> Result<T, GeminiError> {
        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: prompt.to_string() }],
            }],
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.3),
                max_output_tokens: Some(4096),
                response_mime_type: Some("application/json".to_string()),
            }),
        };

        let response = self.generate(request).await?;
        
        let text = response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| GeminiError::Parse("No text in response".into()))?;

        serde_json::from_str(&text)
            .map_err(|e| GeminiError::Parse(e.to_string()))
    }
}
```

---

## 3.2 Prompt Templates

Create `src-tauri/src/ai/prompts.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResult {
    pub summary: String,
    pub highlights: Vec<String>,
    pub category: String,
    pub category_confidence: f64,
    pub importance_score: f64,
    pub entities: Entities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entities {
    pub people: Vec<String>,
    pub projects: Vec<String>,
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestSummary {
    pub summary: String,
    pub key_themes: Vec<String>,
    pub top_items: Vec<TopItem>,
    pub action_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopItem {
    pub title: String,
    pub reason: String,
}

pub fn slack_message_prompt(channel: &str, messages: &str) -> String {
    format!(r#"Analyze this Slack conversation from #{channel} and provide a JSON response:

{messages}

Return JSON with this structure:
{{
  "summary": "2-3 sentence summary of the conversation",
  "highlights": ["key point 1", "key point 2", "key point 3"],
  "category": "one of: sales, marketing, product, engineering, research, other",
  "category_confidence": 0.0-1.0,
  "importance_score": 0.0-1.0,
  "entities": {{
    "people": ["mentioned people"],
    "projects": ["mentioned projects"],
    "topics": ["key topics"]
  }}
}}"#)
}

pub fn jira_issue_prompt(key: &str, summary: &str, description: &str) -> String {
    format!(r#"Analyze this Jira issue and provide a JSON response:

Issue: {key}
Summary: {summary}
Description: {description}

Return JSON with this structure:
{{
  "summary": "2-3 sentence summary explaining what this issue is about and its significance",
  "highlights": ["key point 1", "key point 2"],
  "category": "one of: sales, marketing, product, engineering, research, other",
  "category_confidence": 0.0-1.0,
  "importance_score": 0.0-1.0,
  "entities": {{
    "people": ["mentioned people"],
    "projects": ["mentioned projects"],
    "topics": ["key topics"]
  }}
}}"#)
}

pub fn confluence_page_prompt(title: &str, space: &str, content: &str) -> String {
    let truncated = if content.len() > 8000 {
        &content[..8000]
    } else {
        content
    };
    
    format!(r#"Analyze this Confluence page and provide a JSON response:

Title: {title}
Space: {space}
Content: {truncated}

Return JSON with this structure:
{{
  "summary": "2-3 sentence summary of the page content",
  "highlights": ["key point 1", "key point 2", "key point 3"],
  "category": "one of: sales, marketing, product, engineering, research, other",
  "category_confidence": 0.0-1.0,
  "importance_score": 0.0-1.0,
  "entities": {{
    "people": ["mentioned people"],
    "projects": ["mentioned projects"],
    "topics": ["key topics"]
  }}
}}"#)
}

pub fn daily_digest_prompt(date: &str, items_json: &str) -> String {
    format!(r#"Create a daily digest summary for {date} from these items:

{items_json}

Return JSON with this structure:
{{
  "summary": "3-4 sentence executive summary of the day's key activities",
  "key_themes": ["theme 1", "theme 2", "theme 3"],
  "top_items": [
    {{"title": "item title", "reason": "why this is important"}},
    {{"title": "item title", "reason": "why this is important"}}
  ],
  "action_items": ["suggested action 1", "suggested action 2"]
}}"#)
}

pub fn weekly_digest_prompt(week_start: &str, daily_summaries: &str) -> String {
    format!(r#"Create a weekly digest summary for the week of {week_start}:

{daily_summaries}

Return JSON with this structure:
{{
  "summary": "4-5 sentence executive summary of the week's key activities and trends",
  "key_themes": ["major theme 1", "major theme 2", "major theme 3"],
  "top_items": [
    {{"title": "most important item", "reason": "why this matters"}},
    {{"title": "second important item", "reason": "why this matters"}}
  ],
  "action_items": ["suggested priority 1", "suggested priority 2"]
}}"#)
}
```

---

## 3.3 Processing Pipeline

Create `src-tauri/src/ai/pipeline.rs`:

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;
use crate::db::Database;
use crate::crypto::CryptoService;
use super::gemini::GeminiClient;
use super::prompts::{self, SummaryResult};

pub struct ProcessingPipeline {
    gemini: GeminiClient,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
    concurrency: Arc<Semaphore>,
}

impl ProcessingPipeline {
    pub fn new(api_key: String, db: Arc<Database>, crypto: Arc<CryptoService>) -> Self {
        Self {
            gemini: GeminiClient::new(api_key),
            db,
            crypto,
            concurrency: Arc::new(Semaphore::new(3)), // Max 3 concurrent API calls
        }
    }

    /// Process all unprocessed content items
    pub async fn process_pending(&self) -> Result<i32, String> {
        let items: Vec<(String, String, String, Option<String>, Option<String>, Option<String>)> = 
            sqlx::query_as(
                "SELECT ci.id, ci.source, ci.content_type, ci.title, ci.body, ci.channel_or_project
                 FROM content_items ci
                 LEFT JOIN ai_summaries s ON ci.id = s.content_item_id
                 WHERE s.id IS NULL
                 LIMIT 50"
            )
            .fetch_all(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;

        let mut processed = 0;
        
        for (id, source, content_type, title, body, channel) in items {
            let _permit = self.concurrency.acquire().await.unwrap();
            
            if let Err(e) = self.process_item(&id, &source, &content_type, title, body, channel).await {
                tracing::error!("Failed to process item {}: {}", id, e);
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
                title.as_deref().unwrap_or(""),
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

        // Store the summary
        let now = chrono::Utc::now().timestamp();
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
        let start_ts = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| e.to_string())?
            .and_hms_opt(0, 0, 0).unwrap()
            .and_utc()
            .timestamp();
        let end_ts = start_ts + 86400;

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

        // Store digest
        let now = chrono::Utc::now().timestamp();
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
```

Update `src-tauri/src/ai/mod.rs`:

```rust
pub mod gemini;
pub mod prompts;
pub mod pipeline;

pub use gemini::GeminiClient;
pub use pipeline::ProcessingPipeline;
pub use prompts::{SummaryResult, DigestSummary};
```

---

## Verification

- [ ] Gemini client can generate text responses
- [ ] JSON structured output works correctly
- [ ] Processing pipeline fetches unprocessed items
- [ ] Summaries are stored with proper categorization
- [ ] Daily digest aggregates items correctly

---

## Next Steps

Proceed to **Phase 4: Frontend Development** to build the React UI.
