//! Transport layer implementations.
//!
//! Provides stdio and HTTP transports for the MCP server.
//! Implements "never-failing" connection handling with:
//! - Session-based message buffering
//! - Last-Event-ID replay on reconnection
//! - Broadcast channels for live event delivery
//! - Async tool execution decoupled from HTTP handlers

use std::borrow::Cow;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use super::session::SessionManager;
use crate::tools::{McpTool, ToolContext};

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 error.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    /// JSON-RPC version constant.
    const VERSION: &'static str = "2.0";

    #[must_use]
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: Cow::Borrowed(Self::VERSION),
            result: Some(result),
            error: None,
            id,
        }
    }

    #[must_use]
    pub fn error(id: Option<serde_json::Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: Cow::Borrowed(Self::VERSION),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }
}

/// MCP tool info for tools/list response.
#[derive(Debug, Serialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// Query parameters for SSE endpoint.
#[derive(Debug, Deserialize)]
pub struct SseQuery {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

/// Query parameters for message endpoint.
#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

/// Shared state for HTTP handlers.
pub struct HttpState {
    pub tools: Vec<Box<dyn McpTool>>,
    pub ctx: ToolContext,
    pub sessions: Arc<SessionManager>,
    /// Base URL for endpoint announcements.
    pub base_url: String,
}

/// Create the HTTP router for MCP.
pub fn create_router(
    tools: Vec<Box<dyn McpTool>>,
    ctx: ToolContext,
    base_url: Option<String>,
) -> Router {
    let sessions = Arc::new(SessionManager::new());

    // Start background cleanup task
    Arc::clone(&sessions).start_cleanup_task();

    let base_url = base_url.unwrap_or_else(|| "https://scholar.jovanovic.org.uk".to_string());

    let state = Arc::new(HttpState {
        tools,
        ctx,
        sessions,
        base_url,
    });

    Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        // Streamable HTTP transport - single endpoint
        .route("/mcp", post(handle_mcp_post).get(handle_mcp_get))
        // Legacy SSE transport for backward compatibility
        .route("/sse", get(handle_sse_legacy))
        .route("/message", post(handle_message_post))
        // Session management
        .route("/sessions", get(handle_sessions_list))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "semantic-scholar-mcp",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn readiness_check(State(state): State<Arc<HttpState>>) -> impl IntoResponse {
    let session_count = state.sessions.session_count().await;
    Json(serde_json::json!({
        "status": "ready",
        "service": "semantic-scholar-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "sessions": session_count,
        "tools": state.tools.len()
    }))
}

/// List active sessions (for debugging).
async fn handle_sessions_list(State(state): State<Arc<HttpState>>) -> impl IntoResponse {
    let count = state.sessions.session_count().await;
    Json(serde_json::json!({
        "count": count
    }))
}

/// Handle POST requests to /mcp (Streamable HTTP transport)
async fn handle_mcp_post(
    State(state): State<Arc<HttpState>>,
    Query(query): Query<MessageQuery>,
    Json(req): Json<JsonRpcRequest>,
) -> Response {
    tracing::debug!(method = %req.method, "Handling MCP POST request");

    // Get or create session
    let session = state
        .sessions
        .get_or_create_session(query.session_id.as_deref())
        .await;

    // Check if this is a notification (no id)
    let is_notification = req.id.is_none();

    let response = match req.method.as_str() {
        "initialize" => {
            let result = handle_initialize(&req.params);

            let mut response =
                Json(JsonRpcResponse::success(req.id, result)).into_response();
            response.headers_mut().insert(
                "Mcp-Session-Id",
                session.id.to_header_value(),
            );
            return response;
        }
        "notifications/initialized" | "initialized" => {
            if is_notification {
                return StatusCode::ACCEPTED.into_response();
            }
            JsonRpcResponse::success(req.id, serde_json::json!({}))
        }
        "tools/list" => handle_tools_list(req.id, &state.tools),
        "tools/call" => {
            // Execute tool asynchronously and push result to session
            let tool_response = handle_tools_call(req.id.clone(), &req.params, &state).await;

            // For tool calls, also push result to session for replay
            if let Some(ref result) = tool_response.result {
                let event_data = serde_json::to_string(&JsonRpcResponse::success(
                    req.id.clone(),
                    result.clone(),
                ))
                .unwrap_or_default();
                session.push_event("message", event_data).await;
            }

            tool_response
        }
        "ping" => JsonRpcResponse::success(req.id, serde_json::json!({})),
        "notifications/cancelled" => {
            if is_notification {
                return StatusCode::ACCEPTED.into_response();
            }
            JsonRpcResponse::success(req.id, serde_json::json!({}))
        }
        _ => {
            if is_notification {
                return StatusCode::ACCEPTED.into_response();
            }
            JsonRpcResponse::error(
                req.id,
                -32601,
                format!("Method not found: {}", req.method),
            )
        }
    };

    let mut res = Json(response).into_response();
    res.headers_mut()
        .insert("Mcp-Session-Id", session.id.to_header_value());
    res
}

/// Handle POST requests to /message (legacy transport)
async fn handle_message_post(
    State(state): State<Arc<HttpState>>,
    Query(query): Query<MessageQuery>,
    Json(req): Json<JsonRpcRequest>,
) -> Response {
    // Delegate to the same handler
    handle_mcp_post(State(state), Query(query), Json(req)).await
}

/// Handle GET requests to /mcp (SSE stream for server-initiated messages)
async fn handle_mcp_get(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    Query(query): Query<SseQuery>,
) -> impl IntoResponse {
    // Parse Last-Event-ID header for replay
    let last_event_id: u64 = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Get or create session
    let session = state
        .sessions
        .get_or_create_session(query.session_id.as_deref())
        .await;

    tracing::info!(
        session_id = %session.id,
        last_event_id = last_event_id,
        "New SSE stream connection"
    );

    // Build stream: replay missed events + live events
    let stream = build_sse_stream(session, last_event_id).await;

    (
        [
            ("X-Accel-Buffering", "no"),
            ("Cache-Control", "no-cache, no-store, must-revalidate"),
        ],
        Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("ping"),
        ),
    )
}

/// Build SSE stream with replay and live events.
async fn build_sse_stream(
    session: Arc<super::session::Session>,
    last_event_id: u64,
) -> impl Stream<Item = Result<Event, Infallible>> {
    // Phase 1: Replay missed events
    let missed_events = session.get_events_after(last_event_id).await;
    let replay_stream = stream::iter(missed_events.into_iter().map(|e| {
        tracing::debug!(event_id = e.id, "Replaying missed event");
        Ok::<_, Infallible>(e.to_sse_event())
    }));

    // Phase 2: Live events from broadcast channel
    let receiver = session.subscribe();
    let live_stream = BroadcastStream::new(receiver).filter_map(
        |result: Result<super::session::BufferedEvent, _>| async move {
            match result {
                Ok(event) => Some(Ok(event.to_sse_event())),
                Err(e) => {
                    tracing::debug!(error = %e, "Broadcast lag, client will catch up");
                    None
                }
            }
        },
    );

    // Combine: replay first, then live
    replay_stream.chain(live_stream)
}

/// Legacy SSE endpoint for old HTTP+SSE transport
async fn handle_sse_legacy(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Parse Last-Event-ID header for replay
    let last_event_id: u64 = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Create new session for legacy connections
    let session = state.sessions.create_session().await;

    tracing::info!(
        session_id = %session.id,
        last_event_id = last_event_id,
        "New legacy SSE connection"
    );

    // Send endpoint event immediately
    let endpoint_url = format!("{}/message?sessionId={}", state.base_url, session.id);
    let endpoint_data = serde_json::json!({
        "endpoint": endpoint_url
    });

    // Push endpoint event to session (for replay)
    session
        .push_event("endpoint", endpoint_data.to_string())
        .await;

    // Build stream
    let stream = build_sse_stream_with_endpoint(session, last_event_id, endpoint_data).await;

    (
        [
            ("X-Accel-Buffering", "no"),
            ("Cache-Control", "no-cache, no-store, must-revalidate"),
            ("Connection", "keep-alive"),
        ],
        Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("ping"),
        ),
    )
}

