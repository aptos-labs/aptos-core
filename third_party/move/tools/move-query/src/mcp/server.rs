// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! MCP server for Move package introspection.
//!
//! Implements JSON-RPC 2.0 over stdio per the [MCP specification](https://modelcontextprotocol.io/specification).
//!
//! # Example
//!
//! ```text
//! # Start the server and initialize
//! $ aptos move query serve
//! Move Query MCP server started.
//! send: {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
//! recv: {"jsonrpc":"2.0","id":1,"result":{...}}
//!
//! # List available tools
//! send: {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
//! recv: {"jsonrpc":"2.0","id":2,"result":{"tools":[...]}}
//!
//! # Build the Move model (required before queries)
//! # Use test_mode:true to include #[test] and #[test_only] code
//! send: {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"build_model","arguments":{"path":"./my-package","test_mode":true}}}
//! recv: {"jsonrpc":"2.0","id":3,"result":{...}}
//!
//! # Query a module
//! send: {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_module","arguments":{"name":"0x1::coin"}}}
//! recv: {"jsonrpc":"2.0","id":4,"result":{...}}
//!
//! # Query a function
//! send: {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_function","arguments":{"name":"0x1::coin::transfer"}}}
//! recv: {"jsonrpc":"2.0","id":5,"result":{...}}
//!
//! # Rebuild the model after source changes
//! send: {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"rebuild_model","arguments":{}}}
//! recv: {"jsonrpc":"2.0","id":6,"result":{...}}
//!
//! # Shutdown the server
//! send: {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"shutdown","arguments":{}}}
//! recv: {"jsonrpc":"2.0","id":7,"result":{...}}
//! Shutting down.
//! ```

use crate::{engine::QueryEngine, types::Location};
use anyhow::{anyhow, Result};
use move_command_line_common::address::NumericalAddress;
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::{BuildConfig, CompilerConfig, ModelConfig};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    str::FromStr,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

fn default_true() -> bool {
    true
}

const PROTOCOL_VERSION: &str = "2025-11-25";

const SERVER_NAME: &str = "move-query";
const SERVER_VERSION: &str = "0.1.0";

// JSON-RPC 2.0 error codes
const JSONRPC_PARSE_ERROR: i32 = -32700;
const JSONRPC_METHOD_NOT_FOUND: i32 = -32601;
const JSONRPC_INVALID_PARAMS: i32 = -32602;

// ========================================
// JSON-RPC Types
// ========================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
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
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// ========================================
// Server State
// ========================================

struct ServerState {
    engine: Option<QueryEngine>,
    should_shutdown: bool,
}

impl ServerState {
    fn new() -> Self {
        Self {
            engine: None,
            should_shutdown: false,
        }
    }

    fn shutdown(&mut self) {
        self.should_shutdown = true;
    }

    fn should_shutdown(&self) -> bool {
        self.should_shutdown
    }

    fn set_engine(&mut self, engine: QueryEngine) {
        self.engine = Some(engine);
    }

    fn get_engine(&self) -> Result<&QueryEngine> {
        self.engine
            .as_ref()
            .ok_or_else(|| anyhow!("No model loaded. Call `build_model` first."))
    }

    fn get_engine_mut(&mut self) -> Result<&mut QueryEngine> {
        self.engine
            .as_mut()
            .ok_or_else(|| anyhow!("No model loaded. Call `build_model` first."))
    }
}

// ========================================
// Tool Definitions
// ========================================

fn json_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).unwrap()
}

