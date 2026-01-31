// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! MCP server with cached model - build once, query many times

use crate::engine::{QueryEngine, QueryOptions};
use crate::types::{ItemType, SearchLevel};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// MCP Protocol Version
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server information
const SERVER_NAME: &str = "move-query";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// JSON-RPC request
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

/// JSON-RPC response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}

/// Server state holding the cached model
struct ServerState {
    engine: Option<QueryEngine>,
    package_path: Option<PathBuf>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            engine: None,
            package_path: None,
        }
    }

    fn build_model(&mut self, arguments: &Value) -> Result<Value> {
        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;

        // Parse named addresses from JSON object
        let named_addresses = arguments
            .get("named_addresses")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        v.as_str().and_then(|s| {
                            let s = s.trim_start_matches("0x");
                            aptos_types::account_address::AccountAddress::from_hex(s)
                                .ok()
                                .map(|addr| (k.clone(), addr))
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let options = QueryOptions {
            named_addresses,
            dev: arguments
                .get("dev")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            bytecode_version: arguments
                .get("bytecode_version")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            language_version: arguments
                .get("language_version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            experiments: arguments
                .get("experiments")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        };

        let path = PathBuf::from(path);
        let engine = QueryEngine::new(path.clone(), options)?;
        let package_info = engine.get_package()?;

        self.engine = Some(engine);
        self.package_path = Some(path);

        Ok(serde_json::to_value(package_info)?)
    }

    fn get_engine(&self) -> Result<&QueryEngine> {
        self.engine
            .as_ref()
            .ok_or_else(|| anyhow!("No model loaded. Call 'build_model' first with a package path."))
    }
}

/// List all available tools
fn list_tools() -> Vec<Value> {
    vec![
        // Build command
        json!({
            "name": "build_model",
            "description": "Build the Move model from a package directory. Must be called before other queries.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the Move package directory"
                    },
                    "named_addresses": {
                        "type": "object",
                        "description": "Named address mapping, e.g., {\"alice\": \"0x1234\", \"bob\": \"0x5678\"}"
                    },
                    "dev": {
                        "type": "boolean",
                        "description": "Development mode - uses dev-addresses and dev-dependencies"
                    },
                    "bytecode_version": {
                        "type": "integer",
                        "description": "Target bytecode version"
                    },
                    "language_version": {
                        "type": "string",
                        "description": "Move language version (e.g., \"1\", \"2\", \"2.1\", \"2.2\")"
                    },
                    "experiments": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Compiler experiments (e.g., [\"optimize=off\", \"checks=off\"])"
                    }
                },
                "required": ["path"]
            }
        }),
        // Hierarchical queries
        json!({
            "name": "get_package",
            "description": "Get package information with module summaries",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "get_module",
            "description": "Get module information with function/struct/constant summaries",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Module name (e.g., 'coin' or '0x1::coin')"
                    }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "get_function",
            "description": "Get full function details including parameters, return type, and specifications",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "module": {
                        "type": "string",
                        "description": "Module name"
                    },
                    "name": {
                        "type": "string",
                        "description": "Function name"
                    }
                },
                "required": ["module", "name"]
            }
        }),
        json!({
            "name": "get_struct",
            "description": "Get full struct details including fields, abilities, and type parameters",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "module": {
                        "type": "string",
                        "description": "Module name"
                    },
                    "name": {
                        "type": "string",
                        "description": "Struct name"
                    }
                },
                "required": ["module", "name"]
            }
        }),
        json!({
            "name": "get_constant",
            "description": "Get full constant details including type and value",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "module": {
                        "type": "string",
                        "description": "Module name"
                    },
                    "name": {
                        "type": "string",
                        "description": "Constant name"
                    }
                },
                "required": ["module", "name"]
            }
        }),
        // Source code query
        json!({
            "name": "get_source",
            "description": "Get source code for a module, function, struct, or constant",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "module": {
                        "type": "string",
                        "description": "Module name"
                    },
                    "name": {
                        "type": "string",
                        "description": "Item name (function, struct, or constant). Omit for entire module."
                    },
                    "item_type": {
                        "type": "string",
                        "enum": ["module", "function", "struct", "constant"],
                        "description": "Type of item. Auto-detected if omitted."
                    }
                },
                "required": ["module"]
            }
        }),
        // Cross-cutting queries
        json!({
            "name": "list_entry_points",
            "description": "List all entry point functions in the package",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "list_resources",
            "description": "List all resource types (structs with key ability)",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "search",
            "description": "Search for modules, functions, structs, or constants by pattern",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (regex supported)"
                    },
                    "level": {
                        "type": "string",
                        "enum": ["module", "function", "struct", "constant"],
                        "description": "Limit search to specific level"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 50)"
                    }
                },
                "required": ["pattern"]
            }
        }),
        // Relationship queries
        json!({
            "name": "get_call_graph",
            "description": "Get function call graph showing calls and callers",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "function": {
                        "type": "string",
                        "description": "Function path in module::function format"
                    }
                },
                "required": ["function"]
            }
        }),
        json!({
            "name": "get_deps",
            "description": "Get module dependencies (uses, used_by, friends)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "module": {
                        "type": "string",
                        "description": "Module name"
                    }
                },
                "required": ["module"]
            }
        }),
    ]
}

