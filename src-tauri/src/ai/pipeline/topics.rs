use sha2::{Sha256, Digest};
use crate::ai::prompts::ExistingTopic;
use super::types::ExistingTopicRow;

/// Generate a deterministic topic ID based on topic name and date.
/// 
/// The ID is case-insensitive to ensure related topics are grouped together
/// even with minor case variations.
/// 
/// # Arguments
/// * `topic` - The topic name/title
/// * `date` - The date string in YYYY-MM-DD format
/// 
/// # Returns
/// A string in the format `topic_<hex>` where hex is derived from SHA256 hash
pub fn generate_topic_id(topic: &str, date: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(topic.to_lowercase().as_bytes());
    hasher.update(date.as_bytes());
    let result = hasher.finalize();
    format!("topic_{:x}", &result[..8].iter().fold(0u64, |acc, &b| (acc << 8) | b as u64))
}

/// Convert database rows to ExistingTopic structs for AI prompts.
/// 
/// Returns a tuple of:
/// - Map of topic IDs to their message IDs (for merge tracking)
/// - List of ExistingTopic structs (only for rows with valid topic fields)
pub fn convert_existing_topics(
    rows: &[ExistingTopicRow],
) -> (std::collections::HashMap<String, Vec<String>>, Vec<ExistingTopic>) {
    let mut message_ids_map: std::collections::HashMap<String, Vec<String>> = 
        std::collections::HashMap::new();
    let mut existing_topics: Vec<ExistingTopic> = Vec::new();
    
    for row in rows {
        let entities: serde_json::Value = row.entities.as_ref()
            .and_then(|e| serde_json::from_str(e).ok())
            .unwrap_or(serde_json::json!({}));
        
        // Only process rows that have a valid topic field
        let Some(topic) = entities.get("topic").and_then(|v| v.as_str()) else {
            continue;
        };
        
        let message_ids: Vec<String> = entities.get("message_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let message_count: i32 = message_ids.len().try_into().unwrap_or(i32::MAX);
        
        // Cache message IDs for merge tracking
        message_ids_map.insert(row.id.clone(), message_ids);
        
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
    
    (message_ids_map, existing_topics)
}

/// Merge new message IDs with existing ones, preserving order and avoiding duplicates.
pub fn merge_message_ids(existing: &[String], new: &[String]) -> Vec<String> {
    let mut merged = existing.to_vec();
    for msg_id in new {
        if !merged.contains(msg_id) {
            merged.push(msg_id.clone());
        }
    }
    merged
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
    fn test_merge_message_ids_basic() {
        let existing = vec!["msg1".into(), "msg2".into(), "msg3".into()];
        let new = vec!["msg3".into(), "msg4".into(), "msg5".into()];
        
        let merged = merge_message_ids(&existing, &new);
        
        assert_eq!(merged.len(), 5);
        assert_eq!(merged, vec!["msg1", "msg2", "msg3", "msg4", "msg5"]);
    }

    #[test]
    fn test_merge_message_ids_preserves_order() {
        let existing = vec!["a".into(), "b".into(), "c".into()];
        let new = vec!["d".into(), "e".into()];
        
        let merged = merge_message_ids(&existing, &new);
        
        assert_eq!(merged, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn test_merge_message_ids_empty_existing() {
        let existing: Vec<String> = vec![];
        let new = vec!["msg1".into(), "msg2".into()];
        
        let merged = merge_message_ids(&existing, &new);
        
        assert_eq!(merged.len(), 2);
        assert_eq!(merged, vec!["msg1", "msg2"]);
    }

    #[test]
    fn test_merge_message_ids_empty_new() {
        let existing = vec!["msg1".into(), "msg2".into()];
        let new: Vec<String> = vec![];
        
        let merged = merge_message_ids(&existing, &new);
        
        assert_eq!(merged.len(), 2);
        assert_eq!(merged, vec!["msg1", "msg2"]);
    }

    #[test]
    fn test_merge_message_ids_all_duplicates() {
        let existing = vec!["msg1".into(), "msg2".into()];
        let new = vec!["msg1".into(), "msg2".into()];
        
        let merged = merge_message_ids(&existing, &new);
        
        assert_eq!(merged, vec!["msg1", "msg2"]);
    }

    #[test]
    fn test_convert_existing_topics_with_valid_rows() {
        let rows = vec![
            ExistingTopicRow {
                id: "topic_valid".to_string(),
                summary: "Valid topic".to_string(),
                category: Some("engineering".to_string()),
                importance_score: Some(0.8),
                entities: Some(r##"{"topic": "Valid Topic", "channels": ["#dev"], "people": ["Alice"], "message_ids": ["msg1", "msg2"]}"##.to_string()),
            },
        ];
        
        let (message_ids_map, existing_topics) = convert_existing_topics(&rows);
        
        assert_eq!(existing_topics.len(), 1);
        assert_eq!(existing_topics[0].topic_id, "topic_valid");
        assert_eq!(existing_topics[0].topic, "Valid Topic");
        assert_eq!(existing_topics[0].channels, vec!["#dev"]);
        assert_eq!(existing_topics[0].people, vec!["Alice"]);
        assert_eq!(existing_topics[0].message_count, 2);
        
        assert_eq!(message_ids_map.len(), 1);
        assert_eq!(
            message_ids_map.get("topic_valid"),
            Some(&vec!["msg1".to_string(), "msg2".to_string()])
        );
    }

    #[test]
    fn test_convert_existing_topics_skips_rows_without_topic() {
        let rows = vec![
            ExistingTopicRow {
                id: "topic_valid".to_string(),
                summary: "Valid topic".to_string(),
                category: Some("engineering".to_string()),
                importance_score: Some(0.8),
                entities: Some(r##"{"topic": "Valid Topic", "channels": [], "people": [], "message_ids": ["msg1"]}"##.to_string()),
            },
            ExistingTopicRow {
                id: "topic_malformed".to_string(),
                summary: "Malformed entry".to_string(),
                category: Some("other".to_string()),
                importance_score: Some(0.5),
                entities: Some(r##"{"channels": [], "people": [], "message_ids": ["msg2", "msg3"]}"##.to_string()),
            },
        ];
        
        let (message_ids_map, existing_topics) = convert_existing_topics(&rows);
        
        // Only valid topic should be in existing_topics
        assert_eq!(existing_topics.len(), 1);
        assert_eq!(existing_topics[0].topic_id, "topic_valid");
        
        // But message_ids_map should only contain rows that made it to existing_topics
        // (rows with valid topic field)
        assert_eq!(message_ids_map.len(), 1);
    }

    #[test]
    fn test_convert_existing_topics_handles_invalid_json() {
        let rows = vec![
            ExistingTopicRow {
                id: "topic_bad".to_string(),
                summary: "Bad JSON".to_string(),
                category: None,
                importance_score: None,
                entities: Some("not valid json".to_string()),
            },
        ];
        
        let (message_ids_map, existing_topics) = convert_existing_topics(&rows);
        
        assert!(existing_topics.is_empty());
        assert!(message_ids_map.is_empty());
    }

    #[test]
    fn test_convert_existing_topics_handles_none_entities() {
        let rows = vec![
            ExistingTopicRow {
                id: "topic_none".to_string(),
                summary: "No entities".to_string(),
                category: None,
                importance_score: None,
                entities: None,
            },
        ];
        
        let (message_ids_map, existing_topics) = convert_existing_topics(&rows);
        
        assert!(existing_topics.is_empty());
        assert!(message_ids_map.is_empty());
    }

    #[test]
    fn test_convert_existing_topics_with_missing_message_ids() {
        // Test case: valid topic but missing message_ids field
        // This documents that an empty vector is inserted into message_ids_map
        // storage::store_results handles this by falling back to DB fetch when vector is empty
        let rows = vec![
            ExistingTopicRow {
                id: "topic_no_msgs".to_string(),
                summary: "Topic without message_ids".to_string(),
                category: Some("engineering".to_string()),
                importance_score: Some(0.7),
                entities: Some(r##"{"topic": "Missing Message IDs", "channels": ["#general"], "people": ["Bob"]}"##.to_string()),
            },
        ];
        
        let (message_ids_map, existing_topics) = convert_existing_topics(&rows);
        
        // Topic should still be processed
        assert_eq!(existing_topics.len(), 1);
        assert_eq!(existing_topics[0].topic, "Missing Message IDs");
        assert_eq!(existing_topics[0].message_count, 0);
        
        // message_ids_map contains an empty vector for this topic
        // storage::store_results should fall back to DB fetch when it encounters this
        assert_eq!(message_ids_map.len(), 1);
        assert_eq!(message_ids_map.get("topic_no_msgs"), Some(&vec![]));
    }

    #[test]
    fn test_convert_existing_topics_with_invalid_message_ids_type() {
        // Test case: message_ids exists but is wrong type (string instead of array)
        let rows = vec![
            ExistingTopicRow {
                id: "topic_bad_type".to_string(),
                summary: "Topic with wrong message_ids type".to_string(),
                category: Some("product".to_string()),
                importance_score: Some(0.6),
                entities: Some(r##"{"topic": "Wrong Type", "channels": [], "people": [], "message_ids": "not_an_array"}"##.to_string()),
            },
        ];
        
        let (message_ids_map, existing_topics) = convert_existing_topics(&rows);
        
        assert_eq!(existing_topics.len(), 1);
        assert_eq!(existing_topics[0].message_count, 0);
        
        // Empty vector because parsing failed
        assert_eq!(message_ids_map.get("topic_bad_type"), Some(&vec![]));
    }
}
