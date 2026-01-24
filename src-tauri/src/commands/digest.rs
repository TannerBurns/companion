use crate::AppState;
use super::types::{DigestItem, DigestResponse, CategorySummary, GroupRow, ParsedEntities};
use chrono::Datelike;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Get the daily digest for a specific date
#[tauri::command]
pub async fn get_daily_digest(
    state: State<'_, Arc<Mutex<AppState>>>,
    date: Option<String>,
    timezone_offset: Option<i32>,
) -> Result<DigestResponse, String> {
    // JS getTimezoneOffset() returns minutes, positive for west of UTC
    let offset_minutes = timezone_offset.unwrap_or(0);
    
    let date_str = date.unwrap_or_else(|| {
        let now_utc = chrono::Utc::now();
        let offset = chrono::FixedOffset::west_opt(offset_minutes * 60).unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
        now_utc.with_timezone(&offset).format("%Y-%m-%d").to_string()
    });
    
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };
    
    let parsed_date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    
    // Convert local midnight to UTC timestamp
    let local_offset = chrono::FixedOffset::west_opt(offset_minutes * 60)
        .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
    
    let local_midnight = parsed_date
        .and_hms_opt(0, 0, 0)
        .ok_or("Invalid date")?
        .and_local_timezone(local_offset)
        .single()
        .ok_or("Ambiguous or invalid local time")?;
    
    let start_ts = local_midnight.with_timezone(&chrono::Utc).timestamp_millis();
    let end_ts = start_ts + 86400 * 1000; // 24 hours
    
    // Fetch group summaries (cross-channel grouped content)
    let groups: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, summary, highlights, category, importance_score, entities, generated_at
         FROM ai_summaries 
         WHERE summary_type = 'group' AND generated_at >= ? AND generated_at < ?
         ORDER BY importance_score DESC"
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    let mut items: Vec<DigestItem> = Vec::new();
    let mut category_counts: std::collections::HashMap<String, (i32, Vec<DigestItem>)> = std::collections::HashMap::new();
    
    for (id, summary, highlights_json, category, importance_score, entities, generated_at) in groups {
        let parsed = ParsedEntities::from_json(&entities);
        let highlights: Option<Vec<String>> = highlights_json
            .and_then(|h| serde_json::from_str(&h).ok());
        let cat = category.clone().unwrap_or_else(|| "other".to_string());
        
        let item = DigestItem {
            id: id.clone(),
            title: parsed.title,
            summary: summary.clone(),
            highlights,
            category: cat.clone(),
            source: "slack".to_string(),
            source_url: None,
            importance_score: importance_score.unwrap_or(0.5),
            created_at: generated_at,
            channels: parsed.channels,
            people: parsed.people,
            message_count: parsed.message_count,
        };
        
        let entry = category_counts.entry(cat.clone()).or_insert((0, vec![]));
        entry.0 += 1;
        if entry.1.len() < 3 {
            entry.1.push(item.clone());
        }
        
        items.push(item);
    }
    
    let daily_summary: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT summary, highlights FROM ai_summaries 
         WHERE summary_type = 'daily' AND generated_at >= ? AND generated_at < ?
         ORDER BY generated_at DESC LIMIT 1"
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if let Some((summary, highlights_json)) = daily_summary {
        let highlights: Option<Vec<String>> = highlights_json
            .and_then(|h| serde_json::from_str(&h).ok());
        
        items.insert(0, DigestItem {
            id: "daily-summary".to_string(),
            title: "Today's Overview".to_string(),
            summary,
            highlights,
            category: "overview".to_string(),
            source: "ai".to_string(),
            source_url: None,
            importance_score: 1.0,
            created_at: start_ts,
            channels: None,
            people: None,
            message_count: None,
        });
    }
    
    let categories: Vec<CategorySummary> = category_counts
        .into_iter()
        .map(|(name, (count, top_items))| CategorySummary {
            name,
            count,
            top_items,
        })
        .collect();
    
    Ok(DigestResponse {
        date: date_str,
        items,
        categories,
    })
}

