// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! JSON-RPC 2.0 batch handler for v2 API.
//!
//! POST /v2/batch accepts an array of JSON-RPC 2.0 requests, executes them
//! concurrently (up to the configured max batch size), and returns an array
//! of JSON-RPC 2.0 responses.

use crate::v2::{
    context::{spawn_blocking, V2Context},
    cursor::Cursor,
    error::{ErrorCode, V2Error},
    types::LedgerMetadata,
};
use aptos_api_types::{AsConverter, MoveModuleBytecode, MoveResource};
use aptos_types::account_address::AccountAddress;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
            id,
        }
    }
}

/// POST /v2/batch
pub async fn batch_handler(
    State(ctx): State<V2Context>,
    Json(requests): Json<Vec<JsonRpcRequest>>,
) -> Result<Json<Vec<JsonRpcResponse>>, V2Error> {
    let max_size = ctx.v2_config.json_rpc_batch_max_size;
    if requests.len() > max_size {
        return Err(V2Error::bad_request(
            ErrorCode::BatchTooLarge,
            format!(
                "Batch size {} exceeds maximum {}",
                requests.len(),
                max_size
            ),
        ));
    }

    if requests.is_empty() {
        return Err(V2Error::bad_request(
            ErrorCode::InvalidInput,
            "Empty batch request",
        ));
    }

    // Validate JSON-RPC version
    for req in &requests {
        if req.jsonrpc != JSONRPC_VERSION {
            return Err(V2Error::bad_request(
                ErrorCode::InvalidInput,
                format!("Unsupported JSON-RPC version: {}", req.jsonrpc),
            ));
        }
    }

    // Execute requests concurrently
    let mut handles = Vec::with_capacity(requests.len());
    for req in requests {
        let ctx = ctx.clone();
        handles.push(tokio::spawn(async move { dispatch_request(&ctx, req).await }));
    }

    let mut responses = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(resp) => responses.push(resp),
            Err(e) => responses.push(JsonRpcResponse::error(
                serde_json::Value::Null,
                -32603,
                format!("Internal error: {}", e),
            )),
        }
    }

    Ok(Json(responses))
}

async fn dispatch_request(ctx: &V2Context, req: JsonRpcRequest) -> JsonRpcResponse {
    let id = req.id.clone();
    match dispatch_inner(ctx, &req).await {
        Ok(result) => JsonRpcResponse::success(id, result),
        Err(e) => {
            let code = match e.http_status().as_u16() {
                400 => -32600,
                404 => -32601,
                _ => -32603,
            };
            JsonRpcResponse::error(id, code, e.message.clone())
        },
    }
}

async fn dispatch_inner(
    ctx: &V2Context,
    req: &JsonRpcRequest,
) -> Result<serde_json::Value, V2Error> {
    match req.method.as_str() {
        "get_ledger_info" => get_ledger_info(ctx).await,
        "get_resources" => get_resources(ctx, &req.params).await,
        "get_resource" => get_resource(ctx, &req.params).await,
        "get_modules" => get_modules(ctx, &req.params).await,
        "get_module" => get_module(ctx, &req.params).await,
        "get_transaction" => get_transaction(ctx, &req.params).await,
        "get_transactions" => get_transactions(ctx, &req.params).await,
        "get_block" => get_block(ctx, &req.params).await,
        _ => Err(V2Error::bad_request(
            ErrorCode::MethodNotFound,
            format!("Unknown method: {}", req.method),
        )),
    }
}

// --- Individual method implementations ---

async fn get_ledger_info(ctx: &V2Context) -> Result<serde_json::Value, V2Error> {
    let ledger_info = ctx.ledger_info()?;
    let metadata = LedgerMetadata::from(&ledger_info);
    serde_json::to_value(&metadata).map_err(|e| V2Error::internal(e))
}

#[derive(Deserialize)]
struct ResourcesParams {
    address: String,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    ledger_version: Option<u64>,
}

