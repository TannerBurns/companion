use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Source {
    Slack,
    Jira,
    Confluence,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum ContentType {
    Message,
    Ticket,
    Page,
    Comment,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum SummaryType {
    Item,
    Daily,
    Weekly,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Category {
    Sales,
    Marketing,
    Product,
    Engineering,
    Research,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum SyncStatus {
    Pending,
    Syncing,
    Complete,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContentItem {
    pub id: String,
    pub source: String,
    pub source_id: String,
    pub source_url: Option<String>,
    pub content_type: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub author: Option<String>,
    pub author_id: Option<String>,
    pub channel_or_project: Option<String>,
    pub parent_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub synced_at: i64,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AiSummary {
    pub id: String,
    pub content_item_id: Option<String>,
    pub summary_type: String,
    pub summary: String,
    pub highlights: Option<String>,
    pub category: Option<String>,
    pub category_confidence: Option<f64>,
    pub importance_score: Option<f64>,
    pub entities: Option<String>,
    pub generated_at: i64,
    pub user_override_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncState {
    pub id: String,
    pub source: String,
    pub resource_type: String,
    pub resource_id: String,
    pub last_sync_at: Option<i64>,
    pub cursor: Option<String>,
    pub status: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Preference {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Credential {
    pub id: String,
    pub service: String,
    pub encrypted_data: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnalyticsEvent {
    pub id: i64,
    pub event_type: String,
    pub event_data: Option<String>,
    pub created_at: i64,
}
