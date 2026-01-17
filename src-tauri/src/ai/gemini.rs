use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Error, Debug)]
pub enum GeminiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
    FunctionCall { function_call: FunctionCall },
    FunctionResponse { function_response: FunctionResponse },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<i32>,
    pub response_mime_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: Content,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: i32,
    pub total_token_count: i32,
}

pub struct GeminiClient {
    http: Client,
    api_key: String,
    model: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            http: Client::new(),
            api_key,
            model: "gemini-3-flash-preview".to_string(),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, GeminiError> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_URL, self.model, self.api_key
        );

        let response = self.http
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(GeminiError::Api(error_text));
        }

        Ok(response.json().await?)
    }

    /// Simple text generation
    pub async fn generate_text(&self, prompt: &str) -> Result<String, GeminiError> {
        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: prompt.to_string() }],
            }],
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.7),
                max_output_tokens: None, // Allow full output from model
                response_mime_type: None,
            }),
        };

        let response = self.generate(request).await?;
        
        response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| GeminiError::Parse("No text in response".into()))
    }

    /// Generate with JSON output
    pub async fn generate_json<T: for<'de> Deserialize<'de>>(
        &self,
        prompt: &str,
    ) -> Result<T, GeminiError> {
        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: prompt.to_string() }],
            }],
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.3),
                max_output_tokens: None, // Allow full output from model
                response_mime_type: Some("application/json".to_string()),
            }),
        };

        let response = self.generate(request).await?;
        
        let text = response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| GeminiError::Parse("No text in response".into()))?;

        serde_json::from_str(&text)
            .map_err(|e| GeminiError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_client_new() {
        let client = GeminiClient::new("test-api-key".into());
        assert_eq!(client.api_key, "test-api-key");
        assert_eq!(client.model, "gemini-3-flash-preview");
    }

    #[test]
    fn test_gemini_client_with_model() {
        let client = GeminiClient::new("key".into())
            .with_model("gemini-1.5-pro");
        assert_eq!(client.model, "gemini-1.5-pro");
    }

    #[test]
    fn test_gemini_error_display() {
        let err = GeminiError::Api("rate_limited".into());
        assert_eq!(err.to_string(), "API error: rate_limited");

        let err = GeminiError::Parse("invalid json".into());
        assert_eq!(err.to_string(), "Parse error: invalid json");
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
        assert!(json.contains("\"temperature\":0.5"));
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
