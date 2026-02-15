use super::types::*;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};

/// Run the MCP server
pub fn run() -> anyhow::Result<()> {
    eprintln!("Starting flat MCP server...");

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = line?;

        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => {
                let response = handle_request(request);
                let json = serde_json::to_string(&response)?;
                writeln!(stdout, "{}", json)?;
                stdout.flush()?;
            }
            Err(e) => {
                eprintln!("Invalid JSON-RPC request: {}", e);
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: "Parse error".to_string(),
                        data: None,
                    }),
                };
                let json = serde_json::to_string(&error_response)?;
                writeln!(stdout, "{}", json)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}

/// Handle a JSON-RPC request
fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => handle_initialize(id),
        "tools/list" => handle_tools_list(id),
        "tools/call" => handle_tool_call(req, id),
        _ => error_response(id, -32601, "Method not found"),
    }
}

/// Handle initialize request
fn handle_initialize(id: Option<Value>) -> JsonRpcResponse {
    let capabilities = ServerCapabilities {
        tools: ToolsCapability {
            list_changed: false,
        },
    };

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": capabilities,
            "serverInfo": {
                "name": "flat",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        error: None,
    }
}

/// Handle tools/list request
fn handle_tools_list(id: Option<Value>) -> JsonRpcResponse {
    let tools = vec![
        json!({
            "name": "analyze_repo",
            "description": "Analyze and flatten a repository",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to repository"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["xml", "json"],
                        "description": "Output format"
                    },
                    "compress": {
                        "type": "boolean",
                        "description": "Compress source code"
                    }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "list_files",
            "description": "List files in a repository",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to repository"
                    }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "get_statistics",
            "description": "Get statistics about a repository",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to repository"
                    }
                },
                "required": ["path"]
            }
        }),
    ];

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(json!({ "tools": tools })),
        error: None,
    }
}

/// Handle tools/call request
fn handle_tool_call(req: JsonRpcRequest, id: Option<Value>) -> JsonRpcResponse {
    let params = match req.params {
        Some(Value::Object(ref obj)) => obj.clone(),
        _ => {
            return error_response(id, -32602, "Invalid params");
        }
    };

    let tool_name = params.get("name").and_then(|v| v.as_str());

    match tool_name {
        Some("analyze_repo") => {
            let path = match params.get("_meta").and_then(|v| v.get("arguments")) {
                Some(Value::Object(args)) => args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("."),
                _ => ".",
            };

            let result = ToolResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: Some(format!("Repository analysis for: {}", path)),
                }],
                is_error: false,
            };

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!(result)),
                error: None,
            }
        }
        _ => error_response(id, -32602, "Unknown tool"),
    }
}

/// Create an error response
fn error_response(id: Option<Value>, code: i32, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        }),
    }
}
