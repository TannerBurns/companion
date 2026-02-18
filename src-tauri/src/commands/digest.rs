use super::credentials::get_gemini_client;
use super::types::{
    CategorySummary, DigestItem, DigestResponse, GroupRow, ParsedEntities, WeeklyBreakdownResponse,
};
use crate::ai::{prompts, GeminiClient, ServiceAccountCredentials};
use crate::db::Database;
use crate::AppState;
use chrono::Datelike;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

type ParsedGroup = (
    String,
    String,
    Option<Vec<String>>,
    String,
    f64,
    i64,
    ParsedEntities,
);

fn append_breakdown_section(output: &mut String, title: &str, items: &[String]) {
    output.push_str(title);
    output.push('\n');
    if items.is_empty() {
        output.push_str("- None noted\n");
    } else {
        for item in items {
            output.push_str("- ");
            output.push_str(item);
            output.push('\n');
        }
    }
    output.push('\n');
}

fn normalize_breakdown_items(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .map(|item| item.trim().trim_start_matches("- ").to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn build_gemini_client(api_key_or_credentials: String) -> Result<GeminiClient, String> {
    if let Some(json_str) = api_key_or_credentials.strip_prefix("SERVICE_ACCOUNT:") {
        let credentials: ServiceAccountCredentials = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse Gemini service account credentials: {}", e))?;
        Ok(GeminiClient::new_with_service_account(credentials))
    } else {
        Ok(GeminiClient::new(api_key_or_credentials))
    }
}

fn week_window(
    week_start: Option<String>,
    timezone_offset: Option<i32>,
) -> Result<(chrono::NaiveDate, String, i64, i64, chrono::FixedOffset), String> {
    let offset_minutes = timezone_offset.unwrap_or(0);
    let local_offset = chrono::FixedOffset::west_opt(offset_minutes * 60)
        .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());

    let now_utc = chrono::Utc::now();
    let today_local = now_utc.with_timezone(&local_offset).date_naive();

    let week_start_date = if let Some(ref ws) = week_start {
        chrono::NaiveDate::parse_from_str(ws, "%Y-%m-%d").map_err(|e| e.to_string())?
    } else {
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
    let start_ts = local_midnight
        .with_timezone(&chrono::Utc)
        .timestamp_millis();
    let end_ts = start_ts + 7 * 86400 * 1000;

    Ok((
        week_start_date,
        week_start_str,
        start_ts,
        end_ts,
        local_offset,
    ))
}

async fn load_user_guidance(db: &Database) -> Option<String> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM preferences WHERE key = 'user_preferences'")
            .fetch_optional(db.pool())
            .await
            .ok()?;

    result.and_then(|(json,)| {
        let prefs: serde_json::Value = serde_json::from_str(&json).ok()?;
        prefs
            .get("userGuidance")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(String::from)
    })
}

async fn batch_lookup_source_urls(
    db: &Database,
    groups: &[(String, Vec<String>)],
    urls_per_group: usize,
) -> Result<HashMap<String, Vec<String>>, String> {
    if groups.is_empty() {
        return Ok(HashMap::new());
    }

    let all_ids: Vec<&String> = groups
        .iter()
        .flat_map(|(_, ids)| ids.iter().take(urls_per_group))
        .collect();

    if all_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders: String = all_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT id, source_url FROM content_items WHERE id IN ({}) AND source_url IS NOT NULL",
        placeholders
    );

    let mut query_builder = sqlx::query_as::<_, (String, String)>(&query);
    for id in &all_ids {
        query_builder = query_builder.bind(*id);
    }

    let results: Vec<(String, String)> = query_builder
        .fetch_all(db.pool())
        .await
        .map_err(|e| e.to_string())?;

    let url_map: HashMap<String, String> = results.into_iter().collect();

    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    for (group_id, message_ids) in groups {
        let urls: Vec<String> = message_ids
            .iter()
            .take(urls_per_group)
            .filter_map(|id| url_map.get(id).cloned())
            .collect();
        if !urls.is_empty() {
            result.insert(group_id.clone(), urls);
        }
    }

    Ok(result)
}

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
        let offset = chrono::FixedOffset::west_opt(offset_minutes * 60)
            .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
        now_utc
            .with_timezone(&offset)
            .format("%Y-%m-%d")
            .to_string()
    });

    let db = {
        let state = state.lock().await;
        state.db.clone()
    };

    let parsed_date =
        chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| e.to_string())?;

    // Convert local midnight to UTC timestamp
    let local_offset = chrono::FixedOffset::west_opt(offset_minutes * 60)
        .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());

    let local_midnight = parsed_date
        .and_hms_opt(0, 0, 0)
        .ok_or("Invalid date")?
        .and_local_timezone(local_offset)
        .single()
        .ok_or("Ambiguous or invalid local time")?;

    let start_ts = local_midnight
        .with_timezone(&chrono::Utc)
        .timestamp_millis();
    let end_ts = start_ts + 86400 * 1000; // 24 hours

    // Fetch group summaries (cross-channel grouped content)
    let groups: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, summary, highlights, category, importance_score, entities, generated_at
         FROM ai_summaries 
         WHERE summary_type = 'group' AND generated_at >= ? AND generated_at < ?
         ORDER BY importance_score DESC",
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(db.pool())
    .await
    .map_err(|e| e.to_string())?;

    let mut parsed_groups: Vec<ParsedGroup> = Vec::new();
    let mut groups_for_url_lookup: Vec<(String, Vec<String>)> = Vec::new();

    for (id, summary, highlights_json, category, importance_score, entities, generated_at) in
        &groups
    {
        let parsed = ParsedEntities::from_json(entities);
        let highlights: Option<Vec<String>> = highlights_json
            .as_ref()
            .and_then(|h| serde_json::from_str(h).ok());
        let cat = category.clone().unwrap_or_else(|| "other".to_string());

        let ids_for_lookup = if !parsed.key_message_ids.is_empty() {
            parsed.key_message_ids.clone()
        } else {
            parsed.message_ids.clone()
        };

        if !ids_for_lookup.is_empty() {
            groups_for_url_lookup.push((id.clone(), ids_for_lookup));
        }

        parsed_groups.push((
            id.clone(),
            summary.clone(),
            highlights,
            cat,
            importance_score.unwrap_or(0.5),
            *generated_at,
            parsed,
        ));
    }

    let source_urls_map = batch_lookup_source_urls(&db, &groups_for_url_lookup, 3).await?;

    let mut items: Vec<DigestItem> = Vec::new();
    let mut category_counts: std::collections::HashMap<String, (i32, Vec<DigestItem>)> =
        std::collections::HashMap::new();

    for (id, summary, highlights, cat, importance_score, generated_at, parsed) in parsed_groups {
        let urls = source_urls_map.get(&id).cloned();
        // Use first URL as primary source_url for backward compatibility
        let primary_url = urls.as_ref().and_then(|u| u.first().cloned());

        let item = DigestItem {
            id: id.clone(),
            title: parsed.title,
            summary,
            highlights,
            category: cat.clone(),
            source: "slack".to_string(),
            source_url: primary_url,
            source_urls: urls,
            importance_score,
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
         ORDER BY generated_at DESC LIMIT 1",
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;

    if let Some((summary, highlights_json)) = daily_summary {
        let highlights: Option<Vec<String>> =
            highlights_json.and_then(|h| serde_json::from_str(&h).ok());

        items.insert(
            0,
            DigestItem {
                id: "daily-summary".to_string(),
                title: "Today's Overview".to_string(),
                summary,
                highlights,
                category: "overview".to_string(),
                source: "ai".to_string(),
                source_url: None,
                source_urls: None,
                importance_score: 1.0,
                created_at: start_ts,
                channels: None,
                people: None,
                message_count: None,
            },
        );
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
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };

    let (_week_start_date, week_start_str, start_ts, end_ts, local_offset) =
        week_window(week_start, timezone_offset)?;

    let weekly_groups: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, summary, highlights, category, importance_score, entities, generated_at
         FROM ai_summaries 
         WHERE summary_type = 'group' AND generated_at >= ? AND generated_at < ?
         ORDER BY importance_score DESC",
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(db.pool())
    .await
    .map_err(|e| e.to_string())?;

    let mut parsed_weekly_groups: Vec<ParsedGroup> = Vec::new();
    let mut weekly_groups_for_url_lookup: Vec<(String, Vec<String>)> = Vec::new();

    for (id, summary, highlights_json, category, importance_score, entities, generated_at) in
        &weekly_groups
    {
        let parsed = ParsedEntities::from_json(entities);
        let highlights: Option<Vec<String>> = highlights_json
            .as_ref()
            .and_then(|h| serde_json::from_str(h).ok());
        let cat = category.clone().unwrap_or_else(|| "other".to_string());

        let ids_for_lookup = if !parsed.key_message_ids.is_empty() {
            parsed.key_message_ids.clone()
        } else {
            parsed.message_ids.clone()
        };

        if !ids_for_lookup.is_empty() {
            weekly_groups_for_url_lookup.push((id.clone(), ids_for_lookup));
        }

        parsed_weekly_groups.push((
            id.clone(),
            summary.clone(),
            highlights,
            cat,
            importance_score.unwrap_or(0.5),
            *generated_at,
            parsed,
        ));
    }

    let weekly_source_urls_map =
        batch_lookup_source_urls(&db, &weekly_groups_for_url_lookup, 3).await?;

    let mut items: Vec<DigestItem> = Vec::new();
    let mut category_counts: std::collections::HashMap<String, (i32, Vec<DigestItem>)> =
        std::collections::HashMap::new();

    for (id, summary, highlights, cat, importance_score, generated_at, parsed) in
        parsed_weekly_groups
    {
        let urls = weekly_source_urls_map.get(&id).cloned();
        // Use first URL as primary source_url for backward compatibility
        let primary_url = urls.as_ref().and_then(|u| u.first().cloned());

        let item = DigestItem {
            id: id.clone(),
            title: parsed.title,
            summary,
            highlights,
            category: cat.clone(),
            source: "slack".to_string(),
            source_url: primary_url,
            source_urls: urls,
            importance_score,
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
         ORDER BY generated_at DESC",
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
            .map(|dt| {
                dt.with_timezone(&local_offset)
                    .format("%A, %b %d")
                    .to_string()
            })
            .unwrap_or_else(|| "Daily".to_string());

        items.push(DigestItem {
            id: id.clone(),
            title: format!("{} Overview", date_display),
            summary: summary.clone(),
            highlights,
            category: "overview".to_string(),
            source: "ai".to_string(),
            source_url: None,
            source_urls: None,
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

/// Generate a weekly breakdown from all captured daily summaries in the selected week.
#[tauri::command]
pub async fn generate_weekly_breakdown(
    state: State<'_, Arc<Mutex<AppState>>>,
    week_start: Option<String>,
    timezone_offset: Option<i32>,
) -> Result<WeeklyBreakdownResponse, String> {
    #[derive(serde::Serialize)]
    struct DailySummaryForPrompt {
        date: String,
        summary: String,
        highlights: Vec<String>,
    }

    let (db, crypto) = {
        let state = state.lock().await;
        (state.db.clone(), Arc::new(state.crypto.clone()))
    };

    let (week_start_date, week_start_str, _start_ts, _end_ts, _local_offset) =
        week_window(week_start, timezone_offset)?;
    let week_end_date = week_start_date + chrono::Duration::days(6);
    let week_end_str = week_end_date.format("%Y-%m-%d").to_string();

    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT id, summary, highlights
         FROM ai_summaries
         WHERE summary_type = 'daily' AND id IN (",
    );
    let mut expected_daily_ids: Vec<String> = Vec::with_capacity(7);
    for day_offset in 0..7 {
        expected_daily_ids.push(format!(
            "daily_{}",
            (week_start_date + chrono::Duration::days(day_offset)).format("%Y-%m-%d")
        ));
    }
    {
        let mut separated = query_builder.separated(", ");
        for id in &expected_daily_ids {
            separated.push_bind(id);
        }
    }
    query_builder.push(") ORDER BY id ASC");
    let daily_summaries: Vec<(String, String, Option<String>)> = query_builder
        .build_query_as()
        .fetch_all(db.pool())
        .await
        .map_err(|e| e.to_string())?;

    if daily_summaries.is_empty() {
        return Err("Not enough weekly summaries to generate a breakdown yet. Sync and generate daily summaries for this week first.".to_string());
    }

    let date_range: Vec<chrono::NaiveDate> = daily_summaries
        .iter()
        .filter_map(|(id, _, _)| {
            id.strip_prefix("daily_")
                .and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        })
        .collect();

    let range_start = date_range.iter().min().copied().unwrap_or(week_start_date);
    let range_end = date_range.iter().max().copied().unwrap_or(week_end_date);

    let title = format!(
        "Update - {} - {}",
        range_start.format("%B %-d, %Y"),
        range_end.format("%B %-d, %Y")
    );

    let prompt_input: Vec<DailySummaryForPrompt> = daily_summaries
        .iter()
        .filter_map(|(id, summary, highlights_json)| {
            let date = id
                .strip_prefix("daily_")
                .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())?;
            let highlights = highlights_json
                .as_ref()
                .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
                .unwrap_or_default();
            Some(DailySummaryForPrompt {
                date: date.format("%Y-%m-%d").to_string(),
                summary: summary.clone(),
                highlights,
            })
        })
        .collect();

    let prompt_input_json =
        serde_json::to_string_pretty(&prompt_input).map_err(|e| e.to_string())?;
    let user_guidance = load_user_guidance(&db).await;
    let prompt =
        prompts::weekly_breakdown_prompt(&title, &prompt_input_json, user_guidance.as_deref());

    let api_key_or_credentials = get_gemini_client(db.clone(), crypto)
        .await
        .ok_or_else(|| "Gemini credentials are not configured. Add Gemini credentials in Settings to generate a weekly breakdown.".to_string())?;
    let gemini = build_gemini_client(api_key_or_credentials)?;

    let ai_result: prompts::WeeklyBreakdown = gemini
        .generate_json(&prompt)
        .await
        .map_err(|e| format!("Failed to generate weekly breakdown: {}", e))?;

    let major = normalize_breakdown_items(ai_result.major);
    let focus = normalize_breakdown_items(ai_result.focus);
    let obstacles = normalize_breakdown_items(ai_result.obstacles);
    let informational = normalize_breakdown_items(ai_result.informational);

    let mut breakdown_text = String::new();
    breakdown_text.push_str(&title);
    breakdown_text.push('\n');
    append_breakdown_section(&mut breakdown_text, "🚀 Major", &major);
    append_breakdown_section(&mut breakdown_text, "🎯 Focus", &focus);
    append_breakdown_section(&mut breakdown_text, "🪨 Obstacles", &obstacles);
    append_breakdown_section(&mut breakdown_text, "📣 Informational", &informational);
    breakdown_text.pop();

    Ok(WeeklyBreakdownResponse {
        week_start: week_start_str,
        week_end: week_end_str,
        title,
        major,
        focus,
        obstacles,
        informational,
        breakdown_text,
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
        assert_eq!(
            monday,
            chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
        );
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

        let start_ts = local_midnight
            .with_timezone(&chrono::Utc)
            .timestamp_millis();
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
