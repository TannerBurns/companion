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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupedAnalysisResult {
    pub groups: Vec<ContentGroup>,
    pub ungrouped: Vec<UngroupedItem>,
    pub daily_summary: String,
    pub key_themes: Vec<String>,
    pub action_items: Vec<String>,
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UngroupedItem {
    pub message_id: String,
    pub summary: String,
    pub category: String,
    pub importance_score: f64,
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
    // Truncate at a safe UTF-8 boundary to avoid panics with multi-byte characters
    let truncated = if content.len() > 8000 {
        let mut end = 8000;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        &content[..end]
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

pub fn batch_analysis_prompt(date: &str, messages_json: &str) -> String {
    format!(r##"You are analyzing all messages from {date} across multiple Slack channels and direct messages.

Your task is to:
1. Identify related discussions that span multiple channels (e.g., a product launch discussed in #product, #marketing, and #sales)
2. Group related messages together by topic/theme
3. Summarize each group
4. Categorize each group (sales, marketing, product, engineering, research, or other)
5. Identify standalone messages that don't fit into any group
6. Create an executive summary of the entire day

Here are all the messages (each includes: id, channel, author, timestamp, and text):

{messages_json}

Return JSON with this exact structure:
{{
  "groups": [
    {{
      "topic": "Clear, descriptive topic name (e.g., 'Q1 Product Launch Planning')",
      "channels": ["#channel1", "#channel2", "DM: Person1 & Person2"],
      "summary": "2-4 sentence summary of this discussion across all channels",
      "highlights": ["key point 1", "key point 2", "key point 3"],
      "category": "one of: sales, marketing, product, engineering, research, other",
      "importance_score": 0.0-1.0,
      "message_ids": ["id1", "id2", "id3"],
      "people": ["person1", "person2"]
    }}
  ],
  "ungrouped": [
    {{
      "message_id": "id",
      "summary": "Brief 1-sentence summary",
      "category": "category",
      "importance_score": 0.0-1.0
    }}
  ],
  "daily_summary": "3-4 sentence executive summary of the day's key activities and themes",
  "key_themes": ["theme 1", "theme 2", "theme 3"],
  "action_items": ["suggested action 1", "suggested action 2"]
}}

Guidelines:
- Group messages that discuss the SAME topic, project, or issue, even across different channels
- A single message can only belong to ONE group (use message_ids to track)
- Low-content messages (just emojis, "ok", "thanks") should go in ungrouped with low importance
- importance_score: 0.9-1.0 for critical business decisions, 0.6-0.8 for important updates, 0.3-0.5 for routine, 0.0-0.2 for noise
- Identify action items that emerge from discussions
- The daily_summary should give an executive the key takeaways in 30 seconds"##)
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
            top_items: vec![
                TopItem { title: "Item 1".into(), reason: "Important".into() },
            ],
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
    fn test_slack_message_prompt_contains_channel() {
        let prompt = slack_message_prompt("general", "Hello world");
        assert!(prompt.contains("#general"));
        assert!(prompt.contains("Hello world"));
        assert!(prompt.contains("Slack conversation"));
    }

    #[test]
    fn test_jira_issue_prompt_contains_fields() {
        let prompt = jira_issue_prompt("PROJ-123", "Fix bug", "Description here");
        assert!(prompt.contains("PROJ-123"));
        assert!(prompt.contains("Fix bug"));
        assert!(prompt.contains("Description here"));
        assert!(prompt.contains("Jira issue"));
    }

    #[test]
    fn test_confluence_page_prompt_contains_fields() {
        let prompt = confluence_page_prompt("My Page", "Engineering", "Page content");
        assert!(prompt.contains("My Page"));
        assert!(prompt.contains("Engineering"));
        assert!(prompt.contains("Page content"));
        assert!(prompt.contains("Confluence page"));
    }

    #[test]
    fn test_confluence_page_prompt_truncates_long_content() {
        let long_content = "x".repeat(10000);
        let prompt = confluence_page_prompt("Title", "Space", &long_content);
        
        // Should contain truncated content (8000 chars), not full 10000
        assert!(prompt.len() < 10000);
        assert!(prompt.contains(&"x".repeat(100))); // Still has some content
    }

    #[test]
    fn test_confluence_page_prompt_handles_multibyte_utf8() {
        // Create content with multi-byte UTF-8 characters (emoji is 4 bytes each)
        // Position truncation to land in the middle of a multi-byte char
        let prefix = "x".repeat(7998);
        let emoji_content = format!("{}🎉🎉🎉", prefix); // 7998 + 12 bytes = 8010 bytes
        
        // This should not panic and should truncate at a valid UTF-8 boundary
        let prompt = confluence_page_prompt("Title", "Space", &emoji_content);
        
        // Verify it's valid UTF-8 (would have panicked if sliced incorrectly)
        assert!(prompt.is_char_boundary(0));
        assert!(prompt.contains(&"x".repeat(100)));
    }

    #[test]
    fn test_daily_digest_prompt_contains_date() {
        let prompt = daily_digest_prompt("2024-01-15", r#"[{"summary": "test"}]"#);
        assert!(prompt.contains("2024-01-15"));
        assert!(prompt.contains("daily digest"));
    }

    #[test]
    fn test_weekly_digest_prompt_contains_week_start() {
        let prompt = weekly_digest_prompt("2024-01-08", "Monday summary\nTuesday summary");
        assert!(prompt.contains("2024-01-08"));
        assert!(prompt.contains("weekly digest"));
        assert!(prompt.contains("Monday summary"));
    }

    #[test]
    fn test_batch_analysis_prompt_contains_date_and_messages() {
        let messages = r##"[{"id": "1", "channel": "#general", "text": "Hello"}]"##;
        let prompt = batch_analysis_prompt("2024-01-15", messages);
        assert!(prompt.contains("2024-01-15"));
        assert!(prompt.contains("#general"));
        assert!(prompt.contains("Hello"));
        assert!(prompt.contains("groups"));
        assert!(prompt.contains("ungrouped"));
        assert!(prompt.contains("daily_summary"));
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
        assert_eq!(parsed.key_themes.len(), 2);
        assert_eq!(parsed.action_items.len(), 1);
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
        };

        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("Test Topic"));
        assert!(json.contains("engineering"));
        assert!(json.contains("Bob"));
    }
}
