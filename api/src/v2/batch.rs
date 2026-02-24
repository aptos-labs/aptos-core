// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! JSON-RPC 2.0 batch handler for v2 API.
//!
//! POST /v2/batch accepts an array of JSON-RPC 2.0 requests, executes them
//! concurrently (up to the configured max batch size), and returns an array
//! of JSON-RPC 2.0 responses.
//!
//! **Phase 3 optimization**: all requests within a single batch share a pinned
//! ledger version. This guarantees consistency (every request sees the same
//! snapshot) and avoids repeated ledger-info lookups.

use crate::v2::{
    context::{spawn_blocking, V2Context},
    cursor::Cursor,
    error::{ErrorCode, V2Error},
    types::LedgerMetadata,
};
use aptos_api_types::{AsConverter, LedgerInfo, MoveModuleBytecode, MoveResource};
use aptos_types::{account_address::AccountAddress, state_store::table::TableHandle};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

/// Snapshot of ledger state pinned at the start of a batch.
/// All requests in the batch use this same version for consistency.
#[derive(Clone)]
struct BatchSnapshot {
    ledger_info: Arc<LedgerInfo>,
    version: u64,
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
            format!("Batch size {} exceeds maximum {}", requests.len(), max_size),
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

    // Record batch size metric.
    crate::v2::metrics::BATCH_SIZE
        .with_label_values(&[])
        .observe(requests.len() as f64);

    // Pin the ledger version once for the entire batch.
    let ledger_info = ctx.ledger_info()?;
    let snapshot = BatchSnapshot {
        version: ledger_info.version(),
        ledger_info: Arc::new(ledger_info),
    };

    // Execute requests concurrently, all sharing the same snapshot.
    let mut handles = Vec::with_capacity(requests.len());
    for req in requests {
        let ctx = ctx.clone();
        let snap = snapshot.clone();
        handles.push(tokio::spawn(async move {
            dispatch_request(&ctx, &snap, req).await
        }));
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

async fn dispatch_request(
    ctx: &V2Context,
    snapshot: &BatchSnapshot,
    req: JsonRpcRequest,
) -> JsonRpcResponse {
    let id = req.id.clone();
    match dispatch_inner(ctx, snapshot, &req).await {
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
    snapshot: &BatchSnapshot,
    req: &JsonRpcRequest,
) -> Result<serde_json::Value, V2Error> {
    match req.method.as_str() {
        "get_ledger_info" => get_ledger_info(snapshot),
        "get_account" => get_account(ctx, snapshot, &req.params).await,
        "get_resources" => get_resources(ctx, snapshot, &req.params).await,
        "get_resource" => get_resource(ctx, snapshot, &req.params).await,
        "get_modules" => get_modules(ctx, snapshot, &req.params).await,
        "get_module" => get_module(ctx, snapshot, &req.params).await,
        "get_transaction" => get_transaction(ctx, snapshot, &req.params).await,
        "get_transaction_by_version" => {
            get_transaction_by_version(ctx, snapshot, &req.params).await
        },
        "get_transactions" => get_transactions(ctx, snapshot, &req.params).await,
        "get_block" => get_block(ctx, &req.params).await,
        "get_block_by_version" => get_block_by_version(ctx, &req.params).await,
        "estimate_gas_price" => estimate_gas_price(ctx, snapshot).await,
        "get_table_item" => get_table_item(ctx, snapshot, &req.params).await,
        _ => Err(V2Error::bad_request(
            ErrorCode::MethodNotFound,
            format!("Unknown method: {}", req.method),
        )),
    }
}

// --- Individual method implementations ---

fn get_ledger_info(snapshot: &BatchSnapshot) -> Result<serde_json::Value, V2Error> {
    let metadata = LedgerMetadata::from(snapshot.ledger_info.as_ref());
    serde_json::to_value(&metadata).map_err(V2Error::internal)
}

#[derive(Deserialize)]
struct ResourcesParams {
    address: String,
    #[serde(default)]
    cursor: Option<String>,
    /// Per-request version override. If absent, uses the batch-pinned version.
    #[serde(default)]
    ledger_version: Option<u64>,
}

async fn get_resources(
    ctx: &V2Context,
    snapshot: &BatchSnapshot,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: ResourcesParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        let address = parse_address(&p.address)?;
        let version = p.ledger_version.unwrap_or(snap.version);

        // Validate version is within range.
        validate_version(version, &snap.ledger_info)?;

        let state_view = ctx
            .inner()
            .state_view_at_version(version)
            .map_err(V2Error::internal)?;

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
            ledger: LedgerMetadata::from(snap.ledger_info.as_ref()),
            cursor: next_cursor.map(|c| c.encode()),
        };

        serde_json::to_value(&result).map_err(V2Error::internal)
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
    snapshot: &BatchSnapshot,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: SingleResourceParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        let address = parse_address(&p.address)?;
        let tag: move_core_types::language_storage::StructTag =
            p.resource_type.parse().map_err(|e| {
                V2Error::bad_request(
                    ErrorCode::InvalidInput,
                    format!("Invalid struct tag: {}", e),
                )
            })?;

        let version = p.ledger_version.unwrap_or(snap.version);
        validate_version(version, &snap.ledger_info)?;

        let state_view = ctx
            .inner()
            .state_view_at_version(version)
            .map_err(V2Error::internal)?;

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

        serde_json::to_value(&resource).map_err(V2Error::internal)
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
    snapshot: &BatchSnapshot,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: ModulesParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        let address = parse_address(&p.address)?;
        let version = p.ledger_version.unwrap_or(snap.version);
        validate_version(version, &snap.ledger_info)?;

        let cursor = p.cursor.as_ref().map(|c| Cursor::decode(c)).transpose()?;

        let (modules, next_cursor) =
            ctx.get_modules_paginated(address, cursor.as_ref(), version)?;

        let mut move_modules = Vec::with_capacity(modules.len());
        for (_id, bytes) in modules {
            let m = MoveModuleBytecode::new(bytes)
                .try_parse_abi()
                .map_err(V2Error::internal)?;
            move_modules.push(m);
        }

        let result = PaginatedResult {
            data: move_modules,
            ledger: LedgerMetadata::from(snap.ledger_info.as_ref()),
            cursor: next_cursor.map(|c| c.encode()),
        };

        serde_json::to_value(&result).map_err(V2Error::internal)
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
    snapshot: &BatchSnapshot,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: SingleModuleParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        let address = parse_address(&p.address)?;
        let version = p.ledger_version.unwrap_or(snap.version);
        validate_version(version, &snap.ledger_info)?;

        let state_view = ctx
            .inner()
            .state_view_at_version(version)
            .map_err(V2Error::internal)?;

        let module_id = move_core_types::language_storage::ModuleId::new(
            address,
            move_core_types::identifier::Identifier::new(p.module_name.clone()).map_err(|e| {
                V2Error::bad_request(
                    ErrorCode::InvalidInput,
                    format!("Invalid module name: {}", e),
                )
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
            .map_err(V2Error::internal)?;

        serde_json::to_value(&module).map_err(V2Error::internal)
    })
    .await
}

#[derive(Deserialize)]
struct TransactionParams {
    hash: String,
}

async fn get_transaction(
    ctx: &V2Context,
    snapshot: &BatchSnapshot,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: TransactionParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let hash = parse_hash_str(&p.hash)?;

    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        let txn_data = ctx
            .inner()
            .get_transaction_by_hash(hash, snap.version)
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
        let state_view = ctx
            .inner()
            .state_view_at_version(snap.version)
            .map_err(V2Error::internal)?;
        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());
        let txn = converter
            .try_into_onchain_transaction(timestamp, txn_data)
            .map_err(V2Error::internal)?;

        serde_json::to_value(&txn).map_err(V2Error::internal)
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
    snapshot: &BatchSnapshot,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: TransactionsListParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        let cursor = p.cursor.as_ref().map(|c| Cursor::decode(c)).transpose()?;

        let (txns, next_cursor) = ctx.get_transactions_paginated(cursor.as_ref(), snap.version)?;

        let state_view = ctx
            .inner()
            .state_view_at_version(snap.version)
            .map_err(V2Error::internal)?;
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
            ledger: LedgerMetadata::from(snap.ledger_info.as_ref()),
            cursor: next_cursor.map(|c| c.encode()),
        };

        serde_json::to_value(&result).map_err(V2Error::internal)
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

        serde_json::to_value(&block).map_err(V2Error::internal)
    })
    .await
}

