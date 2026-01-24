/// Generate a prompt for analyzing Slack messages.
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

/// Generate a prompt for analyzing Jira issues.
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

/// Generate a prompt for analyzing Confluence pages.
/// 
/// Content is automatically truncated at 8000 bytes at a valid UTF-8 boundary.
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

/// Generate a prompt for creating a daily digest.
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

/// Generate a prompt for creating a weekly digest.
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

/// Generate a prompt for batch analysis of messages.
/// 
/// This is a convenience wrapper around `batch_analysis_prompt_with_existing` with no existing topics.
pub fn batch_analysis_prompt(date: &str, messages_json: &str) -> String {
    batch_analysis_prompt_with_existing(date, messages_json, None)
}

/// Generate a prompt for batch analysis with optional existing topics.
/// 
/// When existing topics are provided, the AI is instructed to merge new messages
/// into existing topics where appropriate.
pub fn batch_analysis_prompt_with_existing(date: &str, messages_json: &str, existing_topics: Option<&str>) -> String {
    let existing_context = if let Some(topics_json) = existing_topics {
        format!(r##"
EXISTING TOPICS FROM EARLIER TODAY:
The following topics were already identified from earlier sync cycles today. When you encounter new messages that relate to these existing topics, you should MERGE them into the existing topic rather than creating a new one.

{topics_json}

IMPORTANT MERGING RULES:
- If new messages relate to an existing topic, include the existing topic_id in your response and UPDATE the summary/highlights to incorporate the new information
- For message_ids: only include the NEW message IDs from this batch (the system will automatically merge them with existing IDs)
- Update channels and people lists to include any new participants (combine with existing)
- Update the summary to reflect ALL information (existing + new)
- Only create a NEW topic if the discussion is genuinely different from all existing topics
- When updating an existing topic, use the SAME topic_id from the existing topic

"##)
    } else {
        String::new()
    };

    let topic_id_instruction = if existing_topics.is_some() {
        r#""topic_id": "topic_abc123","#
    } else {
        r#""topic_id": null,"#
    };

    format!(r##"You are analyzing all messages from {date} across multiple Slack channels and direct messages.
{existing_context}
Your task is to:
1. Identify related discussions that span multiple channels (e.g., a product launch discussed in #product, #marketing, and #sales)
2. Group related messages together by topic/theme
3. Summarize each group
4. Categorize each group (sales, marketing, product, engineering, research, or other)
5. Identify standalone messages that don't fit into any group
6. Create an executive summary of the entire day (incorporating all topics, both existing and new)

Here are the NEW messages to process (each includes: id, channel, author, timestamp, and text):

{messages_json}

Return JSON with this exact structure:
{{
  "groups": [
    {{
      {topic_id_instruction}
      "topic": "Clear, descriptive topic name (e.g., 'Q1 Product Launch Planning')",
      "channels": ["#channel1", "#channel2", "DM: Person1 & Person2"],
      "summary": "2-4 sentence summary of this discussion across all channels",
      "highlights": ["key point 1", "key point 2", "key point 3"],
      "category": "one of: sales, marketing, product, engineering, research, other",
      "importance_score": 0.0-1.0,
      "message_ids": ["id1", "id2", "id3"],
      "key_message_ids": ["id1", "id2"],
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
- The daily_summary should give an executive the key takeaways in 30 seconds
- topic_id: When updating an existing topic, copy the exact topic_id string from the existing topics list. For new topics, set topic_id to null
- key_message_ids: Select 1-3 of the MOST IMPORTANT messages that would be best for jumping back into the original conversation. Choose messages that provide the most context or contain key decisions/information. These will be shown as direct links to Slack."##)
}

/// Generate a prompt for summarizing a single channel.
/// 
/// Used in hierarchical summarization for high-volume channels.
pub fn channel_summary_prompt(channel: &str, purpose: Option<&str>, messages_json: &str) -> String {
    let purpose_line = purpose
        .map(|p| format!("Channel purpose: {}\n", p))
        .unwrap_or_default();
    
    format!(r##"Summarize the discussion in #{channel}.
{purpose_line}
Messages:
{messages_json}

Return JSON with this structure:
{{
  "channel": "{channel}",
  "summary": "2-3 sentence summary of the key discussions in this channel",
  "key_topics": ["topic1", "topic2", "topic3"],
  "key_people": ["person1", "person2"],
  "importance_score": 0.0-1.0,
  "notable_message_ids": ["id1", "id2"]
}}

Guidelines:
- Focus on the most significant discussions and decisions
- importance_score: 0.9-1.0 for critical decisions, 0.6-0.8 for important updates, 0.3-0.5 for routine
- notable_message_ids: include IDs of the 2-5 most important messages"##)
}

/// Generate a prompt for cross-channel grouping.
/// 
/// Used as the second pass in hierarchical summarization to combine
/// channel summaries into topic groups.
pub fn cross_channel_grouping_prompt(date: &str, channel_summaries_json: &str, ungrouped_messages_json: Option<&str>) -> String {
    let ungrouped_section = ungrouped_messages_json
        .map(|json| format!(r##"
MESSAGES FROM LOW-VOLUME CHANNELS (process directly):
{json}
"##))
        .unwrap_or_default();
    
    format!(r##"You are creating a daily digest for {date} by combining summaries from multiple Slack channels.

CHANNEL SUMMARIES:
{channel_summaries_json}
{ungrouped_section}
Your task is to:
1. Identify topics that span multiple channels (cross-channel themes)
2. Group related channel discussions together
3. Create an executive summary of the entire day

Return JSON with this structure:
{{
  "groups": [
    {{
      "topic": "Cross-channel topic name (e.g., 'Q1 Product Launch')",
      "channels": ["#channel1", "#channel2"],
      "summary": "2-4 sentence summary combining the related discussions",
      "highlights": ["key point 1", "key point 2"],
      "category": "one of: sales, marketing, product, engineering, research, other",
      "importance_score": 0.0-1.0,
      "message_ids": ["notable_id1", "notable_id2"],
      "key_message_ids": ["notable_id1"],
      "people": ["person1", "person2"]
    }}
  ],
  "daily_summary": "3-4 sentence executive summary of the day",
  "key_themes": ["theme1", "theme2", "theme3"],
  "action_items": ["action1", "action2"]
}}

Guidelines:
- Group discussions by TOPIC, not by channel
- A channel's content can be split across multiple topic groups
- importance_score: based on business impact, not just activity level
- Include action items that emerge from discussions
- key_message_ids: Select 1-3 of the MOST IMPORTANT messages for jumping back into the conversation"##)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(prompt.contains(&"x".repeat(100)));
    }

    #[test]
    fn test_confluence_page_prompt_handles_multibyte_utf8() {
        // Create content with multi-byte UTF-8 characters
        let prefix = "x".repeat(7998);
        let emoji_content = format!("{}🎉🎉🎉", prefix);
        
        // This should not panic and should truncate at a valid UTF-8 boundary
        let prompt = confluence_page_prompt("Title", "Space", &emoji_content);
        
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
    }

    #[test]
    fn test_batch_analysis_prompt_contains_date_and_messages() {
        let messages = r##"[{"id": "1", "channel": "#general", "text": "Hello"}]"##;
        let prompt = batch_analysis_prompt("2024-01-15", messages);
        assert!(prompt.contains("2024-01-15"));
        assert!(prompt.contains("#general"));
        assert!(prompt.contains("groups"));
        assert!(prompt.contains("ungrouped"));
        assert!(prompt.contains("daily_summary"));
    }

    #[test]
    fn test_batch_analysis_prompt_with_existing_topics() {
        let messages = r##"[{"id": "1", "channel": "#general", "text": "More about the launch"}]"##;
        let existing_topics = r##"[{"topic_id": "topic_123", "topic": "Q1 Launch"}]"##;
        
        let prompt = batch_analysis_prompt_with_existing("2024-01-15", messages, Some(existing_topics));
        
        assert!(prompt.contains("EXISTING TOPICS FROM EARLIER TODAY"));
        assert!(prompt.contains("topic_123"));
        assert!(prompt.contains("MERGING RULES"));
    }

    #[test]
    fn test_batch_analysis_prompt_without_existing_topics() {
        let messages = r##"[{"id": "1", "channel": "#general", "text": "Hello"}]"##;
        
        let prompt = batch_analysis_prompt_with_existing("2024-01-15", messages, None);
        
        assert!(!prompt.contains("EXISTING TOPICS FROM EARLIER TODAY"));
        assert!(!prompt.contains("MERGING RULES"));
    }

    #[test]
    fn test_batch_analysis_prompt_backwards_compatible() {
        let messages = r##"[{"id": "1", "channel": "#test", "text": "Test"}]"##;
        
        let prompt1 = batch_analysis_prompt("2024-01-15", messages);
        let prompt2 = batch_analysis_prompt_with_existing("2024-01-15", messages, None);
        
        assert!(!prompt1.contains("EXISTING TOPICS"));
        assert!(!prompt2.contains("EXISTING TOPICS"));
    }

    #[test]
    fn test_batch_analysis_prompt_includes_topic_id_instruction() {
        let messages = r##"[{"id": "1", "channel": "#test", "text": "Test"}]"##;
        
        let prompt_without = batch_analysis_prompt_with_existing("2024-01-15", messages, None);
        assert!(prompt_without.contains(r#""topic_id": null"#));
        
        let existing = r##"[{"topic_id": "t1", "topic": "Test"}]"##;
        let prompt_with = batch_analysis_prompt_with_existing("2024-01-15", messages, Some(existing));
        assert!(prompt_with.contains(r#""topic_id": "topic_abc123""#));
    }

    #[test]
    fn test_channel_summary_prompt_contains_channel() {
        let prompt = channel_summary_prompt("general", None, "[]");
        assert!(prompt.contains("#general"));
        assert!(prompt.contains("Messages:"));
        assert!(prompt.contains("key_topics"));
    }

    #[test]
    fn test_channel_summary_prompt_with_purpose() {
        let prompt = channel_summary_prompt("sales", Some("Sales team discussions"), "[]");
        assert!(prompt.contains("#sales"));
        assert!(prompt.contains("Channel purpose: Sales team discussions"));
    }

    #[test]
    fn test_channel_summary_prompt_without_purpose() {
        let prompt = channel_summary_prompt("random", None, "[]");
        assert!(prompt.contains("#random"));
        assert!(!prompt.contains("Channel purpose:"));
    }

    #[test]
    fn test_cross_channel_grouping_prompt_contains_date() {
        let prompt = cross_channel_grouping_prompt("2024-01-20", "[]", None);
        assert!(prompt.contains("2024-01-20"));
        assert!(prompt.contains("CHANNEL SUMMARIES"));
    }

    #[test]
    fn test_cross_channel_grouping_prompt_with_ungrouped() {
        let prompt = cross_channel_grouping_prompt("2024-01-20", "[]", Some("[{\"id\":\"1\"}]"));
        assert!(prompt.contains("MESSAGES FROM LOW-VOLUME CHANNELS"));
        assert!(prompt.contains("[{\"id\":\"1\"}]"));
    }

    #[test]
    fn test_cross_channel_grouping_prompt_without_ungrouped() {
        let prompt = cross_channel_grouping_prompt("2024-01-20", "[]", None);
        assert!(!prompt.contains("MESSAGES FROM LOW-VOLUME CHANNELS"));
    }
}