/// Call a tool by name
fn call_tool(state: &mut ServerState, name: &str, arguments: &Value) -> Result<Value> {
    match name {
        "build_model" => state.build_model(arguments),
        "get_package" => {
            let engine = state.get_engine()?;
            let result = engine.get_package()?;
            Ok(serde_json::to_value(result)?)
        }
        "get_module" => {
            let engine = state.get_engine()?;
            let module = arguments
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'name' argument"))?;
            let result = engine.get_module(module)?;
            Ok(serde_json::to_value(result)?)
        }
        "get_function" => {
            let engine = state.get_engine()?;
            let module = arguments
                .get("module")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'module' argument"))?;
            let name = arguments
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'name' argument"))?;
            let result = engine.get_function(module, name)?;
            Ok(serde_json::to_value(result)?)
        }
        "get_struct" => {
            let engine = state.get_engine()?;
            let module = arguments
                .get("module")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'module' argument"))?;
            let name = arguments
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'name' argument"))?;
            let result = engine.get_struct(module, name)?;
            Ok(serde_json::to_value(result)?)
        }
        "get_constant" => {
            let engine = state.get_engine()?;
            let module = arguments
                .get("module")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'module' argument"))?;
            let name = arguments
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'name' argument"))?;
            let result = engine.get_constant(module, name)?;
            Ok(serde_json::to_value(result)?)
        }
        "get_source" => {
            let engine = state.get_engine()?;
            let module = arguments
                .get("module")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'module' argument"))?;
            let name = arguments.get("name").and_then(|v| v.as_str());
            let item_type = arguments
                .get("item_type")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "module" => ItemType::Module,
                    "function" => ItemType::Function,
                    "struct" => ItemType::Struct,
                    "constant" => ItemType::Constant,
                    _ => ItemType::Function,
                });
            let result = engine.get_source(module, name, item_type)?;
            Ok(serde_json::to_value(result)?)
        }
        "list_entry_points" => {
            let engine = state.get_engine()?;
            let result = engine.list_entry_points();
            Ok(serde_json::to_value(result)?)
        }
        "list_resources" => {
            let engine = state.get_engine()?;
            let result = engine.list_resources();
            Ok(serde_json::to_value(result)?)
        }
        "search" => {
            let engine = state.get_engine()?;
            let pattern = arguments
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'pattern' argument"))?;
            let level = arguments.get("level").and_then(|v| v.as_str()).map(|s| {
                match s {
                    "module" => SearchLevel::Module,
                    "function" => SearchLevel::Function,
                    "struct" => SearchLevel::Struct,
                    "constant" => SearchLevel::Constant,
                    _ => SearchLevel::Function,
                }
            });
            let limit = arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(50);
            let result = engine.search(pattern, level, limit)?;
            Ok(serde_json::to_value(result)?)
        }
        "get_call_graph" => {
            let engine = state.get_engine()?;
            let function = arguments
                .get("function")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'function' argument"))?;
            let result = engine.get_call_graph(function)?;
            Ok(serde_json::to_value(result)?)
        }
        "get_deps" => {
            let engine = state.get_engine()?;
            let module = arguments
                .get("module")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'module' argument"))?;
            let result = engine.get_deps(module)?;
            Ok(serde_json::to_value(result)?)
        }
        _ => Err(anyhow!("Unknown tool: {}", name)),
    }
}

/// Handle a JSON-RPC request
fn handle_request(state: &mut ServerState, request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            }),
        ),
        "initialized" => JsonRpcResponse::success(id, json!({})),
        "tools/list" => {
            let tools = list_tools();
            JsonRpcResponse::success(id, json!({ "tools": tools }))
        }
        "tools/call" => {
            let name = request
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));

            match call_tool(state, name, &arguments) {
                Ok(result) => JsonRpcResponse::success(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                        }]
                    }),
                ),
                Err(e) => JsonRpcResponse::success(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Error: {}", e)
                        }],
                        "isError": true
                    }),
                ),
            }
        }
        "ping" => JsonRpcResponse::success(id, json!({})),
        _ => JsonRpcResponse::error(id, -32601, format!("Method not found: {}", request.method)),
    }
}

/// Run the MCP server with stdio transport
pub async fn run_server() -> Result<()> {
    let mut state = ServerState::new();

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    eprintln!("Move Query MCP server started (no model loaded).");
    eprintln!("Use 'build_model' tool to load a Move package.");

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let response =
                    JsonRpcResponse::error(Value::Null, -32700, format!("Parse error: {}", e));
                let output = serde_json::to_string(&response)?;
                writeln!(stdout, "{}", output)?;
                stdout.flush()?;
                continue;
            }
        };

        // Handle request
        let response = handle_request(&mut state, &request);

        // Send response (only if not a notification)
        if request.id.is_some() {
            let output = serde_json::to_string(&response)?;
            writeln!(stdout, "{}", output)?;
            stdout.flush()?;
        }
    }

    Ok(())
}