// ----------------------------------------
// Tool Args
// ----------------------------------------

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct NoArgs {}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct BuildRequest {
    /// Path to Move package
    path: String,
    /// Address mapping
    #[serde(default)]
    named_addresses: HashMap<String, String>,
    /// Dev mode
    #[serde(default)]
    dev: bool,
    /// Test mode (include #[test] and #[test_only] code)
    #[serde(default)]
    test_mode: bool,
    /// Verify mode (include #[verify_only] code)
    #[serde(default = "default_true")]
    verify_mode: bool,
    /// Module filter (only compile modules matching this string)
    target_filter: Option<String>,
    /// Bytecode version
    bytecode_version: Option<u32>,
    /// Compiler version (e.g., "2.0", "2.1")
    compiler_version: Option<String>,
    /// Language version (e.g., "2.0", "2.1")
    language_version: Option<String>,
    /// Skip attribute checks
    #[serde(default)]
    skip_attribute_checks: bool,
    /// Additional known attributes
    #[serde(default)]
    known_attributes: Vec<String>,
    /// Compiler experiments to enable
    #[serde(default)]
    experiments: Vec<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct FullName {
    /// Full name (e.g., "0x1::module" or "0x1::module::item")
    name: String,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct SourceSpan {
    /// File path
    file: String,
    /// Start line (1-indexed)
    start_line: u32,
    /// Start column (1-indexed, optional, defaults to 1)
    #[serde(default = "default_column")]
    start_column: u32,
    /// End line (1-indexed)
    end_line: u32,
    /// End column (1-indexed, optional, defaults to end of line)
    #[serde(default)]
    end_column: u32,
}

fn default_column() -> u32 {
    1
}

// ----------------------------------------
// Tool Enum
// ----------------------------------------

/// MCP tools. Adding a variant requires updating all match arms below.
#[derive(Debug, Deserialize, EnumIter)]
#[serde(tag = "name", content = "arguments", rename_all = "snake_case")]
enum Tool {
    BuildModel(BuildRequest),
    RebuildModel(NoArgs),
    GetPackage(NoArgs),
    GetModule(FullName),
    GetFunction(FullName),
    GetStruct(FullName),
    GetConstant(FullName),
    GetSource(SourceSpan),
    Shutdown(NoArgs),
}

impl Tool {
    fn call(self, state: &mut ServerState) -> Result<Value> {
        match self {
            Tool::BuildModel(req) => {
                let named_addresses = req
                    .named_addresses
                    .into_iter()
                    .map(|(k, v)| {
                        NumericalAddress::parse_str(v.trim())
                            .map(|addr| (k.clone(), addr.into_inner()))
                            .map_err(|_| anyhow!("Invalid address format for '{}': {}", k, v))
                    })
                    .collect::<Result<_>>()?;

                let compiler_version = req
                    .compiler_version
                    .as_ref()
                    .map(|s| {
                        CompilerVersion::from_str(s).map_err(|_| {
                            anyhow!("Invalid compiler_version '{}': expected '2.0' or '2.1'", s)
                        })
                    })
                    .transpose()?;
                let language_version = req
                    .language_version
                    .as_ref()
                    .map(|s| {
                        LanguageVersion::from_str(s).map_err(|_| {
                            anyhow!(
                                "Invalid language_version '{}': expected '1.0', '2.0', or '2.1'",
                                s
                            )
                        })
                    })
                    .transpose()?;
                let bytecode_version = Some(
                    language_version
                        .unwrap_or_default()
                        .infer_bytecode_version(req.bytecode_version),
                );

                let build_config = BuildConfig {
                    dev_mode: req.dev,
                    test_mode: req.test_mode,
                    verify_mode: req.verify_mode,
                    additional_named_addresses: named_addresses,
                    skip_fetch_latest_git_deps: true,
                    compiler_config: CompilerConfig {
                        bytecode_version,
                        compiler_version,
                        language_version,
                        skip_attribute_checks: req.skip_attribute_checks,
                        known_attributes: req.known_attributes.into_iter().collect(),
                        experiments: req.experiments,
                        ..CompilerConfig::default()
                    },
                    ..BuildConfig::default()
                };

                let model_config = ModelConfig {
                    target_filter: req.target_filter,
                    all_files_as_targets: false,
                    compiler_version: compiler_version.unwrap_or_default(),
                    language_version: language_version.unwrap_or_default(),
                };

                let engine = QueryEngine::new(PathBuf::from(req.path), build_config, model_config)?;
                let package = engine.get_package()?;
                state.set_engine(engine);
                Ok(serde_json::to_value(package)?)
            },
            Tool::RebuildModel(_) => {
                state.get_engine_mut()?.rebuild()?;
                Ok(serde_json::to_value(state.get_engine()?.get_package()?)?)
            },
            Tool::GetPackage(_) => Ok(serde_json::to_value(state.get_engine()?.get_package()?)?),
            Tool::GetModule(arg) => Ok(serde_json::to_value(
                state.get_engine()?.get_module(&arg.name)?,
            )?),
            Tool::GetFunction(arg) => Ok(serde_json::to_value(
                state.get_engine()?.get_function(&arg.name)?,
            )?),
            Tool::GetStruct(arg) => Ok(serde_json::to_value(
                state.get_engine()?.get_struct(&arg.name)?,
            )?),
            Tool::GetConstant(arg) => Ok(serde_json::to_value(
                state.get_engine()?.get_constant(&arg.name)?,
            )?),
            Tool::GetSource(args) => {
                let loc = Location {
                    file: args.file,
                    start_line: args.start_line,
                    start_column: args.start_column,
                    end_line: args.end_line,
                    end_column: args.end_column,
                };
                Ok(serde_json::to_value(state.get_engine()?.get_source(&loc)?)?)
            },
            Tool::Shutdown(_) => {
                state.shutdown();
                Ok(json!({"message": "Server shutting down"}))
            },
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Tool::BuildModel(_) => "build_model",
            Tool::RebuildModel(_) => "rebuild_model",
            Tool::GetPackage(_) => "get_package",
            Tool::GetModule(_) => "get_module",
            Tool::GetFunction(_) => "get_function",
            Tool::GetStruct(_) => "get_struct",
            Tool::GetConstant(_) => "get_constant",
            Tool::GetSource(_) => "get_source",
            Tool::Shutdown(_) => "shutdown",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Tool::BuildModel(_) => "Build Move model from package directory (required first).",
            Tool::RebuildModel(_) => "Rebuild Move model after source changes.",
            Tool::GetPackage(_) => "Get package with module list.",
            Tool::GetModule(_) => "Get module with function/struct/constant names.",
            Tool::GetFunction(_) => "Get function details.",
            Tool::GetStruct(_) => "Get struct/enum details.",
            Tool::GetConstant(_) => "Get constant details.",
            Tool::GetSource(_) => "Get source code for a location (returns full lines).",
            Tool::Shutdown(_) => "Shut down the server.",
        }
    }

    fn input_schema(&self) -> Value {
        match self {
            Tool::BuildModel(_) => json_schema::<BuildRequest>(),
            Tool::RebuildModel(_) | Tool::GetPackage(_) | Tool::Shutdown(_) => {
                json_schema::<NoArgs>()
            },
            Tool::GetModule(_)
            | Tool::GetFunction(_)
            | Tool::GetStruct(_)
            | Tool::GetConstant(_) => json_schema::<FullName>(),
            Tool::GetSource(_) => json_schema::<SourceSpan>(),
        }
    }

    fn schema(&self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "inputSchema": self.input_schema()
        })
    }
}

