use serde::{Deserialize, Serialize};

/// Result of summarizing a single content item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResult {
    pub summary: String,
    pub highlights: Vec<String>,
    pub category: String,
    pub category_confidence: f64,
    pub importance_score: f64,
    pub entities: Entities,
}

/// Entities extracted from content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entities {
    pub people: Vec<String>,
    pub projects: Vec<String>,
    pub topics: Vec<String>,
}

/// Result of generating a daily/weekly digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestSummary {
    pub summary: String,
    pub key_themes: Vec<String>,
    pub top_items: Vec<TopItem>,
    pub action_items: Vec<String>,
}

/// A top item in a digest summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopItem {
    pub title: String,
    pub reason: String,
}

/// Result of generating a weekly breakdown for status updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyBreakdown {
    #[serde(default)]
    pub major: Vec<String>,
    #[serde(default)]
    pub focus: Vec<String>,
    #[serde(default)]
    pub obstacles: Vec<String>,
    #[serde(default)]
    pub informational: Vec<String>,
}

/// Result of batch analysis with grouped content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupedAnalysisResult {
    pub groups: Vec<ContentGroup>,
    pub ungrouped: Vec<UngroupedItem>,
    pub daily_summary: String,
    pub key_themes: Vec<String>,
    pub action_items: Vec<String>,
}

/// A group of related content across channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentGroup {
    pub topic: String,
    pub channels: Vec<String>,
    pub summary: String,
    pub highlights: Vec<String>,
    pub category: String,
    pub importance_score: f64,
    pub message_ids: Vec<String>,
    pub people: Vec<String>,
    /// Stable ID for topic continuity (hash of topic + date)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_id: Option<String>,
}

/// An item that couldn't be grouped with others.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UngroupedItem {
    pub message_id: String,
    pub summary: String,
    pub category: String,
    pub importance_score: f64,
}

/// Represents an existing topic from a previous sync cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistingTopic {
    pub topic_id: String,
    pub topic: String,
    pub channels: Vec<String>,
    pub summary: String,
    pub category: String,
    pub importance_score: f64,
    pub message_count: i32,
    pub people: Vec<String>,
}

