use serde::Serialize;

pub const HIERARCHICAL_CHANNEL_THRESHOLD: usize = 50;
pub const HIERARCHICAL_TOTAL_THRESHOLD: usize = 200;
pub const HISTORICAL_AI_CHUNK_SIZE: i64 = 150;
pub const HIERARCHICAL_CHANNEL_CHUNK_SIZE: usize = 80;

/// Database row for content items
#[derive(sqlx::FromRow)]
pub struct ContentItemRow {
    pub id: String,
    pub source: String,
    pub content_type: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub author_id: Option<String>,
    pub channel_or_project: Option<String>,
    pub source_url: Option<String>,
    pub parent_id: Option<String>,
    pub created_at: i64,
}

/// Database row for Slack users
#[derive(sqlx::FromRow)]
pub struct SlackUserRow {
    pub user_id: String,
    pub real_name: Option<String>,
    pub display_name: Option<String>,
}

/// Message formatted for AI prompts
#[derive(Clone, Serialize)]
pub struct MessageForPrompt {
    pub id: String,
    pub channel: String,
    pub author: String,
    pub timestamp: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

/// Database row for existing topic summaries
#[derive(sqlx::FromRow)]
pub struct ExistingTopicRow {
    pub id: String,
    pub summary: String,
    pub category: Option<String>,
    pub importance_score: Option<f64>,
    pub entities: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_thresholds() {
        assert_eq!(HIERARCHICAL_CHANNEL_THRESHOLD, 50);
        assert_eq!(HIERARCHICAL_TOTAL_THRESHOLD, 200);
    }

    #[test]
    fn test_historical_ai_chunk_size() {
        assert_eq!(HISTORICAL_AI_CHUNK_SIZE, 150);
    }

    #[test]
    fn test_hierarchical_channel_chunk_size() {
        assert_eq!(HIERARCHICAL_CHANNEL_CHUNK_SIZE, 80);
    }

    #[test]
    fn test_message_for_prompt_clone() {
        let original = MessageForPrompt {
            id: "msg1".to_string(),
            channel: "#general".to_string(),
            author: "Alice".to_string(),
            timestamp: "10:30".to_string(),
            text: "Hello world".to_string(),
            url: Some("https://slack.com/msg1".to_string()),
            thread_id: Some("thread-123".to_string()),
        };

        let cloned = original.clone();

        assert_eq!(cloned.id, "msg1");
        assert_eq!(cloned.channel, "#general");
        assert_eq!(cloned.author, "Alice");
        assert_eq!(cloned.timestamp, "10:30");
        assert_eq!(cloned.text, "Hello world");
        assert_eq!(cloned.url, Some("https://slack.com/msg1".to_string()));
        assert_eq!(cloned.thread_id, Some("thread-123".to_string()));
    }

    #[test]
    fn test_existing_topic_row_fields() {
        let row = ExistingTopicRow {
            id: "test_id".to_string(),
            summary: "Test summary".to_string(),
            category: Some("engineering".to_string()),
            importance_score: Some(0.8),
            entities: Some(
                r#"{"topic": "Test", "channels": [], "people": [], "message_ids": []}"#.to_string(),
            ),
        };

        assert_eq!(row.id, "test_id");
        assert_eq!(row.summary, "Test summary");
        assert_eq!(row.category, Some("engineering".to_string()));
        assert_eq!(row.importance_score, Some(0.8));
        assert!(row.entities.is_some());
    }

    #[test]
    fn test_message_for_prompt_serialization() {
        let msg = MessageForPrompt {
            id: "msg1".to_string(),
            channel: "#general".to_string(),
            author: "Alice".to_string(),
            timestamp: "10:30".to_string(),
            text: "Hello world".to_string(),
            url: Some("https://slack.com/msg1".to_string()),
            thread_id: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"id\":\"msg1\""));
        assert!(json.contains("\"channel\":\"#general\""));
        assert!(json.contains("\"url\":\"https://slack.com/msg1\""));
        // thread_id is None so should not be serialized
        assert!(!json.contains("thread_id"));
    }

    #[test]
    fn test_message_for_prompt_with_thread() {
        let msg = MessageForPrompt {
            id: "msg2".to_string(),
            channel: "#dev".to_string(),
            author: "Bob".to_string(),
            timestamp: "11:00".to_string(),
            text: "Reply".to_string(),
            url: None,
            thread_id: Some("1234567890.123456".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"thread_id\":\"1234567890.123456\""));
        // url is None so should not be serialized
        assert!(!json.contains("\"url\""));
    }

    #[test]
    fn test_slack_user_row_display_name_preference() {
        let user = SlackUserRow {
            user_id: "U123".to_string(),
            real_name: Some("Alice Smith".to_string()),
            display_name: Some("alice".to_string()),
        };

        // Should prefer display_name over real_name
        let name = user
            .display_name
            .filter(|s| !s.is_empty())
            .or(user.real_name)
            .unwrap_or_else(|| user.user_id.clone());

        assert_eq!(name, "alice");
    }

    #[test]
    fn test_slack_user_row_fallback_to_real_name() {
        let user = SlackUserRow {
            user_id: "U123".to_string(),
            real_name: Some("Alice Smith".to_string()),
            display_name: Some("".to_string()),
        };

        let name = user
            .display_name
            .filter(|s| !s.is_empty())
            .or(user.real_name)
            .unwrap_or_else(|| user.user_id.clone());

        assert_eq!(name, "Alice Smith");
    }

    #[test]
    fn test_slack_user_row_fallback_to_user_id() {
        let user = SlackUserRow {
            user_id: "U123".to_string(),
            real_name: None,
            display_name: None,
        };

        let name = user
            .display_name
            .filter(|s| !s.is_empty())
            .or(user.real_name)
            .unwrap_or_else(|| user.user_id.clone());

        assert_eq!(name, "U123");
    }
}
