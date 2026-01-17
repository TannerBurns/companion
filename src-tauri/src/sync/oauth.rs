//! Shared OAuth callback handling utilities

use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::timeout;

/// Default timeout for OAuth callback (5 minutes)
const OAUTH_TIMEOUT_SECS: u64 = 300;

#[derive(Error, Debug)]
pub enum OAuthCallbackError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Timeout waiting for OAuth callback")]
    Timeout,
    
    #[error("Invalid callback: {0}")]
    InvalidCallback(String),
    
    #[error("State mismatch")]
    StateMismatch,
    
    #[error("Callback cancelled")]
    Cancelled,
}

pub struct CallbackResult {
    pub code: String,
}

/// Start a local server to receive an OAuth callback with timeout.
pub async fn wait_for_oauth_callback(
    port: u16,
    expected_state: String,
    timeout_secs: Option<u64>,
) -> Result<CallbackResult, OAuthCallbackError> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    handle_oauth_callback(listener, expected_state, timeout_secs).await
}

/// Spawns the OAuth callback listener in a background task and returns a receiver
/// for the authorization code.
/// 
/// DEPRECATED: Use `spawn_oauth_callback_listener_ready` instead to avoid race conditions.
/// This function spawns the listener but doesn't guarantee the port is bound before returning.
pub fn spawn_oauth_callback_listener(
    port: u16,
    expected_state: String,
) -> oneshot::Receiver<Result<String, OAuthCallbackError>> {
    let (tx, rx) = oneshot::channel();
    
    tokio::spawn(async move {
        let result = wait_for_oauth_callback(port, expected_state, None).await;
        let _ = tx.send(result.map(|r| r.code));
    });
    
    rx
}

/// Binds the OAuth callback listener to the port and returns a receiver for the authorization code.
/// 
/// Unlike `spawn_oauth_callback_listener`, this function ensures the listener is bound to the port
/// before returning, eliminating race conditions when opening the browser afterwards.
pub async fn spawn_oauth_callback_listener_ready(
    port: u16,
    expected_state: String,
) -> Result<oneshot::Receiver<Result<String, OAuthCallbackError>>, OAuthCallbackError> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    
    let (tx, rx) = oneshot::channel();
    
    tokio::spawn(async move {
        let result = handle_oauth_callback(listener, expected_state, None).await;
        let _ = tx.send(result.map(|r| r.code));
    });
    
    Ok(rx)
}

/// Handle OAuth callback on an already-bound listener.
async fn handle_oauth_callback(
    listener: TcpListener,
    expected_state: String,
    timeout_secs: Option<u64>,
) -> Result<CallbackResult, OAuthCallbackError> {
    let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(OAUTH_TIMEOUT_SECS));
    let accept_result = timeout(timeout_duration, listener.accept()).await;
    
    let (mut socket, _) = match accept_result {
        Ok(Ok(conn)) => conn,
        Ok(Err(e)) => return Err(OAuthCallbackError::Io(e)),
        Err(_) => return Err(OAuthCallbackError::Timeout),
    };
    
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    let mut buffer = [0u8; 2048];
    let bytes_read = socket.read(&mut buffer).await?;
    
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let (code, state) = parse_oauth_callback(&request)
        .ok_or_else(|| OAuthCallbackError::InvalidCallback("Could not parse callback URL".into()))?;
    
    if state != expected_state {
        let error_response = build_error_response("State mismatch - possible CSRF attack");
        socket.write_all(error_response.as_bytes()).await?;
        return Err(OAuthCallbackError::StateMismatch);
    }
    
    let success_response = build_success_response();
    socket.write_all(success_response.as_bytes()).await?;
    
    Ok(CallbackResult { code })
}

/// Parse authorization code and state from an OAuth callback request
fn parse_oauth_callback(request: &str) -> Option<(String, String)> {
    // Find query string in "GET /path?query HTTP/1.1"
    let query_start = request.find('?')?;
    let query_end = request[query_start..].find(' ')?;
    let query = &request[query_start + 1..query_start + query_end];
    
    let params: HashMap<&str, &str> = query
        .split('&')
        .filter_map(|p| {
            let mut parts = p.split('=');
            Some((parts.next()?, parts.next()?))
        })
        .collect();
    
    let code = params.get("code")?.to_string();
    let state = params.get("state")?.to_string();
    
    Some((code, state))
}

fn build_success_response() -> String {
    "HTTP/1.1 200 OK\r\n\
     Content-Type: text/html\r\n\
     Connection: close\r\n\r\n\
     <!DOCTYPE html>\
     <html><head><title>Authorization Successful</title>\
     <style>body{font-family:system-ui,sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#f0f0f0;}\
     .card{background:white;padding:2rem;border-radius:8px;box-shadow:0 2px 10px rgba(0,0,0,0.1);text-align:center;}\
     h1{color:#22c55e;margin-bottom:0.5rem;}p{color:#666;}</style></head>\
     <body><div class='card'><h1>Authorization Successful</h1>\
     <p>You can close this window and return to the app.</p></div></body></html>"
        .to_string()
}

fn build_error_response(message: &str) -> String {
    format!(
        "HTTP/1.1 400 Bad Request\r\n\
         Content-Type: text/html\r\n\
         Connection: close\r\n\r\n\
         <!DOCTYPE html>\
         <html><head><title>Authorization Failed</title>\
         <style>body{{font-family:system-ui,sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#f0f0f0;}}\
         .card{{background:white;padding:2rem;border-radius:8px;box-shadow:0 2px 10px rgba(0,0,0,0.1);text-align:center;}}\
         h1{{color:#ef4444;margin-bottom:0.5rem;}}p{{color:#666;}}</style></head>\
         <body><div class='card'><h1>Authorization Failed</h1>\
         <p>{}</p></div></body></html>",
        message
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_oauth_callback() {
        let request = "GET /callback?code=abc123&state=xyz789 HTTP/1.1\r\nHost: localhost";
        let result = parse_oauth_callback(request);
        assert!(result.is_some());
        let (code, state) = result.unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "xyz789");
    }
    
    #[test]
    fn test_parse_oauth_callback_invalid() {
        let request = "GET /callback HTTP/1.1\r\nHost: localhost";
        let result = parse_oauth_callback(request);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_oauth_callback_missing_code() {
        let request = "GET /callback?state=xyz789 HTTP/1.1\r\nHost: localhost";
        let result = parse_oauth_callback(request);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_oauth_callback_missing_state() {
        let request = "GET /callback?code=abc123 HTTP/1.1\r\nHost: localhost";
        let result = parse_oauth_callback(request);
        assert!(result.is_none());
    }

    #[test]
    fn test_oauth_callback_error_display() {
        let err = OAuthCallbackError::Timeout;
        assert_eq!(err.to_string(), "Timeout waiting for OAuth callback");

        let err = OAuthCallbackError::StateMismatch;
        assert_eq!(err.to_string(), "State mismatch");

        let err = OAuthCallbackError::Cancelled;
        assert_eq!(err.to_string(), "Callback cancelled");

        let err = OAuthCallbackError::InvalidCallback("bad request".into());
        assert_eq!(err.to_string(), "Invalid callback: bad request");
    }

    #[test]
    fn test_build_success_response_is_valid_http() {
        let response = build_success_response();
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/html"));
        assert!(response.contains("Authorization Successful"));
    }

    #[test]
    fn test_build_error_response_is_valid_http() {
        let response = build_error_response("Test error message");
        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("Content-Type: text/html"));
        assert!(response.contains("Test error message"));
    }
}