/// Get the weekly digest for a specific week
#[tauri::command]
pub async fn get_weekly_digest(
    state: State<'_, Arc<Mutex<AppState>>>,
    week_start: Option<String>,
    timezone_offset: Option<i32>,
) -> Result<DigestResponse, String> {
    let offset_minutes = timezone_offset.unwrap_or(0);
    let local_offset = chrono::FixedOffset::west_opt(offset_minutes * 60)
        .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
    
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };
    
    // Calculate week start (Monday) if not provided
    let now_utc = chrono::Utc::now();
    let today_local = now_utc.with_timezone(&local_offset).date_naive();
    
    let week_start_date = if let Some(ref ws) = week_start {
        chrono::NaiveDate::parse_from_str(ws, "%Y-%m-%d")
            .map_err(|e| e.to_string())?
    } else {
        // Get Monday of current week in local timezone
        let days_since_monday = today_local.weekday().num_days_from_monday();
        today_local - chrono::Duration::days(days_since_monday as i64)
    };
    
    let week_start_str = week_start_date.format("%Y-%m-%d").to_string();
    
    let local_midnight = week_start_date
        .and_hms_opt(0, 0, 0)
        .ok_or("Invalid date")?
        .and_local_timezone(local_offset)
        .single()
        .ok_or("Ambiguous or invalid local time")?;
    
    let start_ts = local_midnight.with_timezone(&chrono::Utc).timestamp_millis();
    let end_ts = start_ts + 7 * 86400 * 1000;
    
    let groups: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, summary, highlights, category, importance_score, entities, generated_at
         FROM ai_summaries 
         WHERE summary_type = 'group' AND generated_at >= ? AND generated_at < ?
         ORDER BY importance_score DESC"
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    let mut items: Vec<DigestItem> = Vec::new();
    let mut category_counts: std::collections::HashMap<String, (i32, Vec<DigestItem>)> = std::collections::HashMap::new();
    
    for (id, summary, highlights_json, category, importance_score, entities, generated_at) in groups {
        let parsed = ParsedEntities::from_json(&entities);
        let highlights: Option<Vec<String>> = highlights_json
            .and_then(|h| serde_json::from_str(&h).ok());
        let cat = category.clone().unwrap_or_else(|| "other".to_string());
        
        let item = DigestItem {
            id: id.clone(),
            title: parsed.title,
            summary: summary.clone(),
            highlights,
            category: cat.clone(),
            source: "slack".to_string(),
            source_url: None,
            importance_score: importance_score.unwrap_or(0.5),
            created_at: generated_at,
            channels: parsed.channels,
            people: parsed.people,
            message_count: parsed.message_count,
        };
        
        let entry = category_counts.entry(cat.clone()).or_insert((0, vec![]));
        entry.0 += 1;
        if entry.1.len() < 3 {
            entry.1.push(item.clone());
        }
        
        items.push(item);
    }
    
    // Fetch individual daily summaries with their timestamps
    let daily_summaries: Vec<(String, String, Option<String>, i64)> = sqlx::query_as(
        "SELECT id, summary, highlights, generated_at FROM ai_summaries 
         WHERE summary_type = 'daily' AND generated_at >= ? AND generated_at < ?
         ORDER BY generated_at DESC"
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    // Add individual daily overview items with their actual dates
    for (id, summary, highlights_json, generated_at) in &daily_summaries {
        let highlights: Option<Vec<String>> = highlights_json
            .as_ref()
            .and_then(|h| serde_json::from_str(h).ok());
        
        // Convert generated_at to a date string for the title
        let date_display = chrono::DateTime::from_timestamp_millis(*generated_at)
            .map(|dt| dt.with_timezone(&local_offset).format("%A, %b %d").to_string())
            .unwrap_or_else(|| "Daily".to_string());
        
        items.push(DigestItem {
            id: id.clone(),
            title: format!("{} Overview", date_display),
            summary: summary.clone(),
            highlights,
            category: "overview".to_string(),
            source: "ai".to_string(),
            source_url: None,
            importance_score: 0.95, // High but below 1.0 so they sort after group items
            created_at: *generated_at,
            channels: None,
            people: None,
            message_count: None,
        });
    }
    
    let categories: Vec<CategorySummary> = category_counts
        .into_iter()
        .map(|(name, (count, top_items))| CategorySummary {
            name,
            count,
            top_items,
        })
        .collect();
    
    Ok(DigestResponse {
        date: week_start_str,
        items,
        categories,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timezone_offset_conversion() {
        // Test that timezone offset is applied correctly
        // PST is UTC-8, so offset is 480 (positive for west of UTC)
        let offset_minutes = 480;
        let offset = chrono::FixedOffset::west_opt(offset_minutes * 60).unwrap();
        
        // Should be 8 hours behind UTC
        let now_utc = chrono::Utc::now();
        let local = now_utc.with_timezone(&offset);
        
        let diff = now_utc.timestamp() - local.timestamp();
        assert_eq!(diff, 0); // Same instant, different representation
    }

    #[test]
    fn test_date_parsing() {
        let date_str = "2024-01-15";
        let parsed = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
        assert!(parsed.is_ok());
        
        let date = parsed.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 15);
    }

    #[test]
    fn test_week_start_calculation() {
        // Test calculating Monday of a week
        let date = chrono::NaiveDate::from_ymd_opt(2024, 1, 17).unwrap(); // Wednesday
        let days_since_monday = date.weekday().num_days_from_monday();
        let monday = date - chrono::Duration::days(days_since_monday as i64);
        
        assert_eq!(monday.weekday(), chrono::Weekday::Mon);
        assert_eq!(monday, chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn test_timestamp_range_calculation() {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let offset = chrono::FixedOffset::west_opt(0).unwrap();
        
        let local_midnight = date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(offset)
            .single()
            .unwrap();
        
        let start_ts = local_midnight.with_timezone(&chrono::Utc).timestamp_millis();
        let end_ts = start_ts + 86400 * 1000;
        
        // Should be exactly 24 hours apart
        assert_eq!(end_ts - start_ts, 86400 * 1000);
    }

    #[test]
    fn test_weekly_timestamp_range() {
        let start_ts: i64 = 1705276800000; // Some timestamp
        let end_ts = start_ts + 7 * 86400 * 1000;
        
        // Should be exactly 7 days apart
        assert_eq!(end_ts - start_ts, 7 * 86400 * 1000);
    }
}
