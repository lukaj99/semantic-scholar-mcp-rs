//! Session management for robust MCP connections.
//!
//! Implements the "Mailbox" pattern for never-failing connections:
//! - In-memory session state with ring buffer for message replay
//! - Last-Event-ID support for reconnection recovery
//! - Broadcast channels for live event delivery
//! - Background cleanup of stale sessions

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::response::sse::Event;
use tokio::sync::{broadcast, RwLock};

/// Maximum number of events to keep in history per session.
const HISTORY_SIZE: usize = 100;

/// Session timeout after which sessions are cleaned up.
const SESSION_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour

/// Cleanup interval for stale sessions.
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

/// A buffered SSE event with ID for replay support.
#[derive(Clone, Debug)]
pub struct BufferedEvent {
    /// Unique event ID (monotonically increasing per session).
    pub id: u64,
    /// Event type (e.g., "message", "endpoint").
    pub event_type: String,
    /// JSON payload.
    pub data: String,
    /// Timestamp when event was created.
    pub created_at: Instant,
}

impl BufferedEvent {
    /// Create a new buffered event.
    pub fn new(id: u64, event_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            id,
            event_type: event_type.into(),
            data: data.into(),
            created_at: Instant::now(),
        }
    }

    /// Convert to an Axum SSE Event.
    pub fn to_sse_event(&self) -> Event {
        Event::default()
            .id(self.id.to_string())
            .event(self.event_type.clone())
            .data(self.data.clone())
    }
}

/// A single MCP session with message buffer and broadcast channel.
pub struct Session {
    /// Unique session identifier.
    pub id: String,
    /// Broadcast sender for live events.
    tx: broadcast::Sender<BufferedEvent>,
    /// Ring buffer of recent events for replay.
    history: RwLock<VecDeque<BufferedEvent>>,
    /// Next event ID (monotonically increasing).
    next_event_id: AtomicU64,
    /// When the session was created.
    pub created_at: Instant,
    /// Last activity timestamp.
    last_active: RwLock<Instant>,
}

impl Session {
    /// Create a new session.
    pub fn new(id: String) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            id,
            tx,
            history: RwLock::new(VecDeque::with_capacity(HISTORY_SIZE)),
            next_event_id: AtomicU64::new(1),
            created_at: Instant::now(),
            last_active: RwLock::new(Instant::now()),
        }
    }

    /// Push an event to the session (stores in history and broadcasts).
    pub async fn push_event(&self, event_type: impl Into<String>, data: impl Into<String>) -> u64 {
        let id = self.next_event_id.fetch_add(1, Ordering::SeqCst);
        let event = BufferedEvent::new(id, event_type, data);

        // Store in history
        {
            let mut history = self.history.write().await;
            if history.len() >= HISTORY_SIZE {
                history.pop_front();
            }
            history.push_back(event.clone());
        }

        // Broadcast to active subscribers (ignore if no subscribers)
        let _ = self.tx.send(event);

        // Update activity timestamp
        *self.last_active.write().await = Instant::now();

        id
    }

    /// Get events after a given ID (for replay on reconnection).
    pub async fn get_events_after(&self, last_event_id: u64) -> Vec<BufferedEvent> {
        let history = self.history.read().await;
        history
            .iter()
            .filter(|e| e.id > last_event_id)
            .cloned()
            .collect()
    }

    /// Subscribe to live events.
    pub fn subscribe(&self) -> broadcast::Receiver<BufferedEvent> {
        self.tx.subscribe()
    }

    /// Check if session is stale.
    pub async fn is_stale(&self) -> bool {
        let last_active = *self.last_active.read().await;
        last_active.elapsed() > SESSION_TIMEOUT
    }

    /// Update last activity timestamp.
    pub async fn touch(&self) {
        *self.last_active.write().await = Instant::now();
    }

    /// Get current event ID (for debugging).
    pub fn current_event_id(&self) -> u64 {
        self.next_event_id.load(Ordering::SeqCst)
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("current_event_id", &self.current_event_id())
            .field("created_at", &self.created_at)
            .finish()
    }
}

/// Global session manager.
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<Session>>>>,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session and return its ID.
    pub async fn create_session(&self) -> Arc<Session> {
        let id = uuid::Uuid::new_v4().to_string();
        let session = Arc::new(Session::new(id.clone()));

        self.sessions.write().await.insert(id, session.clone());

        tracing::info!(session_id = %session.id, "Created new session");
        session
    }

    /// Get an existing session by ID.
    pub async fn get_session(&self, id: &str) -> Option<Arc<Session>> {
        let sessions = self.sessions.read().await;
        sessions.get(id).cloned()
    }

    /// Get or create a session.
    pub async fn get_or_create_session(&self, id: Option<&str>) -> Arc<Session> {
        if let Some(id) = id {
            if let Some(session) = self.get_session(id).await {
                session.touch().await;
                return session;
            }
        }
        self.create_session().await
    }

    /// Remove a session.
    pub async fn remove_session(&self, id: &str) -> bool {
        let removed = self.sessions.write().await.remove(id).is_some();
        if removed {
            tracing::info!(session_id = %id, "Removed session");
        }
        removed
    }

    /// Clean up stale sessions.
    pub async fn cleanup_stale_sessions(&self) -> usize {
        let mut to_remove = Vec::new();

        {
            let sessions = self.sessions.read().await;
            for (id, session) in sessions.iter() {
                if session.is_stale().await {
                    to_remove.push(id.clone());
                }
            }
        }

        let count = to_remove.len();
        if count > 0 {
            let mut sessions = self.sessions.write().await;
            for id in to_remove {
                sessions.remove(&id);
                tracing::info!(session_id = %id, "Cleaned up stale session");
            }
        }

        count
    }

    /// Get session count (for monitoring).
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Start background cleanup task.
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
            loop {
                interval.tick().await;
                let cleaned = self.cleanup_stale_sessions().await;
                if cleaned > 0 {
                    tracing::debug!(count = cleaned, "Session cleanup completed");
                }
            }
        });
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionManager").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new();
        let session = manager.create_session().await;

        assert!(!session.id.is_empty());
        assert_eq!(manager.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_event_push_and_replay() {
        let session = Session::new("test".to_string());

        // Push events
        let id1 = session.push_event("message", r#"{"test": 1}"#).await;
        let id2 = session.push_event("message", r#"{"test": 2}"#).await;
        let id3 = session.push_event("message", r#"{"test": 3}"#).await;

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);

        // Replay from id 1
        let events = session.get_events_after(1).await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id, 2);
        assert_eq!(events[1].id, 3);
    }

    #[tokio::test]
    async fn test_session_lookup() {
        let manager = SessionManager::new();
        let session = manager.create_session().await;
        let id = session.id.clone();

        let found = manager.get_session(&id).await;
        assert!(found.is_some());

        let not_found = manager.get_session("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_ring_buffer_overflow() {
        let session = Session::new("test".to_string());

        // Push more than HISTORY_SIZE events
        for i in 0..150 {
            session.push_event("message", format!(r#"{{"n": {}}}"#, i)).await;
        }

        // Should only keep HISTORY_SIZE events
        let events = session.get_events_after(0).await;
        assert_eq!(events.len(), HISTORY_SIZE);

        // First event should be 51 (0-50 were evicted)
        assert_eq!(events[0].id, 51);
    }
}
