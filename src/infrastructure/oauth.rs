//! OAuth authentication implementation

use crate::application::ports::OAuthPort;
use crate::domain::{
    errors::{MCPError, MCPResult},
    services::EnvResolver,
    value_objects::OAuthConfig,
};
use async_trait::async_trait;
use axum::{
    extract::Query,
    response::Html,
    routing::get,
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::oneshot;
use url::Url;

/// OAuth service implementation
pub struct OAuthServiceImpl;

impl OAuthServiceImpl {
    pub fn new() -> Self {
        Self
    }

    /// Configure OAuth and get access token
    async fn configure_oauth(&self, config: &OAuthConfig) -> MCPResult<String> {
        // If we have a static access token, use it directly
        if let Some(token) = &config.access_token {
            let resolved = EnvResolver::resolve(token);
            return Ok(resolved);
        }

        // If we have full OAuth config, perform OAuth flow
        if config.client_id.is_some()
            && config.authorization_url.is_some()
            && config.token_url.is_some()
        {
            return self.perform_oauth_flow(config).await;
        }

        // No authentication configured
        Ok(String::new())
    }

    /// Perform OAuth 2.1 Authorization Code flow with PKCE
    async fn perform_oauth_flow(&self, config: &OAuthConfig) -> MCPResult<String> {
        let client_id = config
            .client_id
            .as_ref()
            .ok_or_else(|| MCPError::auth_failed("Missing client_id"))?
            .clone();
        let auth_url = config
            .authorization_url
            .as_ref()
            .ok_or_else(|| MCPError::auth_failed("Missing authorization_url"))?
            .clone();
        let token_url = config
            .token_url
            .as_ref()
            .ok_or_else(|| MCPError::auth_failed("Missing token_url"))?
            .clone();

        // Generate PKCE parameters
        let code_verifier = generate_code_verifier(32);
        let code_challenge = generate_code_challenge(&code_verifier);
        let state = generate_state(16);

        // Create channels for callback
        let (code_tx, code_rx) = oneshot::channel::<String>();
        let (err_tx, err_rx) = oneshot::channel::<String>();

        let code_tx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(code_tx)));
        let err_tx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(err_tx)));
        let state_check = state.clone();

        // Create callback server
        let app = Router::new().route(
            "/callback",
            get(
                move |Query(params): Query<HashMap<String, String>>| async move {
                    let code = params.get("code").cloned();
                    let state_received = params.get("state").cloned();

                    if let Some(code) = code {
                        if state_received.as_ref() == Some(&state_check) {
                            if let Some(tx) = code_tx.lock().await.take() {
                                let _ = tx.send(code);
                            }
                            Html("<html><body><h1>Authorization Complete</h1><p>You can close this window.</p></body></html>")
                        } else {
                            if let Some(tx) = err_tx.lock().await.take() {
                                let _ = tx.send("State mismatch".to_string());
                            }
                            Html("<html><body><h1>Authorization Failed</h1><p>State mismatch.</p></body></html>")
                        }
                    } else {
                        if let Some(tx) = err_tx.lock().await.take() {
                            let _ = tx.send("No code in callback".to_string());
                        }
                        Html("<html><body><h1>Authorization Failed</h1><p>No authorization code received.</p></body></html>")
                    }
                },
            ),
        );

        // Find an available port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
            MCPError::auth_failed(format!("Failed to bind callback server: {}", e))
        })?;
        let port = listener.local_addr().map_err(|e| {
            MCPError::auth_failed(format!("Failed to get local address: {}", e))
        })?.port();
        let callback_url = format!("http://127.0.0.1:{}/callback", port);

        // Start server
        let server = axum::serve(listener, app);
        let server_handle = tokio::spawn(async move {
            server.await.ok();
        });

        // Build authorization URL
        let auth_url = build_auth_url(
            &auth_url,
            &EnvResolver::resolve(&client_id),
            &callback_url,
            &state,
            &code_challenge,
            config.scopes.as_deref(),
        ).map_err(|e| MCPError::auth_failed(format!("Failed to build auth URL: {}", e)))?;

        // Open browser
        println!("Opening browser for OAuth authorization...");
        println!("Auth URL: {}", auth_url);
        
        if let Err(e) = open_browser(&auth_url).await {
            println!("Failed to open browser: {}. Please open the URL manually.", e);
        }

        // Wait for callback with timeout
        let result = tokio::time::timeout(Duration::from_secs(300), async {
            tokio::select! {
                code = code_rx => code.map_err(|_| "Channel closed".to_string()),
                err = err_rx => Err(err.unwrap_or_else(|_| "Unknown error".to_string())),
            }
        }).await;

        // Shutdown server
        server_handle.abort();

        let code = match result {
            Ok(Ok(code)) => code,
            Ok(Err(e)) => return Err(MCPError::auth_failed(format!("OAuth error: {}", e))),
            Err(_) => return Err(MCPError::auth_failed("OAuth authorization timed out after 5 minutes")),
        };

        // Exchange code for token
        self.exchange_code(
            &token_url,
            &code,
            &code_verifier,
            &callback_url,
            &client_id,
            config.client_secret.as_deref(),
        ).await
    }

    /// Exchange authorization code for access token
    async fn exchange_code(
        &self,
        token_url: &str,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        client_id: &str,
        client_secret: Option<&str>,
    ) -> MCPResult<String> {
        let client = reqwest::Client::new();

        let resolved_client_id = EnvResolver::resolve(client_id);
        let resolved_secret = client_secret.map(EnvResolver::resolve);
        
        let mut params: HashMap<&str, &str> = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", redirect_uri);
        params.insert("code_verifier", code_verifier);
        params.insert("client_id", &resolved_client_id);

        if let Some(ref secret) = resolved_secret {
            params.insert("client_secret", secret);
        }

        let response = client
            .post(token_url)
            .form(&params)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| MCPError::auth_failed(format!("Token request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(MCPError::auth_failed(format!(
                "Token exchange failed: {}",
                body
            )));
        }

        let token_response: serde_json::Value = response.json().await.map_err(|e| {
            MCPError::auth_failed(format!("Failed to parse token response: {}", e))
        })?;

        let access_token = token_response
            .get("access_token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| MCPError::auth_failed("No access token in response"))?;

        Ok(access_token.to_string())
    }
}

