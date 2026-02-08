//! In-memory OAuth store following the `SessionManager` pattern.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use super::types::{AccessToken, AuthCode, OAuthClient, RefreshToken};

/// Auth code lifetime: 10 minutes.
const AUTH_CODE_LIFETIME: u64 = 600;
/// Access token lifetime: 1 hour.
const ACCESS_TOKEN_LIFETIME: u64 = 3600;
/// Refresh token lifetime: 30 days.
const REFRESH_TOKEN_LIFETIME: u64 = 30 * 24 * 3600;
/// Cleanup interval: 5 minutes.
const CLEANUP_INTERVAL: Duration = Duration::from_secs(300);

/// In-memory OAuth state store.
#[derive(Clone)]
pub struct OAuthStore {
    clients: Arc<RwLock<HashMap<String, OAuthClient>>>,
    auth_codes: Arc<RwLock<HashMap<String, AuthCode>>>,
    access_tokens: Arc<RwLock<HashMap<String, AccessToken>>>,
    refresh_tokens: Arc<RwLock<HashMap<String, RefreshToken>>>,
}

impl OAuthStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            auth_codes: Arc::new(RwLock::new(HashMap::new())),
            access_tokens: Arc::new(RwLock::new(HashMap::new())),
            refresh_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a random token using two UUIDs (256 bits).
    fn generate_token() -> String {
        format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
    }

    /// Register a new OAuth client (Dynamic Client Registration).
    pub async fn register_client(
        &self,
        client_name: Option<String>,
        redirect_uris: Vec<String>,
    ) -> OAuthClient {
        let client_id = uuid::Uuid::new_v4().simple().to_string();

        let client = OAuthClient {
            client_id: client_id.clone(),
            client_name,
            redirect_uris,
            created_at: Instant::now(),
        };

        self.clients.write().await.insert(
            client_id,
            OAuthClient {
                client_id: client.client_id.clone(),
                client_name: client.client_name.clone(),
                redirect_uris: client.redirect_uris.clone(),
                created_at: client.created_at,
            },
        );

        client
    }

    /// Look up a client by ID.
    pub async fn get_client(&self, client_id: &str) -> Option<ClientInfo> {
        let clients = self.clients.read().await;
        clients.get(client_id).map(|c| ClientInfo {
            client_id: c.client_id.clone(),
            client_name: c.client_name.clone(),
            redirect_uris: c.redirect_uris.clone(),
        })
    }

    /// Create an authorization code for an approved request.
    pub async fn create_auth_code(
        &self,
        client_id: String,
        redirect_uri: String,
        code_challenge: String,
        scope: String,
    ) -> String {
        let code = Self::generate_token();

        self.auth_codes.write().await.insert(
            code.clone(),
            AuthCode {
                client_id,
                redirect_uri,
                code_challenge,
                scope,
                created_at: Instant::now(),
                used: false,
            },
        );

        code
    }

    /// Consume an authorization code (one-time use).
    ///
    /// Returns the code details if valid, unused, and not expired.
    pub async fn consume_auth_code(&self, code: &str) -> Option<AuthCodeInfo> {
        let mut codes = self.auth_codes.write().await;
        let auth_code = codes.get_mut(code)?;

        if auth_code.used || auth_code.is_expired() {
            return None;
        }

        auth_code.used = true;

        Some(AuthCodeInfo {
            client_id: auth_code.client_id.clone(),
            redirect_uri: auth_code.redirect_uri.clone(),
            code_challenge: auth_code.code_challenge.clone(),
            scope: auth_code.scope.clone(),
        })
    }

    /// Create an access + refresh token pair.
    pub async fn create_token_pair(&self, client_id: &str, scope: &str) -> TokenPair {
        let access = Self::generate_token();
        let refresh = Self::generate_token();

        self.access_tokens.write().await.insert(
            access.clone(),
            AccessToken {
                client_id: client_id.to_owned(),
                created_at: Instant::now(),
                expires_in: ACCESS_TOKEN_LIFETIME,
            },
        );

        self.refresh_tokens.write().await.insert(
            refresh.clone(),
            RefreshToken {
                client_id: client_id.to_owned(),
                access_token: access.clone(),
                scope: scope.to_owned(),
                created_at: Instant::now(),
                expires_in: REFRESH_TOKEN_LIFETIME,
            },
        );

        TokenPair {
            access_token: access,
            refresh_token: refresh,
            expires_in: ACCESS_TOKEN_LIFETIME,
            scope: scope.to_owned(),
        }
    }

    /// Validate an access token. Returns the client_id if valid.
    pub async fn validate_access_token(&self, token: &str) -> Option<String> {
        let tokens = self.access_tokens.read().await;
        let access = tokens.get(token)?;
        if access.is_expired() {
            return None;
        }
        Some(access.client_id.clone())
    }

    /// Refresh a token pair: invalidate old tokens and issue new ones.
    pub async fn refresh_token_pair(&self, refresh_token: &str) -> Option<TokenPair> {
        // Validate and remove the old refresh token
        let old = {
            let mut tokens = self.refresh_tokens.write().await;
            tokens.remove(refresh_token)?
        };

        if old.is_expired() {
            return None;
        }

        // Remove old access token
        self.access_tokens.write().await.remove(&old.access_token);

        // Issue new pair
        Some(self.create_token_pair(&old.client_id, &old.scope).await)
    }

    /// Start background cleanup task for expired tokens and codes.
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
            loop {
                interval.tick().await;
                self.cleanup_expired().await;
            }
        });
    }

    async fn cleanup_expired(&self) {
        let now = Instant::now();

        // Cleanup auth codes (expired or used and older than 1 minute)
        {
            let mut codes = self.auth_codes.write().await;
            codes.retain(|_, code| {
                if code.is_expired() {
                    return false;
                }
                // Keep used codes for a short while (for error messages), then remove
                if code.used && now.duration_since(code.created_at).as_secs() > AUTH_CODE_LIFETIME {
                    return false;
                }
                true
            });
        }

        // Cleanup access tokens
        {
            let mut tokens = self.access_tokens.write().await;
            let before = tokens.len();
            tokens.retain(|_, token| !token.is_expired());
            let removed = before - tokens.len();
            if removed > 0 {
                tracing::debug!(count = removed, "Cleaned up expired access tokens");
            }
        }

        // Cleanup refresh tokens
        {
            let mut tokens = self.refresh_tokens.write().await;
            let before = tokens.len();
            tokens.retain(|_, token| !token.is_expired());
            let removed = before - tokens.len();
            if removed > 0 {
                tracing::debug!(count = removed, "Cleaned up expired refresh tokens");
            }
        }
    }
}

