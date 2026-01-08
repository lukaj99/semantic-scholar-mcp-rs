//! Stdio transport for MCP protocol.
//!
//! Handles JSON-RPC 2.0 over stdin/stdout.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::tools::{McpTool, ToolContext};

use super::transport::{JsonRpcRequest, JsonRpcResponse, McpToolInfo};

// Re-use types from transport module

/// Handle MCP protocol over stdio.
pub async fn run_stdio(tools: Vec<Box<dyn McpTool>>, ctx: ToolContext) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    tracing::info!("MCP stdio server ready, waiting for requests...");

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            // EOF
            tracing::info!("Stdin closed, shutting down");
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let error_response = JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("Parse error: {}", e),
                );
                let response_json = serde_json::to_string(&error_response)?;
                stdout.write_all(response_json.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                continue;
            }
        };

        tracing::debug!(method = %request.method, "Received request");

        // Handle request
        let response = handle_request(&request, &tools, &ctx).await;

        // Write response
        let response_json = serde_json::to_string(&response)?;
        stdout.write_all(response_json.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(
    req: &JsonRpcRequest,
    tools: &[Box<dyn McpTool>],
    ctx: &ToolContext,
) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => handle_initialize(req.id.clone(), &req.params),
        "initialized" => JsonRpcResponse::success(req.id.clone(), serde_json::json!({})),
        "tools/list" => handle_tools_list(req.id.clone(), tools),
        "tools/call" => handle_tools_call(req.id.clone(), &req.params, tools, ctx).await,
        "ping" => JsonRpcResponse::success(req.id.clone(), serde_json::json!({})),
        _ => JsonRpcResponse::error(
            req.id.clone(),
            -32601,
            format!("Method not found: {}", req.method),
        ),
    }
}

fn handle_initialize(id: Option<serde_json::Value>, params: &serde_json::Value) -> JsonRpcResponse {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("2024-11-05");

    tracing::info!("MCP initialize: protocol version {}", protocol_version);

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "protocolVersion": protocol_version,
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "semantic-scholar-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
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
    tools: &[Box<dyn McpTool>],
    ctx: &ToolContext,
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

    let tool = match tools.iter().find(|t| t.name() == tool_name) {
        Some(t) => t,
        None => {
            return JsonRpcResponse::error(id, -32602, format!("Tool not found: {}", tool_name));
        }
    };

    tracing::info!(tool = %tool_name, "Executing tool");

    match tool.execute(ctx, arguments).await {
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