/// Build SSE stream with initial endpoint event.
async fn build_sse_stream_with_endpoint(
    session: Arc<super::session::Session>,
    last_event_id: u64,
    endpoint_data: serde_json::Value,
) -> impl Stream<Item = Result<Event, Infallible>> {
    // Initial endpoint event (always send first)
    let initial_event = if last_event_id == 0 {
        Some(Ok::<_, Infallible>(
            Event::default()
                .id("0")
                .event("endpoint")
                .data(endpoint_data.to_string()),
        ))
    } else {
        None
    };

    let initial_stream = stream::iter(initial_event.into_iter());

    // Replay missed events (excluding endpoint if already sent)
    let missed_events = session.get_events_after(last_event_id.max(1)).await;
    let replay_stream = stream::iter(missed_events.into_iter().map(|e| {
        tracing::debug!(event_id = e.id, "Replaying missed event");
        Ok::<_, Infallible>(e.to_sse_event())
    }));

    // Live events
    let receiver = session.subscribe();
    let live_stream = BroadcastStream::new(receiver).filter_map(
        |result: Result<super::session::BufferedEvent, _>| async move {
            result.ok().map(|event| Ok(event.to_sse_event()))
        },
    );

    initial_stream.chain(replay_stream).chain(live_stream)
}

fn handle_initialize(params: &serde_json::Value) -> serde_json::Value {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("2024-11-05");

    tracing::info!("MCP initialize: protocol version {}", protocol_version);

    serde_json::json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": "semantic-scholar-mcp",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn handle_tools_list(id: Option<serde_json::Value>, tools: &[Box<dyn McpTool>]) -> JsonRpcResponse {
    let tool_list: Vec<McpToolInfo> = tools
        .iter()
        .map(|t| McpToolInfo {
            name: t.name().to_string(),
            description: t.description().to_string(),
            input_schema: t.input_schema(),
        })
        .collect();

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "tools": tool_list
        }),
    )
}

async fn handle_tools_call(
    id: Option<serde_json::Value>,
    params: &serde_json::Value,
    state: &HttpState,
) -> JsonRpcResponse {
    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => {
            return JsonRpcResponse::error(id, -32602, "Missing 'name' parameter");
        }
    };

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let tool = match state.tools.iter().find(|t| t.name() == tool_name) {
        Some(t) => t,
        None => {
            return JsonRpcResponse::error(id, -32602, format!("Tool not found: {}", tool_name));
        }
    };

    tracing::info!(tool = %tool_name, "Executing tool");

    match tool.execute(&state.ctx, arguments).await {
        Ok(result) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": result
                }]
            }),
        ),
        Err(e) => {
            tracing::error!(tool = %tool_name, error = %e, "Tool execution failed");
            JsonRpcResponse::error(id, -32000, format!("Tool error: {}", e))
        }
    }
}
