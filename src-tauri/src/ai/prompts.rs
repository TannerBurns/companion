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
}
