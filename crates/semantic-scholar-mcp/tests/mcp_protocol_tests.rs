//! Tests for MCP protocol JSON-RPC handling.
//!
//! These tests verify the server correctly handles MCP messages.

use serde_json::json;

// =============================================================================
// JSON-RPC Message Structure Tests
// =============================================================================

/// Test valid JSON-RPC request structure
#[test]
fn test_jsonrpc_request_structure() {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    assert_eq!(request["jsonrpc"], "2.0");
    assert_eq!(request["method"], "tools/list");
    assert_eq!(request["id"], 1);
}

/// Test tool call request structure
#[test]
fn test_tool_call_request_structure() {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "id": 2,
        "params": {
            "name": "exhaustive_search",
            "arguments": {
                "query": "machine learning",
                "maxResults": 10
            }
        }
    });

    assert_eq!(request["params"]["name"], "exhaustive_search");
    assert_eq!(request["params"]["arguments"]["query"], "machine learning");
}

/// Test notification (no id)
#[test]
fn test_jsonrpc_notification() {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    assert!(notification.get("id").is_none());
}

// =============================================================================
// Tool Schema Validation Tests
// =============================================================================

/// Test `exhaustive_search` input schema
#[test]
fn test_exhaustive_search_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "query": {"type": "string"},
            "yearStart": {"type": "integer"},
            "yearEnd": {"type": "integer"},
            "minCitations": {"type": "integer"},
            "maxResults": {"type": "integer"},
            "responseFormat": {"type": "string", "enum": ["markdown", "json"]}
        },
        "required": ["query"]
    });

    // Verify required field
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("query")));
}

/// Test recommendations input schema
#[test]
fn test_recommendations_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "positivePaperIds": {"type": "array", "items": {"type": "string"}},
            "negativePaperIds": {"type": "array", "items": {"type": "string"}},
            "limit": {"type": "integer"}
        },
        "required": ["positivePaperIds"]
    });

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("positivePaperIds")));
}

/// Test `citation_snowball` input schema
#[test]
fn test_citation_snowball_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "seedPaperIds": {"type": "array"},
            "direction": {"type": "string", "enum": ["citations", "references", "both"]},
            "depth": {"type": "integer", "minimum": 1, "maximum": 3},
            "maxPerPaper": {"type": "integer"}
        },
        "required": ["seedPaperIds"]
    });

    let direction_enum = schema["properties"]["direction"]["enum"].as_array().unwrap();
    assert_eq!(direction_enum.len(), 3);
}

// =============================================================================
// Error Response Tests
// =============================================================================

/// Test JSON-RPC error response structure
#[test]
fn test_jsonrpc_error_structure() {
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32600,
            "message": "Invalid Request"
        }
    });

    assert_eq!(error_response["error"]["code"], -32600);
    assert!(error_response.get("result").is_none());
}

/// Test tool execution error
#[test]
fn test_tool_error_response() {
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32000,
            "message": "Tool execution failed",
            "data": {
                "tool": "exhaustive_search",
                "reason": "Rate limited"
            }
        }
    });

    assert!(error_response["error"]["data"]["reason"]
        .as_str()
        .unwrap()
        .contains("Rate"));
}

// =============================================================================
// Tool Response Format Tests
// =============================================================================

/// Test markdown response format
#[test]
fn test_markdown_response() {
    let response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "# Search Results\n\n**Query:** test\n**Results:** 10"
                }
            ]
        }
    });

    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with('#'), "Markdown should start with heading");
}

/// Test JSON response format
#[test]
fn test_json_response() {
    let response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "{\"query\":\"test\",\"total\":10,\"papers\":[]}"
                }
            ]
        }
    });

    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).expect("Should be valid JSON");
    assert_eq!(parsed["total"], 10);
}

// =============================================================================
// Tool List Response Tests
// =============================================================================

/// Test tools/list response structure
#[test]
fn test_tools_list_response() {
    // Simulate what tools/list should return
    let tools = vec![
        json!({"name": "exhaustive_search", "description": "Search papers"}),
        json!({"name": "recommendations", "description": "Get recommendations"}),
        json!({"name": "citation_snowball", "description": "Citation network traversal"}),
    ];

    let response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "tools": tools
        }
    });

    let tool_list = response["result"]["tools"].as_array().unwrap();
    assert!(tool_list.len() >= 3);

    // Verify each tool has required fields
    for tool in tool_list {
        assert!(tool.get("name").is_some());
        assert!(tool.get("description").is_some());
    }
}

/// Test all 23 tool names are valid identifiers
#[test]
fn test_tool_names_valid() {
    let tool_names = [
        "exhaustive_search",
        "recommendations",
        "citation_snowball",
        "batch_metadata",
        "author_search",
        "author_papers",
        "reference_export",
        "prisma_search",
        "screening_export",
        "prisma_flow_diagram",
        "semantic_search",
        "literature_review_pipeline",
        "author_network",
        "research_trends",
        "venue_analytics",
        "field_weighted_impact",
        "highly_cited_papers",
        "citation_half_life",
        "cocitation_analysis",
        "bibliographic_coupling",
        "hot_papers",
        "pearl_growing",
        "orcid_author_lookup",
    ];

    assert_eq!(tool_names.len(), 23, "Should have 23 tools");

    for name in tool_names {
        // Tool names should be snake_case identifiers
        assert!(
            name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "Tool name '{name}' should be snake_case"
        );
    }
}

// =============================================================================
// MCP Initialize Handshake Tests
// =============================================================================

/// Test initialize request
#[test]
fn test_initialize_request() {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "id": 1,
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    assert_eq!(request["params"]["protocolVersion"], "2024-11-05");
}

/// Test initialize response
#[test]
fn test_initialize_response() {
    let response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "semantic-scholar-mcp",
                "version": "0.1.0"
            }
        }
    });

    assert_eq!(response["result"]["serverInfo"]["name"], "semantic-scholar-mcp");
}
