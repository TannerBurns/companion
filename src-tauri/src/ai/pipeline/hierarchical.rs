use std::collections::HashMap;
use crate::ai::gemini::GeminiClient;
use crate::ai::prompts::{self, ChannelSummary, GroupedAnalysisResult};
use super::types::{MessageForPrompt, HIERARCHICAL_CHANNEL_THRESHOLD};

/// Process messages using hierarchical summarization.
///
/// High-volume channels are summarized individually first, then
/// those summaries are combined with smaller channels for cross-channel grouping.
///
/// # Arguments
/// * `gemini` - The Gemini client for AI requests
/// * `date_str` - The date string in YYYY-MM-DD format
/// * `messages_by_channel` - Messages grouped by channel name
///
/// # Returns
/// A GroupedAnalysisResult containing topic groups and ungrouped items
pub async fn process_hierarchical(
    gemini: &GeminiClient,
    date_str: &str,
    messages_by_channel: HashMap<String, Vec<MessageForPrompt>>,
) -> Result<GroupedAnalysisResult, String> {
    let mut channel_summaries: Vec<ChannelSummary> = Vec::new();
    let mut small_channel_messages: Vec<MessageForPrompt> = Vec::new();
    
    for (channel, messages) in messages_by_channel {
        if messages.len() >= HIERARCHICAL_CHANNEL_THRESHOLD {
            tracing::info!(
                "Summarizing high-volume channel {} ({} messages)",
                channel,
                messages.len()
            );
            
            let messages_json = serde_json::to_string_pretty(&messages)
                .map_err(|e| e.to_string())?;
            
            let prompt = prompts::channel_summary_prompt(&channel, None, &messages_json);
            
            match gemini.generate_json::<ChannelSummary>(&prompt).await {
                Ok(mut summary) => {
                    summary.message_count = messages.len() as i32;
                    channel_summaries.push(summary);
                }
                Err(e) => {
                    tracing::error!("Failed to summarize channel {}: {}", channel, e);
                    // Fall back to including messages directly
                    small_channel_messages.extend(messages);
                }
            }
        } else {
            small_channel_messages.extend(messages);
        }
    }
    
    tracing::info!(
        "Pass 1 complete: {} channel summaries, {} messages for direct processing",
        channel_summaries.len(),
        small_channel_messages.len()
    );
    
    // Build the cross-channel grouping prompt
    let channel_summaries_json = serde_json::to_string_pretty(&channel_summaries)
        .map_err(|e| e.to_string())?;
    
    let ungrouped_json = if small_channel_messages.is_empty() {
        None
    } else {
        Some(serde_json::to_string_pretty(&small_channel_messages).map_err(|e| e.to_string())?)
    };
    
    let prompt = prompts::cross_channel_grouping_prompt(
        date_str,
        &channel_summaries_json,
        ungrouped_json.as_deref(),
    );
    
    let result: GroupedAnalysisResult = gemini
        .generate_json(&prompt)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_threshold_value() {
        assert_eq!(HIERARCHICAL_CHANNEL_THRESHOLD, 50);
    }

    #[test]
    fn test_channel_partitioning_logic() {
        // Test that channels are partitioned correctly based on threshold
        let mut messages_by_channel: HashMap<String, Vec<MessageForPrompt>> = HashMap::new();
        
        // Add a large channel (above threshold)
        let large_channel_msgs: Vec<MessageForPrompt> = (0..60)
            .map(|i| MessageForPrompt {
                id: format!("msg{}", i),
                channel: "#large".to_string(),
                author: "user".to_string(),
                timestamp: "10:00".to_string(),
                text: format!("Message {}", i),
                url: None,
                thread_id: None,
            })
            .collect();
        messages_by_channel.insert("#large".to_string(), large_channel_msgs);
        
        // Add a small channel (below threshold)
        let small_channel_msgs: Vec<MessageForPrompt> = (0..10)
            .map(|i| MessageForPrompt {
                id: format!("small_msg{}", i),
                channel: "#small".to_string(),
                author: "user".to_string(),
                timestamp: "11:00".to_string(),
                text: format!("Small message {}", i),
                url: None,
                thread_id: None,
            })
            .collect();
        messages_by_channel.insert("#small".to_string(), small_channel_msgs);
        
        let mut large_channels = 0;
        let mut small_channels = 0;
        
        for messages in messages_by_channel.values() {
            if messages.len() >= HIERARCHICAL_CHANNEL_THRESHOLD {
                large_channels += 1;
            } else {
                small_channels += 1;
            }
        }
        
        assert_eq!(large_channels, 1);
        assert_eq!(small_channels, 1);
    }

    #[test]
    fn test_empty_small_channel_messages() {
        // Test that empty small_channel_messages results in None for ungrouped_json
        let small_channel_messages: Vec<MessageForPrompt> = vec![];
        
        let ungrouped_json = if small_channel_messages.is_empty() {
            None
        } else {
            Some(serde_json::to_string_pretty(&small_channel_messages).unwrap())
        };
        
        assert!(ungrouped_json.is_none());
    }

    #[test]
    fn test_non_empty_small_channel_messages() {
        let small_channel_messages = vec![
            MessageForPrompt {
                id: "msg1".to_string(),
                channel: "#test".to_string(),
                author: "user".to_string(),
                timestamp: "10:00".to_string(),
                text: "Test message".to_string(),
                url: None,
                thread_id: None,
            }
        ];
        
        let ungrouped_json = if small_channel_messages.is_empty() {
            None
        } else {
            Some(serde_json::to_string_pretty(&small_channel_messages).unwrap())
        };
        
        assert!(ungrouped_json.is_some());
        assert!(ungrouped_json.unwrap().contains("msg1"));
    }
}