// --- New batch methods ---

#[derive(Deserialize)]
struct AccountParams {
    address: String,
    #[serde(default)]
    ledger_version: Option<u64>,
}

async fn get_account(
    ctx: &V2Context,
    snapshot: &BatchSnapshot,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: AccountParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        let address = parse_address(&p.address)?;
        let version = p.ledger_version.unwrap_or(snap.version);
        validate_version(version, &snap.ledger_info)?;

        let state_view = ctx
            .inner()
            .state_view_at_version(version)
            .map_err(V2Error::internal)?;

        let state_key = aptos_types::state_store::state_key::StateKey::resource_typed::<
            aptos_types::account_config::AccountResource,
        >(&address)
        .map_err(V2Error::internal)?;

        use aptos_types::state_store::TStateView;
        let bytes = state_view
            .get_state_value_bytes(&state_key)
            .map_err(V2Error::internal)?;

        let account_data = match bytes {
            Some(bytes) => {
                let ar: aptos_types::account_config::AccountResource =
                    bcs::from_bytes(&bytes).map_err(V2Error::internal)?;
                aptos_api_types::AccountData::from(ar)
            },
            None => {
                return Err(V2Error::not_found(
                    ErrorCode::AccountNotFound,
                    format!("Account {} not found", address),
                ));
            },
        };

        serde_json::to_value(&account_data).map_err(V2Error::internal)
    })
    .await
}

#[derive(Deserialize)]
struct TransactionByVersionParams {
    version: u64,
}

