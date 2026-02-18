//! Atlassian data types and error definitions

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AtlassianError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("OAuth error: {0}")]
    OAuth(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlassianTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudResource {
    pub id: String,
    pub name: String,
    pub url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub id: String,
    pub key: String,
    pub summary: String,
    pub description: Option<String>,
    pub status: String,
    pub assignee: Option<String>,
    pub reporter: String,
    pub project_key: String,
    pub created: String,
    pub updated: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfluencePage {
    pub id: String,
    pub title: String,
    pub space_key: String,
    pub body: Option<String>,
    pub author: String,
    pub created: String,
    pub updated: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlassian_tokens_serialization() {
        let tokens = AtlassianTokens {
            access_token: "eyJ...".into(),
            refresh_token: Some("refresh123".into()),
            expires_in: 3600,
            scope: "read:jira-work".into(),
        };

        let json = serde_json::to_string(&tokens).unwrap();
        let parsed: AtlassianTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "eyJ...");
        assert_eq!(parsed.refresh_token, Some("refresh123".into()));
        assert_eq!(parsed.expires_in, 3600);
    }

    #[test]
    fn test_atlassian_tokens_without_refresh() {
        let tokens = AtlassianTokens {
            access_token: "token".into(),
            refresh_token: None,
            expires_in: 3600,
            scope: "read:jira-work".into(),
        };

        let json = serde_json::to_string(&tokens).unwrap();
        let parsed: AtlassianTokens = serde_json::from_str(&json).unwrap();
        assert!(parsed.refresh_token.is_none());
    }

    #[test]
    fn test_cloud_resource_serialization() {
        let resource = CloudResource {
            id: "abc-123".into(),
            name: "My Workspace".into(),
            url: "https://myworkspace.atlassian.net".into(),
            scopes: vec![
                "read:jira-work".into(),
                "read:confluence-content.all".into(),
            ],
        };

        let json = serde_json::to_string(&resource).unwrap();
        let parsed: CloudResource = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "abc-123");
        assert_eq!(parsed.scopes.len(), 2);
    }

    #[test]
    fn test_jira_issue_serialization() {
        let issue = JiraIssue {
            id: "10001".into(),
            key: "TEST-123".into(),
            summary: "Fix the bug".into(),
            description: Some("Detailed description".into()),
            status: "In Progress".into(),
            assignee: Some("John Doe".into()),
            reporter: "Jane Smith".into(),
            project_key: "TEST".into(),
            created: "2024-01-15T10:00:00Z".into(),
            updated: "2024-01-16T14:30:00Z".into(),
            url: "https://test.atlassian.net/browse/TEST-123".into(),
        };

        let json = serde_json::to_string(&issue).unwrap();
        let parsed: JiraIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key, "TEST-123");
        assert_eq!(parsed.description, Some("Detailed description".into()));
    }

    #[test]
    fn test_jira_issue_without_optional_fields() {
        let issue = JiraIssue {
            id: "10001".into(),
            key: "TEST-456".into(),
            summary: "No description".into(),
            description: None,
            status: "Open".into(),
            assignee: None,
            reporter: "Bot".into(),
            project_key: "TEST".into(),
            created: "2024-01-15T10:00:00Z".into(),
            updated: "2024-01-15T10:00:00Z".into(),
            url: "https://test.atlassian.net/browse/TEST-456".into(),
        };

        let json = serde_json::to_string(&issue).unwrap();
        let parsed: JiraIssue = serde_json::from_str(&json).unwrap();
        assert!(parsed.description.is_none());
        assert!(parsed.assignee.is_none());
    }

    #[test]
    fn test_confluence_page_serialization() {
        let page = ConfluencePage {
            id: "12345".into(),
            title: "Getting Started".into(),
            space_key: "DOCS".into(),
            body: Some("<p>Welcome</p>".into()),
            author: "Admin".into(),
            created: "2024-01-10T08:00:00Z".into(),
            updated: "2024-01-12T16:00:00Z".into(),
            url: "https://test.atlassian.net/wiki/spaces/DOCS/pages/12345".into(),
        };

        let json = serde_json::to_string(&page).unwrap();
        let parsed: ConfluencePage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.title, "Getting Started");
        assert_eq!(parsed.space_key, "DOCS");
    }

    #[test]
    fn test_atlassian_error_display() {
        let err = AtlassianError::OAuth("Invalid grant".into());
        assert_eq!(err.to_string(), "OAuth error: Invalid grant");

        let err = AtlassianError::Api("Not found".into());
        assert_eq!(err.to_string(), "API error: Not found");

        let err = AtlassianError::Crypto("Decryption failed".into());
        assert_eq!(err.to_string(), "Crypto error: Decryption failed");
    }
}