async fn get_resources(
    ctx: &V2Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: ResourcesParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&p.address)?;
        let (ledger_info, version, state_view) = ctx.state_view_at(p.ledger_version)?;
        let cursor = p.cursor.as_ref().map(|c| Cursor::decode(c)).transpose()?;

        let (resources, next_cursor) =
            ctx.get_resources_paginated(address, cursor.as_ref(), version)?;

        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

        let move_resources: Vec<MoveResource> = resources
            .into_iter()
            .map(|(tag, bytes)| converter.try_into_resource(&tag, &bytes))
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(V2Error::internal)?;

        let result = PaginatedResult {
            data: move_resources,
            ledger: LedgerMetadata::from(&ledger_info),
            cursor: next_cursor.map(|c| c.encode()),
        };

        serde_json::to_value(&result).map_err(|e| V2Error::internal(e))
    })
    .await
}

#[derive(Deserialize)]
struct SingleResourceParams {
    address: String,
    resource_type: String,
    #[serde(default)]
    ledger_version: Option<u64>,
}

async fn get_resource(
    ctx: &V2Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: SingleResourceParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&p.address)?;
        let tag: move_core_types::language_storage::StructTag =
            p.resource_type.parse().map_err(|e| {
                V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid struct tag: {}", e))
            })?;

        let (_ledger_info, _version, state_view) = ctx.state_view_at(p.ledger_version)?;
        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());
        let api_address: aptos_api_types::Address = address.into();

        let bytes = converter
            .find_resource(&state_view, api_address, &tag)
            .map_err(V2Error::internal)?
            .ok_or_else(|| {
                V2Error::not_found(
                    ErrorCode::ResourceNotFound,
                    format!("Resource {:?} not found", tag),
                )
            })?;

        let resource = converter
            .try_into_resource(&tag, &bytes)
            .map_err(V2Error::internal)?;

        serde_json::to_value(&resource).map_err(|e| V2Error::internal(e))
    })
    .await
}

#[derive(Deserialize)]
struct ModulesParams {
    address: String,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    ledger_version: Option<u64>,
}

async fn get_modules(
    ctx: &V2Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: ModulesParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&p.address)?;
        let (ledger_info, version, _state_view) = ctx.state_view_at(p.ledger_version)?;
        let cursor = p.cursor.as_ref().map(|c| Cursor::decode(c)).transpose()?;

        let (modules, next_cursor) =
            ctx.get_modules_paginated(address, cursor.as_ref(), version)?;

        let mut move_modules = Vec::with_capacity(modules.len());
        for (_id, bytes) in modules {
            let m = MoveModuleBytecode::new(bytes)
                .try_parse_abi()
                .map_err(|e| V2Error::internal(e))?;
            move_modules.push(m);
        }

        let result = PaginatedResult {
            data: move_modules,
            ledger: LedgerMetadata::from(&ledger_info),
            cursor: next_cursor.map(|c| c.encode()),
        };

        serde_json::to_value(&result).map_err(|e| V2Error::internal(e))
    })
    .await
}

#[derive(Deserialize)]
struct SingleModuleParams {
    address: String,
    module_name: String,
    #[serde(default)]
    ledger_version: Option<u64>,
}

async fn get_module(
    ctx: &V2Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: SingleModuleParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&p.address)?;
        let (_ledger_info, _version, state_view) = ctx.state_view_at(p.ledger_version)?;

        let module_id = move_core_types::language_storage::ModuleId::new(
            address,
            move_core_types::identifier::Identifier::new(p.module_name.clone()).map_err(|e| {
                V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid module name: {}", e))
            })?,
        );

        let state_key = aptos_types::state_store::state_key::StateKey::module_id(&module_id);

        use aptos_types::state_store::TStateView;
        let module_bytes = state_view
            .get_state_value_bytes(&state_key)
            .map_err(V2Error::internal)?
            .ok_or_else(|| {
                V2Error::not_found(
                    ErrorCode::ModuleNotFound,
                    format!("Module {} not found", p.module_name),
                )
            })?;

        let module = MoveModuleBytecode::new(module_bytes.to_vec())
            .try_parse_abi()
            .map_err(|e| V2Error::internal(e))?;

        serde_json::to_value(&module).map_err(|e| V2Error::internal(e))
    })
    .await
}

