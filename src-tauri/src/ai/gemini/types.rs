use serde::{Deserialize, Serialize};
use thiserror::Error;

/// API URL for the Generative Language API
pub const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Default region for Vertex AI
pub const DEFAULT_VERTEX_REGION: &str = "global";

/// Errors that can occur when interacting with the Gemini API.
#[derive(Error, Debug)]
pub enum GeminiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("JWT error: {0}")]
    Jwt(String),
    #[error("Auth error: {0}")]
    Auth(String),
}

/// Tool definition for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

/// Function declaration for tool use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Request body for content generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

/// Content message (role + parts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    #[serde(default)]
    pub parts: Vec<Part>,
}

/// Part of a content message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
    FunctionCall { function_call: FunctionCall },
    FunctionResponse { function_response: FunctionResponse },
}

/// Function call from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// Response to a function call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

/// Configuration for content generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<i32>,
    pub response_mime_type: Option<String>,
}

/// Response from content generation.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: Option<UsageMetadata>,
}

/// A generated candidate response.
#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: Content,
    pub finish_reason: Option<String>,
}

/// Token usage metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: i32,
    pub total_token_count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_error_display() {
        let err = GeminiError::Api("rate_limited".into());
        assert_eq!(err.to_string(), "API error: rate_limited");

        let err = GeminiError::Parse("invalid json".into());
        assert_eq!(err.to_string(), "Parse error: invalid json");
    }

    #[test]
    fn test_gemini_error_jwt_display() {
        let err = GeminiError::Jwt("invalid key format".into());
        assert_eq!(err.to_string(), "JWT error: invalid key format");
    }

    #[test]
    fn test_gemini_error_auth_display() {
        let err = GeminiError::Auth("token exchange failed".into());
        assert_eq!(err.to_string(), "Auth error: token exchange failed");
    }

    #[test]
    fn test_content_deserialization_missing_parts() {
        let json = r#"{"role": "model"}"#;
        let content: Content = serde_json::from_str(json).unwrap();
        assert_eq!(content.role, "model");
        assert!(content.parts.is_empty());
    }

    #[test]
    fn test_content_deserialization_with_parts() {
        let json = r#"{"role": "user", "parts": [{"text": "hello"}]}"#;
        let content: Content = serde_json::from_str(json).unwrap();
        assert_eq!(content.role, "user");
        assert_eq!(content.parts.len(), 1);
    }

    #[test]
    fn test_content_serialization() {
        let content = Content {
            role: "user".into(),
            parts: vec![Part::Text { text: "Hello".into() }],
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn test_generate_request_skips_none_fields() {
        let request = GenerateRequest {
            contents: vec![],
            tools: None,
            generation_config: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("tools"));
        assert!(!json.contains("generation_config"));
    }

    #[test]
    fn test_generate_request_includes_optional_fields() {
        let request = GenerateRequest {
            contents: vec![],
            tools: Some(vec![Tool {
                function_declarations: vec![FunctionDeclaration {
                    name: "test_fn".into(),
                    description: "A test function".into(),
                    parameters: serde_json::json!({}),
                }],
            }]),
            generation_config: Some(GenerationConfig {
                temperature: Some(0.5),
                max_output_tokens: Some(1024),
                response_mime_type: None,
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("tools"));
        assert!(json.contains("test_fn"));
        assert!(json.contains("generation_config"));
    }

    #[test]
    fn test_generate_response_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello back!"}]
                },
                "finish_reason": "STOP"
            }],
            "usage_metadata": {
                "prompt_token_count": 10,
                "candidates_token_count": 5,
                "total_token_count": 15
            }
        }"#;

        let response: GenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(response.candidates[0].finish_reason, Some("STOP".into()));
        
        let usage = response.usage_metadata.unwrap();
        assert_eq!(usage.prompt_token_count, 10);
        assert_eq!(usage.total_token_count, 15);
    }

    #[test]
    fn test_part_text_serialization() {
        let part = Part::Text { text: "Hello".into() };
        let json = serde_json::to_string(&part).unwrap();
        assert_eq!(json, r#"{"text":"Hello"}"#);
    }

    #[test]
    fn test_part_function_call_serialization() {
        let part = Part::FunctionCall {
            function_call: FunctionCall {
                name: "get_weather".into(),
                args: serde_json::json!({"location": "NYC"}),
            },
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("function_call"));
        assert!(json.contains("get_weather"));
    }
}
