use serde::{Deserialize, Serialize};

/// A single item in the daily/weekly digest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DigestItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub highlights: Option<Vec<String>>,
    pub category: String,
    pub source: String,
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_urls: Option<Vec<String>>,
    pub importance_score: f64,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub people: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<i32>,
}

/// Response containing digest items organized by category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestResponse {
    pub date: String,
    pub items: Vec<DigestItem>,
    pub categories: Vec<CategorySummary>,
}

/// Summary of items in a category
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySummary {
    pub name: String,
    pub count: i32,
    pub top_items: Vec<DigestItem>,
}

/// Current sync status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub is_syncing: bool,
    pub last_sync_at: Option<i64>,
    pub next_sync_at: Option<i64>,
    pub sources: Vec<SourceStatus>,
}

/// Status of a single sync source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub name: String,
    pub status: String,
    pub items_synced: i32,
    pub last_error: Option<String>,
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub items_synced: i32,
    pub channels_processed: i32,
    pub errors: Vec<String>,
}

/// User preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub sync_interval_minutes: i32,
    pub enabled_sources: Vec<String>,
    pub enabled_categories: Vec<String>,
    pub notifications_enabled: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            sync_interval_minutes: 15,
            enabled_sources: vec![],
            enabled_categories: vec![
                "sales".to_string(),
                "marketing".to_string(),
                "product".to_string(),
                "engineering".to_string(),
                "research".to_string(),
            ],
            notifications_enabled: true,
        }
    }
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStats {
    pub content_items: i64,
    pub ai_summaries: i64,
    pub slack_users: i64,
    pub sync_states: i64,
}

/// Result of clearing data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearDataResult {
    pub items_deleted: i64,
}

/// Analytics summary
#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsSummary {
    pub event_counts: std::collections::HashMap<String, i64>,
    pub days: i32,
}

/// Type alias for group row from database
pub type GroupRow = (String, String, Option<String>, Option<String>, Option<f64>, Option<String>, i64);

pub struct ParsedEntities {
    pub title: String,
    pub channels: Option<Vec<String>>,
    pub people: Option<Vec<String>>,
    pub message_count: Option<i32>,
    pub message_ids: Vec<String>,
    pub key_message_ids: Vec<String>,
}

