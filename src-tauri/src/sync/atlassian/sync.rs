//! Atlassian synchronization service for Jira and Confluence

use super::client::AtlassianClient;
use super::types::{AtlassianError, ConfluencePage, JiraIssue};
use crate::crypto::CryptoService;
use crate::db::Database;
use std::sync::Arc;

pub struct AtlassianSyncService {
    client: AtlassianClient,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
}

impl AtlassianSyncService {
    pub fn new(client: AtlassianClient, db: Arc<Database>, crypto: Arc<CryptoService>) -> Self {
        Self { client, db, crypto }
    }

    /// Sync Jira issues updated in the last N days
    pub async fn sync_jira(&self, days: i32) -> Result<i32, AtlassianError> {
        let jql = format!("updated >= -{}d ORDER BY updated DESC", days);
        let mut total = 0;
        let mut start_at = 0;

        loop {
            let issues = self.client.search_issues(&jql, start_at, 50).await?;

            if issues.is_empty() {
                break;
            }

            for issue in &issues {
                self.store_jira_issue(issue).await?;
                total += 1;
            }

            start_at += 50;
        }

        Ok(total)
    }

    /// Sync Confluence pages updated in the last N days
    pub async fn sync_confluence(&self, days: i32) -> Result<i32, AtlassianError> {
        let cql = format!(
            "lastModified >= now('-{}d') ORDER BY lastModified DESC",
            days
        );
        let mut total = 0;
        let mut start = 0;

        loop {
            let pages = self.client.search_pages(&cql, start, 25).await?;

            if pages.is_empty() {
                break;
            }

            for page in &pages {
                self.store_confluence_page(page).await?;
                total += 1;
            }

            start += 25;
        }

        Ok(total)
    }

    async fn store_jira_issue(&self, issue: &JiraIssue) -> Result<(), AtlassianError> {
        let now = chrono::Utc::now().timestamp();
        let created_at = chrono::DateTime::parse_from_rfc3339(&issue.created)
            .map(|dt| dt.timestamp())
            .unwrap_or(now);
        let updated_at = chrono::DateTime::parse_from_rfc3339(&issue.updated)
            .map(|dt| dt.timestamp())
            .unwrap_or(now);

        let description = issue.description.as_deref().unwrap_or("");
        let encrypted_body = self
            .crypto
            .encrypt_string(description)
            .map_err(|e| AtlassianError::Crypto(e.to_string()))?;

        sqlx::query(
            "INSERT INTO content_items (id, source, source_id, source_url, content_type, title, body, author_id, channel_or_project, created_at, updated_at, synced_at)
             VALUES (?, 'jira', ?, ?, 'ticket', ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source, source_id) DO UPDATE SET title = ?, body = ?, updated_at = ?, synced_at = ?"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&issue.key)
        .bind(&issue.url)
        .bind(&issue.summary)
        .bind(&encrypted_body)
        .bind(&issue.reporter)
        .bind(&issue.project_key)
        .bind(created_at)
        .bind(updated_at)
        .bind(now)
        .bind(&issue.summary)
        .bind(&encrypted_body)
        .bind(updated_at)
        .bind(now)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    async fn store_confluence_page(&self, page: &ConfluencePage) -> Result<(), AtlassianError> {
        let now = chrono::Utc::now().timestamp();
        let created_at = chrono::DateTime::parse_from_rfc3339(&page.created)
            .map(|dt| dt.timestamp())
            .unwrap_or(now);
        let updated_at = chrono::DateTime::parse_from_rfc3339(&page.updated)
            .map(|dt| dt.timestamp())
            .unwrap_or(now);

        let body = page.body.as_deref().unwrap_or("");
        let encrypted_body = self
            .crypto
            .encrypt_string(body)
            .map_err(|e| AtlassianError::Crypto(e.to_string()))?;

        sqlx::query(
            "INSERT INTO content_items (id, source, source_id, source_url, content_type, title, body, author_id, channel_or_project, created_at, updated_at, synced_at)
             VALUES (?, 'confluence', ?, ?, 'page', ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source, source_id) DO UPDATE SET title = ?, body = ?, updated_at = ?, synced_at = ?"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&page.id)
        .bind(&page.url)
        .bind(&page.title)
        .bind(&encrypted_body)
        .bind(&page.author)
        .bind(&page.space_key)
        .bind(created_at)
        .bind(updated_at)
        .bind(now)
        .bind(&page.title)
        .bind(&encrypted_body)
        .bind(updated_at)
        .bind(now)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }
}