impl Default for OAuthStore {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for OAuthStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthStore").finish()
    }
}

/// Subset of client info returned from lookups.
pub struct ClientInfo {
    pub client_id: String,
    pub client_name: Option<String>,
    pub redirect_uris: Vec<String>,
}

/// Subset of auth code info returned from consume.
pub struct AuthCodeInfo {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub scope: String,
}

/// A token pair returned from token creation/refresh.
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub scope: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_registration() {
        let store = OAuthStore::new();
        let client = store
            .register_client(Some("Test App".into()), vec!["http://localhost/callback".into()])
            .await;

        assert!(!client.client_id.is_empty());

        let info = store.get_client(&client.client_id).await;
        assert!(info.is_some());
        assert_eq!(info.unwrap().client_name.as_deref(), Some("Test App"));
    }

    #[tokio::test]
    async fn test_auth_code_lifecycle() {
        let store = OAuthStore::new();

        let code = store
            .create_auth_code(
                "client1".into(),
                "http://localhost/callback".into(),
                "challenge".into(),
                "mcp".into(),
            )
            .await;

        // First consume succeeds
        let info = store.consume_auth_code(&code).await;
        assert!(info.is_some());
        assert_eq!(info.unwrap().client_id, "client1");

        // Second consume fails (already used)
        assert!(store.consume_auth_code(&code).await.is_none());
    }

    #[tokio::test]
    async fn test_token_lifecycle() {
        let store = OAuthStore::new();
        let pair = store.create_token_pair("client1", "mcp").await;

        // Validate access token
        let client_id = store.validate_access_token(&pair.access_token).await;
        assert_eq!(client_id.as_deref(), Some("client1"));

        // Invalid token
        assert!(store.validate_access_token("invalid").await.is_none());
    }

    #[tokio::test]
    async fn test_refresh_token() {
        let store = OAuthStore::new();
        let pair = store.create_token_pair("client1", "mcp").await;

        // Refresh
        let new_pair = store.refresh_token_pair(&pair.refresh_token).await;
        assert!(new_pair.is_some());
        let new_pair = new_pair.unwrap();

        // Old access token is invalid
        assert!(store.validate_access_token(&pair.access_token).await.is_none());

        // New access token is valid
        assert!(store.validate_access_token(&new_pair.access_token).await.is_some());

        // Old refresh token can't be reused
        assert!(store.refresh_token_pair(&pair.refresh_token).await.is_none());
    }
}