fn list_tools() -> Vec<Value> {
    Tool::iter().map(|t| t.schema()).collect()
}

// ========================================
// Request Handler
// ========================================

fn handle_request(state: &mut ServerState, request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
            }),
        ),
        "notifications/initialized" => JsonRpcResponse::success(id, json!({})),
        "tools/list" => JsonRpcResponse::success(id, json!({ "tools": list_tools() })),
        "tools/call" => {
            let tool: Tool = match serde_json::from_value(request.params.clone()) {
                Ok(t) => t,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        JSONRPC_INVALID_PARAMS,
                        format!("Invalid tool call: {}", e),
                    );
                },
            };

            match tool.call(state) {
                Ok(value) => JsonRpcResponse::success(
                    id,
                    json!({
                        "content": [{"type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())}],
                        "isError": false
                    }),
                ),
                Err(e) => JsonRpcResponse::success(
                    id,
                    json!({
                        "content": [{"type": "text", "text": format!("Error: {}", e)}],
                        "isError": true
                    }),
                ),
            }
        },
        "ping" => JsonRpcResponse::success(id, json!({})),
        _ => JsonRpcResponse::error(
            id,
            JSONRPC_METHOD_NOT_FOUND,
            format!("Method not found: {}", request.method),
        ),
    }
}

// ========================================
// Server Entry Point
// ========================================

/// Run the MCP server over stdio. See module documentation for usage.
pub fn run_server() -> Result<()> {
    let mut state = ServerState::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    eprintln!("Move Query MCP server started.");

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let response = JsonRpcResponse::error(
                    Value::Null,
                    JSONRPC_PARSE_ERROR,
                    format!("Parse error: {}", e),
                );
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                continue;
            },
        };

        let response = handle_request(&mut state, &request);

        if request.id.is_some() {
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }

        if state.should_shutdown() {
            eprintln!("Shutting down.");
            break;
        }
    }

    Ok(())
}