#[derive(Deserialize)]
struct TransactionParams {
    hash: String,
}

async fn get_transaction(
    ctx: &V2Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: TransactionParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let hash = parse_hash_str(&p.hash)?;

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let ledger_info = ctx.ledger_info()?;
        let version = ledger_info.version();

        let txn_data = ctx
            .inner()
            .get_transaction_by_hash(hash, version)
            .map_err(V2Error::internal)?
            .ok_or_else(|| {
                V2Error::not_found(
                    ErrorCode::TransactionNotFound,
                    format!("Transaction {} not found", hash),
                )
            })?;

        let timestamp = ctx
            .inner()
            .db
            .get_block_timestamp(txn_data.version)
            .map_err(V2Error::internal)?;
        let state_view = ctx.inner().latest_state_view().map_err(V2Error::internal)?;
        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());
        let txn = converter
            .try_into_onchain_transaction(timestamp, txn_data)
            .map_err(V2Error::internal)?;

        serde_json::to_value(&txn).map_err(|e| V2Error::internal(e))
    })
    .await
}

#[derive(Deserialize)]
struct TransactionsListParams {
    #[serde(default)]
    cursor: Option<String>,
}

async fn get_transactions(
    ctx: &V2Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: TransactionsListParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let ledger_info = ctx.ledger_info()?;
        let ledger_version = ledger_info.version();
        let cursor = p.cursor.as_ref().map(|c| Cursor::decode(c)).transpose()?;

        let (txns, next_cursor) =
            ctx.get_transactions_paginated(cursor.as_ref(), ledger_version)?;

        let state_view = ctx.inner().latest_state_view().map_err(V2Error::internal)?;
        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

        let rendered: Vec<aptos_api_types::Transaction> = txns
            .into_iter()
            .map(|txn_data| {
                let timestamp = ctx
                    .inner()
                    .db
                    .get_block_timestamp(txn_data.version)
                    .map_err(V2Error::internal)?;
                converter
                    .try_into_onchain_transaction(timestamp, txn_data)
                    .map_err(V2Error::internal)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let result = PaginatedResult {
            data: rendered,
            ledger: LedgerMetadata::from(&ledger_info),
            cursor: next_cursor.map(|c| c.encode()),
        };

        serde_json::to_value(&result).map_err(|e| V2Error::internal(e))
    })
    .await
}

#[derive(Deserialize)]
struct BlockParams {
    height: u64,
    #[serde(default)]
    with_transactions: Option<bool>,
}

async fn get_block(
    ctx: &V2Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: BlockParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let with_txns = p.with_transactions.unwrap_or(false);
        let (bcs_block, ledger_info) = ctx.get_block_by_height(p.height, with_txns)?;

        let block_height: u64 = ledger_info.block_height.into();
        let block = aptos_api_types::Block {
            block_height: block_height.into(),
            block_hash: bcs_block.block_hash.into(),
            block_timestamp: bcs_block.block_timestamp.into(),
            first_version: bcs_block.first_version.into(),
            last_version: bcs_block.last_version.into(),
            transactions: None, // Simplified for batch
        };

        serde_json::to_value(&block).map_err(|e| V2Error::internal(e))
    })
    .await
}

// --- Shared types ---

#[derive(Serialize)]
struct PaginatedResult<T: Serialize> {
    data: T,
    ledger: LedgerMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
}

// --- Helpers ---

fn parse_address(s: &str) -> Result<AccountAddress, V2Error> {
    AccountAddress::from_hex_literal(s)
        .or_else(|_| AccountAddress::from_hex(s))
        .map_err(|e| {
            V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid address: {}", e))
        })
}

fn parse_hash_str(s: &str) -> Result<aptos_crypto::HashValue, V2Error> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    aptos_crypto::HashValue::from_hex(s).map_err(|e| {
        V2Error::bad_request(
            ErrorCode::InvalidInput,
            format!("Invalid hash: {}", e),
        )
    })
}
