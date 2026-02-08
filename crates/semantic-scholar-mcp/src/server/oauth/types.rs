//! OAuth 2.0 types for MCP authentication.

use std::time::Instant;

/// A dynamically registered OAuth client.
pub struct OAuthClient {
    pub client_id: String,
    pub client_name: Option<String>,
    pub redirect_uris: Vec<String>,
    pub created_at: Instant,
}

/// An authorization code issued after user approval.
pub struct AuthCode {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub scope: String,
    pub created_at: Instant,
    pub used: bool,
}

/// An access token for API authentication.
pub struct AccessToken {
    pub client_id: String,
    pub created_at: Instant,
    pub expires_in: u64,
}

/// A refresh token for obtaining new access tokens.
pub struct RefreshToken {
    pub client_id: String,
    pub access_token: String,
    pub scope: String,
    pub created_at: Instant,
    pub expires_in: u64,
}

impl AccessToken {
    /// Check if the token has expired.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > self.expires_in
    }
}

impl RefreshToken {
    /// Check if the token has expired.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > self.expires_in
    }
}

impl AuthCode {
    /// Check if the code has expired (10 minute lifetime).
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > 600
    }
}