/// Channel summary for hierarchical summarization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSummary {
    pub channel: String,
    pub summary: String,
    pub key_topics: Vec<String>,
    pub key_people: Vec<String>,
    pub importance_score: f64,
    pub notable_message_ids: Vec<String>,
    pub message_count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_result_serialization() {
        let result = SummaryResult {
            summary: "Test summary".into(),
            highlights: vec!["point 1".into(), "point 2".into()],
            category: "engineering".into(),
            category_confidence: 0.95,
            importance_score: 0.8,
            entities: Entities {
                people: vec!["Alice".into()],
                projects: vec!["Project X".into()],
                topics: vec!["testing".into()],
            },
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: SummaryResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.summary, "Test summary");
        assert_eq!(parsed.category, "engineering");
        assert_eq!(parsed.category_confidence, 0.95);
        assert_eq!(parsed.highlights.len(), 2);
        assert_eq!(parsed.entities.people[0], "Alice");
    }

    #[test]
    fn test_digest_summary_serialization() {
        let digest = DigestSummary {
            summary: "Daily summary".into(),
            key_themes: vec!["theme1".into(), "theme2".into()],
            top_items: vec![TopItem {
                title: "Item 1".into(),
                reason: "Important".into(),
            }],
            action_items: vec!["Action 1".into()],
        };

        let json = serde_json::to_string(&digest).unwrap();
        let parsed: DigestSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.summary, "Daily summary");
        assert_eq!(parsed.key_themes.len(), 2);
        assert_eq!(parsed.top_items[0].title, "Item 1");
        assert_eq!(parsed.action_items[0], "Action 1");
    }

    #[test]
    fn test_grouped_analysis_result_serialization() {
        let result = GroupedAnalysisResult {
            groups: vec![ContentGroup {
                topic: "Product Launch".into(),
                channels: vec!["product".into(), "marketing".into()],
                summary: "Discussion about launch".into(),
                highlights: vec!["Key point".into()],
                category: "product".into(),
                importance_score: 0.85,
                message_ids: vec!["msg1".into(), "msg2".into()],
                people: vec!["Alice".into()],
                topic_id: None,
            }],
            ungrouped: vec![UngroupedItem {
                message_id: "msg3".into(),
                summary: "Standalone message".into(),
                category: "other".into(),
                importance_score: 0.3,
            }],
            daily_summary: "Busy day with product discussions".into(),
            key_themes: vec!["product".into(), "launch".into()],
            action_items: vec!["Review launch plan".into()],
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: GroupedAnalysisResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.groups.len(), 1);
        assert_eq!(parsed.groups[0].topic, "Product Launch");
        assert_eq!(parsed.groups[0].channels.len(), 2);
        assert_eq!(parsed.ungrouped.len(), 1);
        assert_eq!(parsed.daily_summary, "Busy day with product discussions");
    }

    #[test]
    fn test_content_group_serialization() {
        let group = ContentGroup {
            topic: "Test Topic".into(),
            channels: vec!["test".into()],
            summary: "Summary".into(),
            highlights: vec!["highlight".into()],
            category: "engineering".into(),
            importance_score: 0.7,
            message_ids: vec!["id1".into()],
            people: vec!["Bob".into()],
            topic_id: None,
        };

        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("Test Topic"));
        assert!(json.contains("engineering"));
        assert!(json.contains("Bob"));
        // topic_id is None so should not be serialized
        assert!(!json.contains("topic_id"));
    }

    #[test]
    fn test_content_group_with_topic_id() {
        let group = ContentGroup {
            topic: "Q1 Launch".into(),
            channels: vec!["product".into()],
            summary: "Launch discussion".into(),
            highlights: vec!["Launch date set".into()],
            category: "product".into(),
            importance_score: 0.9,
            message_ids: vec!["msg1".into()],
            people: vec!["Alice".into()],
            topic_id: Some("topic_abc123".into()),
        };

        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("topic_id"));
        assert!(json.contains("topic_abc123"));

        let parsed: ContentGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.topic_id, Some("topic_abc123".into()));
    }

    #[test]
    fn test_content_group_deserialize_without_topic_id() {
        let json = r##"{
            "topic": "Test Topic",
            "channels": ["#general"],
            "summary": "A summary",
            "highlights": ["point 1"],
            "category": "engineering",
            "importance_score": 0.8,
            "message_ids": ["msg1"],
            "people": ["Alice"]
        }"##;

        let parsed: ContentGroup = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.topic, "Test Topic");
        assert_eq!(parsed.topic_id, None);
    }

    #[test]
    fn test_content_group_deserialize_with_null_topic_id() {
        let json = r##"{
            "topic": "Test Topic",
            "channels": ["#general"],
            "summary": "A summary",
            "highlights": ["point 1"],
            "category": "engineering",
            "importance_score": 0.8,
            "message_ids": ["msg1"],
            "people": ["Alice"],
            "topic_id": null
        }"##;

        let parsed: ContentGroup = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.topic_id, None);
    }

    #[test]
    fn test_existing_topic_serialization() {
        let existing = ExistingTopic {
            topic_id: "topic_xyz789".into(),
            topic: "Sprint Planning".into(),
            channels: vec!["engineering".into(), "product".into()],
            summary: "Discussed sprint goals".into(),
            category: "engineering".into(),
            importance_score: 0.8,
            message_count: 15,
            people: vec!["Bob".into(), "Carol".into()],
        };

        let json = serde_json::to_string(&existing).unwrap();
        assert!(json.contains("topic_xyz789"));
        assert!(json.contains("Sprint Planning"));
        assert!(json.contains("message_count"));

        let parsed: ExistingTopic = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.topic_id, "topic_xyz789");
        assert_eq!(parsed.message_count, 15);
    }

    #[test]
    fn test_existing_topic_all_fields_required() {
        let json = r#"{
            "topic_id": "t1",
            "topic": "Topic",
            "channels": [],
            "summary": "Sum",
            "category": "other",
            "importance_score": 0.5,
            "message_count": 0,
            "people": []
        }"#;

        let parsed: ExistingTopic = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.topic_id, "t1");
        assert_eq!(parsed.message_count, 0);
        assert!(parsed.channels.is_empty());
    }

    #[test]
    fn test_channel_summary_serialization() {
        let summary = ChannelSummary {
            channel: "engineering".into(),
            summary: "Discussion about API design".into(),
            key_topics: vec!["api".into(), "design".into()],
            key_people: vec!["alice".into(), "bob".into()],
            importance_score: 0.8,
            notable_message_ids: vec!["msg1".into(), "msg2".into()],
            message_count: 42,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let parsed: ChannelSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.channel, "engineering");
        assert_eq!(parsed.key_topics.len(), 2);
        assert_eq!(parsed.message_count, 42);
    }

    #[test]
    fn test_content_group_topic_id_roundtrip() {
        let group = ContentGroup {
            topic: "Roundtrip Test".into(),
            channels: vec!["test".into()],
            summary: "Summary".into(),
            highlights: vec![],
            category: "other".into(),
            importance_score: 0.5,
            message_ids: vec![],
            people: vec![],
            topic_id: Some("topic_roundtrip_123".into()),
        };

        let json = serde_json::to_string(&group).unwrap();
        let parsed: ContentGroup = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.topic_id, Some("topic_roundtrip_123".into()));
    }
}