impl Default for OAuthServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OAuthPort for OAuthServiceImpl {
    async fn authenticate(&self, config: &OAuthConfig) -> MCPResult<String> {
        self.configure_oauth(config).await
    }
}

/// Generate PKCE code verifier
fn generate_code_verifier(length: usize) -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..length).map(|_| rng.random()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Generate PKCE code challenge from verifier
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(&hash)
}

/// Generate random state parameter
fn generate_state(length: usize) -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..length).map(|_| rng.random()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Build OAuth authorization URL
fn build_auth_url(
    auth_url: &str,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
    scopes: Option<&str>,
) -> anyhow::Result<String> {
    let mut url = Url::parse(auth_url)?;

    let scopes = scopes.map(EnvResolver::resolve).unwrap_or_else(|| "openid profile".to_string());

    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", &scopes)
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");

    Ok(url.to_string())
}

/// Open URL in default browser
async fn open_browser(url: &str) -> anyhow::Result<()> {
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "start"
    } else {
        "xdg-open"
    };

    if cfg!(target_os = "windows") {
        tokio::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to open browser: {}", e))?;
    } else {
        tokio::process::Command::new(cmd)
            .arg(url)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to open browser: {}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_verifier() {
        let verifier = generate_code_verifier(32);
        assert_eq!(verifier.len(), 43); // base64 encoding of 32 bytes
    }

    #[test]
    fn test_generate_code_challenge() {
        let verifier = "test_verifier";
        let challenge = generate_code_challenge(verifier);
        assert!(!challenge.is_empty());
        assert_ne!(challenge, verifier);
    }

    #[test]
    fn test_build_auth_url() {
        let url = build_auth_url(
            "https://auth.example.com/authorize",
            "client123",
            "http://localhost:7777/callback",
            "state123",
            "challenge123",
            Some("read write"),
        ).unwrap();

        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=client123"));
        assert!(url.contains("state=state123"));
        assert!(url.contains("code_challenge=challenge123"));
        assert!(url.contains("scope=read+write"));
    }
}
