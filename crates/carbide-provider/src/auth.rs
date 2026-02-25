//! Authentication middleware for the provider HTTP server.
//!
//! Supports two authentication methods:
//! - **Bearer JWT**: `Authorization: Bearer <token>` (stateless, preferred)
//! - **API Key**: `X-API-Key: <key>` (validated by SHA-256 hash comparison)
//!
//! Health and status endpoints are always public.

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::warn;

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Whether authentication is enabled
    pub enabled: bool,
    /// JWT secret for token verification
    pub jwt_secret: Option<String>,
    /// SHA-256 hashes of accepted API keys
    pub api_key_hashes: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: None,
            api_key_hashes: Vec::new(),
        }
    }
}

/// JWT claims payload
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Key or subject identifier
    pub sub: String,
    /// Role (admin or provider)
    pub role: String,
    /// Issued at
    pub iat: usize,
    /// Expiration
    pub exp: usize,
}

/// Context attached to authenticated requests
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// Subject identifier
    pub subject: String,
    /// Role
    pub role: String,
}

/// Server state wrapper that includes auth config
#[derive(Debug, Clone)]
pub struct AuthState {
    /// Auth configuration
    pub config: AuthConfig,
}

/// Hash an API key using SHA-256
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Axum middleware function for authentication.
///
/// Public endpoints (health, status) are always allowed through.
/// All other endpoints require valid Bearer JWT or API key when auth is enabled.
pub async fn auth_middleware(
    State(auth): State<Arc<AuthState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !auth.config.enabled {
        return Ok(next.run(request).await);
    }

    // Public endpoints — always bypass auth
    let path = request.uri().path();
    if path.starts_with("/api/v1/health") || path.starts_with("/api/v1/provider/status") {
        return Ok(next.run(request).await);
    }

    // Try Bearer JWT
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(header_str) = auth_header.to_str() {
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                if let Some(ref secret) = auth.config.jwt_secret {
                    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
                    let validation = Validation::default();
                    match decode::<Claims>(token, &decoding_key, &validation) {
                        Ok(_token_data) => {
                            return Ok(next.run(request).await);
                        }
                        Err(e) => {
                            warn!("JWT validation failed: {}", e);
                        }
                    }
                }
            }
        }
    }

    // Try API key
    if let Some(api_key_header) = request.headers().get("x-api-key") {
        if let Ok(api_key) = api_key_header.to_str() {
            let key_hash = hash_api_key(api_key);
            if auth.config.api_key_hashes.contains(&key_hash) {
                return Ok(next.run(request).await);
            }
            warn!("Invalid API key presented");
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_api_key() {
        let key = "cbk_test_key_12345";
        let h1 = hash_api_key(key);
        let h2 = hash_api_key(key);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_different_keys_produce_different_hashes() {
        let h1 = hash_api_key("key1");
        let h2 = hash_api_key("key2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert!(config.jwt_secret.is_none());
        assert!(config.api_key_hashes.is_empty());
    }
}