impl ParsedEntities {
    pub fn from_json(entities: &Option<String>) -> Self {
        let value: serde_json::Value = entities.as_ref()
            .and_then(|e| serde_json::from_str(e).ok())
            .unwrap_or(serde_json::json!({}));
        
        let title = value["topic"].as_str()
            .map(String::from)
            .unwrap_or_else(|| "Discussion".to_string());
        
        let channels: Option<Vec<String>> = value.get("channels")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .filter(|v: &Vec<String>| !v.is_empty());
        
        let people: Option<Vec<String>> = value.get("people")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .filter(|v: &Vec<String>| !v.is_empty());
        
        let message_ids: Vec<String> = value.get("message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        
        let key_message_ids: Vec<String> = value.get("key_message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        
        let message_count: Option<i32> = if message_ids.is_empty() {
            None
        } else {
            Some(message_ids.len() as i32)
        };
        
        Self { title, channels, people, message_count, message_ids, key_message_ids }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digest_item_serialization() {
        let item = DigestItem {
            id: "test-1".to_string(),
            title: "Test Title".to_string(),
            summary: "Test summary".to_string(),
            highlights: Some(vec!["highlight 1".to_string()]),
            category: "engineering".to_string(),
            source: "slack".to_string(),
            source_url: None,
            source_urls: Some(vec!["https://slack.com/msg1".to_string(), "https://slack.com/msg2".to_string()]),
            importance_score: 0.8,
            created_at: 1234567890,
            channels: Some(vec!["#general".to_string()]),
            people: None,
            message_count: Some(5),
        };
        
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"id\":\"test-1\""));
        assert!(json.contains("\"importanceScore\":0.8"));
        assert!(json.contains("\"sourceUrls\""));
        // people is None so should not be serialized
        assert!(!json.contains("\"people\""));
    }

    #[test]
    fn test_digest_item_deserialization() {
        let json = r#"{
            "id": "test-1",
            "title": "Test",
            "summary": "Summary",
            "category": "product",
            "source": "slack",
            "importanceScore": 0.5,
            "createdAt": 1000
        }"#;
        
        let item: DigestItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "test-1");
        assert_eq!(item.importance_score, 0.5);
        assert!(item.highlights.is_none());
    }

    #[test]
    fn test_preferences_default() {
        let prefs = Preferences::default();
        assert_eq!(prefs.sync_interval_minutes, 15);
        assert!(prefs.notifications_enabled);
        assert!(prefs.enabled_categories.contains(&"engineering".to_string()));
    }

    #[test]
    fn test_preferences_serialization() {
        let prefs = Preferences {
            sync_interval_minutes: 30,
            enabled_sources: vec!["slack".to_string()],
            enabled_categories: vec!["sales".to_string()],
            notifications_enabled: false,
        };
        
        let json = serde_json::to_string(&prefs).unwrap();
        assert!(json.contains("\"syncIntervalMinutes\":30"));
        assert!(json.contains("\"notificationsEnabled\":false"));
    }

    #[test]
    fn test_sync_result_serialization() {
        let result = SyncResult {
            items_synced: 10,
            channels_processed: 3,
            errors: vec!["error1".to_string()],
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"itemsSynced\":10"));
        assert!(json.contains("\"channelsProcessed\":3"));
    }

    #[test]
    fn test_parsed_entities_from_json_full() {
        let entities = Some(r##"{
            "topic": "Sprint Planning",
            "channels": ["#engineering", "#product"],
            "people": ["Alice", "Bob"],
            "message_ids": ["msg1", "msg2", "msg3"],
            "key_message_ids": ["msg1", "msg3"]
        }"##.to_string());
        
        let parsed = ParsedEntities::from_json(&entities);
        assert_eq!(parsed.title, "Sprint Planning");
        assert_eq!(parsed.channels.unwrap(), vec!["#engineering", "#product"]);
        assert_eq!(parsed.people.unwrap(), vec!["Alice", "Bob"]);
        assert_eq!(parsed.message_count, Some(3));
        assert_eq!(parsed.message_ids, vec!["msg1", "msg2", "msg3"]);
        assert_eq!(parsed.key_message_ids, vec!["msg1", "msg3"]);
    }

    #[test]
    fn test_parsed_entities_from_json_empty() {
        let entities: Option<String> = None;
        let parsed = ParsedEntities::from_json(&entities);
        assert_eq!(parsed.title, "Discussion");
        assert!(parsed.channels.is_none());
        assert!(parsed.people.is_none());
        assert!(parsed.message_count.is_none());
        assert!(parsed.message_ids.is_empty());
        assert!(parsed.key_message_ids.is_empty());
    }

    #[test]
    fn test_parsed_entities_from_json_partial() {
        let entities = Some(r#"{"topic": "Bug Fix"}"#.to_string());
        let parsed = ParsedEntities::from_json(&entities);
        assert_eq!(parsed.title, "Bug Fix");
        assert!(parsed.channels.is_none());
        assert!(parsed.message_ids.is_empty());
        assert!(parsed.key_message_ids.is_empty());
    }
    
    #[test]
    fn test_parsed_entities_fallback_to_message_ids() {
        // When key_message_ids is missing, it should be empty
        let entities = Some(r##"{
            "topic": "Discussion",
            "message_ids": ["msg1", "msg2"]
        }"##.to_string());
        
        let parsed = ParsedEntities::from_json(&entities);
        assert_eq!(parsed.message_ids, vec!["msg1", "msg2"]);
        assert!(parsed.key_message_ids.is_empty());
    }

    #[test]
    fn test_parsed_entities_filters_empty_arrays() {
        let entities = Some(r#"{
            "topic": "Test",
            "channels": [],
            "people": []
        }"#.to_string());
        
        let parsed = ParsedEntities::from_json(&entities);
        assert!(parsed.channels.is_none());
        assert!(parsed.people.is_none());
    }

    #[test]
    fn test_data_stats_serialization() {
        let stats = DataStats {
            content_items: 100,
            ai_summaries: 50,
            slack_users: 25,
            sync_states: 10,
        };
        
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"contentItems\":100"));
        assert!(json.contains("\"slackUsers\":25"));
    }

    #[test]
    fn test_sync_status_serialization() {
        let status = SyncStatus {
            is_syncing: true,
            last_sync_at: Some(1234567890),
            next_sync_at: None,
            sources: vec![SourceStatus {
                name: "slack".to_string(),
                status: "connected".to_string(),
                items_synced: 50,
                last_error: None,
            }],
        };
        
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"isSyncing\":true"));
        assert!(json.contains("\"lastSyncAt\":1234567890"));
    }
}