async fn get_transaction_by_version(
    ctx: &V2Context,
    snapshot: &BatchSnapshot,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: TransactionByVersionParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        validate_version(p.version, &snap.ledger_info)?;

        let txn_data = ctx
            .inner()
            .get_transaction_by_version(p.version, snap.version)
            .map_err(V2Error::internal)?;

        let timestamp = ctx
            .inner()
            .db
            .get_block_timestamp(txn_data.version)
            .map_err(V2Error::internal)?;
        let state_view = ctx
            .inner()
            .state_view_at_version(snap.version)
            .map_err(V2Error::internal)?;
        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());
        let txn = converter
            .try_into_onchain_transaction(timestamp, txn_data)
            .map_err(V2Error::internal)?;

        serde_json::to_value(&txn).map_err(V2Error::internal)
    })
    .await
}

#[derive(Deserialize)]
struct BlockByVersionParams {
    version: u64,
    #[serde(default)]
    with_transactions: Option<bool>,
}

async fn get_block_by_version(
    ctx: &V2Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: BlockByVersionParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let ledger_info = ctx.ledger_info()?;
        let with_txns = p.with_transactions.unwrap_or(false);

        let bcs_block = ctx
            .inner()
            .get_block_by_version::<crate::response::BasicErrorWith404>(
                p.version,
                &ledger_info,
                with_txns,
            )
            .map_err(|e| V2Error::internal(format!("{}", e)))?;

        let block_height: u64 = ledger_info.block_height.into();
        let block = aptos_api_types::Block {
            block_height: block_height.into(),
            block_hash: bcs_block.block_hash.into(),
            block_timestamp: bcs_block.block_timestamp.into(),
            first_version: bcs_block.first_version.into(),
            last_version: bcs_block.last_version.into(),
            transactions: None,
        };

        serde_json::to_value(&block).map_err(V2Error::internal)
    })
    .await
}

async fn estimate_gas_price(
    ctx: &V2Context,
    snapshot: &BatchSnapshot,
) -> Result<serde_json::Value, V2Error> {
    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        let gas_estimation = ctx
            .inner()
            .estimate_gas_price(&snap.ledger_info)
            .map_err(|e: crate::response::BasicError| V2Error::internal(format!("{}", e)))?;

        serde_json::to_value(&gas_estimation).map_err(V2Error::internal)
    })
    .await
}

#[derive(Deserialize)]
struct TableItemParams {
    table_handle: String,
    key_type: aptos_api_types::MoveType,
    value_type: aptos_api_types::MoveType,
    key: serde_json::Value,
    #[serde(default)]
    ledger_version: Option<u64>,
}

async fn get_table_item(
    ctx: &V2Context,
    snapshot: &BatchSnapshot,
    params: &serde_json::Value,
) -> Result<serde_json::Value, V2Error> {
    let p: TableItemParams = serde_json::from_value(params.clone())
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    let snap = snapshot.clone();
    spawn_blocking(move || {
        let handle_address = parse_address(&p.table_handle)?;
        let version = p.ledger_version.unwrap_or(snap.version);
        validate_version(version, &snap.ledger_info)?;

        let state_view = ctx
            .inner()
            .state_view_at_version(version)
            .map_err(V2Error::internal)?;

        let key_type = (&p.key_type).try_into().map_err(|e: anyhow::Error| {
            V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid key_type: {}", e))
        })?;
        let value_type = (&p.value_type).try_into().map_err(|e: anyhow::Error| {
            V2Error::bad_request(
                ErrorCode::InvalidInput,
                format!("Invalid value_type: {}", e),
            )
        })?;

        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

        let vm_key = converter
            .try_into_vm_value(&key_type, p.key.clone())
            .map_err(|e| {
                V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid key: {}", e))
            })?;
        let raw_key = vm_key.undecorate().simple_serialize().ok_or_else(|| {
            V2Error::bad_request(ErrorCode::InvalidInput, "Failed to serialize table key")
        })?;

        let state_key = aptos_types::state_store::state_key::StateKey::table_item(
            &TableHandle(handle_address.into()),
            &raw_key,
        );

        use aptos_types::state_store::TStateView;
        let bytes = state_view
            .get_state_value_bytes(&state_key)
            .map_err(V2Error::internal)?
            .ok_or_else(|| {
                V2Error::not_found(ErrorCode::TableItemNotFound, "Table item not found")
            })?;

        let move_value = converter
            .try_into_move_value(&value_type, &bytes)
            .map_err(V2Error::internal)?;

        serde_json::to_value(&move_value).map_err(V2Error::internal)
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
    aptos_crypto::HashValue::from_hex(s)
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid hash: {}", e)))
}

/// Validate that a version is within the range visible to the batch snapshot.
fn validate_version(version: u64, ledger_info: &LedgerInfo) -> Result<(), V2Error> {
    if version > ledger_info.version() {
        return Err(V2Error::not_found(
            ErrorCode::VersionNotFound,
            format!(
                "Version {} is in the future (latest: {})",
                version,
                ledger_info.version()
            ),
        ));
    }
    if version < ledger_info.oldest_version() {
        return Err(V2Error::gone(
            ErrorCode::VersionPruned,
            format!(
                "Version {} has been pruned (oldest: {})",
                version,
                ledger_info.oldest_version()
            ),
        ));
    }
    Ok(())
}
