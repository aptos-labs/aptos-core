# V2 Design: Endpoints & Request/Response Model

## Overview

This document details the v2 endpoint handler patterns, the request/response serialization
model, the BCS versioned envelope, and the JSON-only output default. It covers how each
v1 endpoint maps to its v2 equivalent, what changes in the API contract, and the concrete
Rust types involved.

---

## Key Differences from v1

| Aspect | v1 (Poem) | v2 (Axum) |
|---|---|---|
| Output format | JSON or BCS (via Accept header) | JSON only (BCS output deprecated) |
| Input format (reads) | JSON or BCS | JSON or BCS (versioned envelope) |
| Input format (tx submit) | JSON or BCS | BCS only (versioned envelope) |
| Error format | `AptosError` with Poem macros | `V2Error` (typed ErrorCode enum) |
| Response headers | `X-Aptos-*` ledger info headers | JSON body includes ledger info |
| OpenAPI generation | `poem-openapi` macros | `utoipa` derive macros |
| Handler pattern | `#[OpenApi] impl XxxApi { ... }` | Free functions with `State` extractor |
| Blocking DB reads | `api_spawn_blocking(closure)` | `spawn_blocking(closure)` |

---

## Response Envelope

v1 embeds ledger info in HTTP headers (`X-Aptos-Chain-Id`, `X-Aptos-Ledger-Version`, etc.).
v2 includes it in the JSON body for better client ergonomics:

```rust
// api/src/v2/types/mod.rs

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct V2Response<T: Serialize> {
    /// The actual response data.
    pub data: T,
    /// Ledger metadata at the time of the request.
    pub ledger: LedgerMetadata,
    /// Optional cursor for paginated responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct LedgerMetadata {
    pub chain_id: u8,
    pub ledger_version: u64,
    pub oldest_ledger_version: u64,
    pub ledger_timestamp_usec: u64,
    pub epoch: u64,
    pub block_height: u64,
    pub oldest_block_height: u64,
}

impl From<&LedgerInfo> for LedgerMetadata {
    fn from(info: &LedgerInfo) -> Self {
        LedgerMetadata {
            chain_id: info.chain_id,
            ledger_version: info.ledger_version.into(),
            oldest_ledger_version: info.oldest_ledger_version.into(),
            ledger_timestamp_usec: info.ledger_timestamp.into(),
            epoch: info.epoch.into(),
            block_height: info.block_height.into(),
            oldest_block_height: info.oldest_block_height.into(),
        }
    }
}
```

Helper to build responses:

```rust
impl<T: Serialize> V2Response<T> {
    pub fn new(data: T, ledger_info: &LedgerInfo) -> Self {
        Self {
            data,
            ledger: LedgerMetadata::from(ledger_info),
            cursor: None,
        }
    }

    pub fn with_cursor(mut self, cursor: Option<String>) -> Self {
        self.cursor = cursor;
        self
    }
}
```

---

## BCS Versioned Input Envelope

For endpoints that accept BCS input, the payload is wrapped in a versioned envelope:

```rust
// api/src/v2/bcs_versioned.rs

/// Versioned BCS request envelope.
/// The first byte(s) are a ULEB128-encoded enum discriminant (0 = V1, 1 = V2, ...),
/// followed by the version-specific BCS-encoded payload.
///
/// This allows us to evolve input types without breaking existing clients:
/// - Clients send V1 payloads today.
/// - When we add new fields, we define V2 payloads.
/// - The server reads the discriminant and deserializes the correct version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Versioned<V1> {
    V1(V1),
    // Future: V2(V2Type), V3(V3Type), etc.
}

impl<V1: serde::de::DeserializeOwned> Versioned<V1> {
    /// Deserialize a versioned BCS payload from raw bytes.
    pub fn from_bcs(bytes: &[u8]) -> Result<Self, V2Error> {
        bcs::from_bytes(bytes).map_err(|e| V2Error::bad_request(
            ErrorCode::InvalidBcsVersion,
            format!("Failed to deserialize versioned BCS input: {}", e),
        ))
    }

    /// Unwrap the inner value regardless of version (when all versions
    /// are convertible to the same inner type).
    pub fn into_inner(self) -> V1 {
        match self {
            Versioned::V1(v) => v,
        }
    }
}
```

### Content-Type Detection

```rust
// api/src/v2/extractors.rs

use axum::{
    extract::FromRequest,
    http::{header::CONTENT_TYPE, Request, StatusCode},
    body::Bytes,
};

/// Extractor that reads the request body as either JSON or versioned BCS,
/// depending on Content-Type header.
pub enum JsonOrBcs<T: serde::de::DeserializeOwned> {
    Json(T),
    Bcs(Versioned<T>),
}

#[axum::async_trait]
impl<S, T> FromRequest<S> for JsonOrBcs<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned + Send + 'static,
{
    type Rejection = V2Error;

    async fn from_request(req: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json");

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

        if content_type.contains("application/json") {
            let value = serde_json::from_slice(&bytes)
                .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid JSON: {}", e)))?;
            Ok(JsonOrBcs::Json(value))
        } else if content_type.contains("bcs") || content_type.contains("octet-stream") {
            let versioned = Versioned::from_bcs(&bytes)?;
            Ok(JsonOrBcs::Bcs(versioned))
        } else {
            Err(V2Error::bad_request(
                ErrorCode::InvalidInput,
                format!("Unsupported Content-Type: {}", content_type),
            ))
        }
    }
}

/// Extractor for BCS-only endpoints (like transaction submission).
/// Rejects JSON input.
pub struct BcsOnly<T: serde::de::DeserializeOwned>(pub Versioned<T>);

#[axum::async_trait]
impl<S, T> FromRequest<S> for BcsOnly<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned + Send + 'static,
{
    type Rejection = V2Error;

    async fn from_request(req: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req.headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("json") {
            return Err(V2Error::bad_request(
                ErrorCode::InvalidInput,
                "This endpoint only accepts BCS input. JSON transaction submission is not supported in v2.",
            ));
        }

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

        let versioned = Versioned::from_bcs(&bytes)?;
        Ok(BcsOnly(versioned))
    }
}
```

---

## Endpoint Implementations

### Health Check

```rust
// api/src/v2/endpoints/health.rs

/// GET /v2/health
#[utoipa::path(get, path = "/v2/health", responses(
    (status = 200, description = "Node is healthy"),
    (status = 503, description = "Node is not ready"),
))]
pub async fn health_handler(
    State(ctx): State<V2Context>,
) -> Result<Json<HealthResponse>, V2Error> {
    let ledger_info = ctx.ledger_info()?;
    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        ledger: LedgerMetadata::from(&ledger_info),
    }))
}

/// GET /v2/info
#[utoipa::path(get, path = "/v2/info", responses(
    (status = 200, description = "Node information"),
))]
pub async fn info_handler(
    State(ctx): State<V2Context>,
) -> Result<Json<V2Response<NodeInfo>>, V2Error> {
    let ledger_info = ctx.ledger_info()?;
    let info = NodeInfo {
        chain_id: ctx.inner().chain_id().id(),
        role: format!("{:?}", ctx.inner().node_role()),
        api_version: "2.0.0".to_string(),
    };
    Ok(Json(V2Response::new(info, &ledger_info)))
}
```

### Resources

```rust
// api/src/v2/endpoints/resources.rs

/// GET /v2/accounts/:address/resources
#[utoipa::path(get, path = "/v2/accounts/{address}/resources",
    params(
        ("address" = String, Path, description = "Account address"),
        ("ledger_version" = Option<u64>, Query, description = "Ledger version"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor"),
        ("limit" = Option<u16>, Query, description = "Page size limit"),
    ),
    responses(
        (status = 200, description = "Account resources"),
        (status = 404, description = "Account not found"),
    ),
)]
pub async fn get_resources_handler(
    State(ctx): State<V2Context>,
    Path(address): Path<String>,
    Query(params): Query<ResourcesParams>,
) -> Result<Json<V2Response<Vec<MoveResource>>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&address)?;
        let (ledger_info, version, state_view) = ctx.state_view_at(params.ledger_version)?;

        let limit = params.limit
            .unwrap_or(ctx.v2_config.max_account_resources_page_size)
            .min(ctx.v2_config.max_account_resources_page_size);

        let prev_key = params.cursor
            .as_ref()
            .map(|c| decode_cursor(c))
            .transpose()?;

        let (resources, next_key) = ctx.inner()
            .get_resources_by_pagination(address, prev_key.as_ref(), version, limit as u64)
            .map_err(V2Error::internal)?;

        // Convert to MoveResource JSON representation
        let converter = state_view.as_converter(
            ctx.inner().db.clone(),
            ctx.inner().indexer_reader.clone(),
        );

        let move_resources: Vec<MoveResource> = resources
            .into_iter()
            .map(|(tag, bytes)| converter.try_into_resource(&tag, &bytes))
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(V2Error::internal)?;

        let cursor = next_key.map(|k| encode_cursor(&k));
        Ok(Json(V2Response::new(move_resources, &ledger_info).with_cursor(cursor)))
    }).await
}

/// GET /v2/accounts/:address/resource/:resource_type
pub async fn get_resource_handler(
    State(ctx): State<V2Context>,
    Path((address, resource_type)): Path<(String, String)>,
    Query(params): Query<LedgerVersionParam>,
) -> Result<Json<V2Response<MoveResource>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&address)?;
        let tag = parse_struct_tag(&resource_type)?;
        let (ledger_info, version, state_view) = ctx.state_view_at(params.ledger_version)?;

        let converter = state_view.as_converter(
            ctx.inner().db.clone(),
            ctx.inner().indexer_reader.clone(),
        );

        let bytes = converter
            .find_resource(&state_view, address.into(), &tag)
            .map_err(V2Error::internal)?
            .ok_or_else(|| V2Error::not_found(
                ErrorCode::ResourceNotFound,
                format!("Resource {} not found at {}", tag, address),
            ))?;

        let resource = converter
            .try_into_resource(&tag, &bytes)
            .map_err(V2Error::internal)?;

        Ok(Json(V2Response::new(resource, &ledger_info)))
    }).await
}
```

### View Function

```rust
// api/src/v2/endpoints/view.rs

/// POST /v2/view
///
/// Accepts JSON or BCS input. Always returns JSON output.
#[utoipa::path(post, path = "/v2/view",
    request_body(content = ViewRequest, content_type = "application/json"),
    params(
        ("ledger_version" = Option<u64>, Query, description = "Ledger version"),
    ),
    responses(
        (status = 200, description = "View function result"),
        (status = 400, description = "Invalid input"),
    ),
)]
pub async fn view_handler(
    State(ctx): State<V2Context>,
    Query(params): Query<LedgerVersionParam>,
    body: JsonOrBcs<ViewRequest>,
) -> Result<Json<V2Response<Vec<serde_json::Value>>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let (ledger_info, version, state_view) = ctx.state_view_at(params.ledger_version)?;

        let view_function = match body {
            JsonOrBcs::Json(request) => {
                let converter = state_view.as_converter(
                    ctx.inner().db.clone(),
                    ctx.inner().indexer_reader.clone(),
                );
                converter.convert_view_function(request)
                    .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?
            },
            JsonOrBcs::Bcs(versioned) => {
                let request = versioned.into_inner();
                // BCS ViewFunction is already in the right format
                request
            },
        };

        // Check view filter
        if !ctx.v2_config.view_filter.allows(
            view_function.module.address(),
            view_function.module.name().as_str(),
            view_function.function.as_str(),
        ) {
            return Err(V2Error::forbidden(
                ErrorCode::ViewFunctionForbidden,
                format!("Function {}::{} is not allowed", view_function.module, view_function.function),
            ));
        }

        // Execute the view function
        let output = AptosVM::execute_view_function(
            &state_view,
            view_function.module.clone(),
            view_function.function.clone(),
            view_function.ty_args.clone(),
            view_function.args.clone(),
            ctx.v2_config.max_gas_view_function,
        );

        let values = output.values.map_err(|status| {
            let (err_string, _vm_code) = convert_view_function_error(&status, &state_view, ctx.inner());
            V2Error::bad_request(ErrorCode::ViewFunctionFailed, err_string)
        })?;

        // Convert return values to JSON
        let converter = state_view.as_converter(
            ctx.inner().db.clone(),
            ctx.inner().indexer_reader.clone(),
        );
        let return_types = converter
            .function_return_types(&view_function)
            .and_then(|tys| tys.iter().map(TypeTag::try_from).collect::<anyhow::Result<Vec<_>>>())
            .map_err(|e| V2Error::internal(e))?;

        let json_values: Vec<serde_json::Value> = values
            .into_iter()
            .zip(return_types)
            .map(|(bytes, ty)| {
                let move_val = converter.try_into_move_value(&ty, &bytes)?;
                serde_json::to_value(&move_val).map_err(anyhow::Error::from)
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(V2Error::internal)?;

        // Track stats
        ctx.inner().view_function_stats().increment(
            FunctionStats::function_to_key(&view_function.module, &view_function.function),
            output.gas_used,
        );

        let mut response = V2Response::new(json_values, &ledger_info);
        Ok(Json(response))
    }).await
}
```

### Transaction Submission

```rust
// api/src/v2/endpoints/transactions.rs

/// POST /v2/transactions
///
/// BCS-only. JSON submission is not supported in v2.
/// The BCS payload is a Versioned<SignedTransaction>.
#[utoipa::path(post, path = "/v2/transactions",
    responses(
        (status = 202, description = "Transaction submitted"),
        (status = 400, description = "Invalid transaction"),
    ),
)]
pub async fn submit_transaction_handler(
    State(ctx): State<V2Context>,
    body: BcsOnly<SignedTransaction>,
) -> Result<(StatusCode, Json<V2Response<PendingTransactionSummary>>), V2Error> {
    let signed_txn: SignedTransaction = body.0.into_inner();

    // Validate transaction
    // (signature verification happens in mempool, but we can do basic checks here)

    let submission_status = ctx.inner()
        .submit_transaction(signed_txn.clone())
        .await
        .map_err(V2Error::internal)?;

    match submission_status.0.code {
        MempoolStatusCode::Accepted => {
            let summary = PendingTransactionSummary {
                hash: signed_txn.committed_hash().to_hex(),
                sender: signed_txn.sender().to_hex_literal(),
                sequence_number: signed_txn.sequence_number(),
            };
            let ledger_info = ctx.ledger_info()?;
            Ok((
                StatusCode::ACCEPTED,
                Json(V2Response::new(summary, &ledger_info)),
            ))
        }
        code => {
            Err(V2Error::from_mempool_status(code, &submission_status.0.message))
        }
    }
}

/// GET /v2/transactions/:hash
#[utoipa::path(get, path = "/v2/transactions/{hash}",
    params(("hash" = String, Path, description = "Transaction hash")),
    responses(
        (status = 200, description = "Transaction found"),
        (status = 404, description = "Transaction not found"),
    ),
)]
pub async fn get_transaction_handler(
    State(ctx): State<V2Context>,
    Path(hash): Path<String>,
) -> Result<Json<V2Response<Transaction>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let hash = parse_hash(&hash)?;
        let ledger_info = ctx.ledger_info()?;
        let version = ledger_info.version();

        match ctx.inner().get_transaction_by_hash(hash, version).map_err(V2Error::internal)? {
            Some(txn_data) => {
                let timestamp = ctx.inner().db.get_block_timestamp(txn_data.version)
                    .map_err(V2Error::internal)?;
                let state_view = ctx.inner().latest_state_view().map_err(V2Error::internal)?;
                let converter = state_view.as_converter(
                    ctx.inner().db.clone(),
                    ctx.inner().indexer_reader.clone(),
                );
                let txn = converter.try_into_onchain_transaction(timestamp, txn_data)
                    .map_err(V2Error::internal)?;
                Ok(Json(V2Response::new(txn, &ledger_info)))
            }
            None => {
                Err(V2Error::not_found(
                    ErrorCode::TransactionNotFound,
                    format!("Transaction {} not found", hash),
                ))
            }
        }
    }).await
}

/// GET /v2/transactions/:hash/wait
///
/// Long-polls until the transaction is committed or timeout.
/// In v2, consider using WebSocket subscription instead.
pub async fn wait_transaction_handler(
    State(ctx): State<V2Context>,
    Path(hash): Path<String>,
) -> Result<Json<V2Response<Transaction>>, V2Error> {
    let hash = parse_hash(&hash)?;
    let timeout = Duration::from_millis(ctx.v2_config.wait_by_hash_timeout_ms);
    let poll_interval = Duration::from_millis(ctx.v2_config.wait_by_hash_poll_interval_ms);
    let deadline = Instant::now() + timeout;

    loop {
        let ledger_info = ctx.ledger_info()?;
        let version = ledger_info.version();

        if let Some(txn_data) = ctx.inner()
            .get_transaction_by_hash(hash, version)
            .map_err(V2Error::internal)?
        {
            // Found it - render and return
            return spawn_blocking(move || {
                // ... render transaction ...
            }).await;
        }

        if Instant::now() >= deadline {
            return Err(V2Error::not_found(
                ErrorCode::TransactionNotFound,
                format!("Transaction {} not found within timeout", hash),
            ));
        }

        tokio::time::sleep(poll_interval).await;
    }
}
```

### Blocks

```rust
// api/src/v2/endpoints/blocks.rs

/// GET /v2/blocks/:height
#[utoipa::path(get, path = "/v2/blocks/{height}",
    params(
        ("height" = u64, Path, description = "Block height"),
        ("with_transactions" = Option<bool>, Query),
    ),
)]
pub async fn get_block_by_height_handler(
    State(ctx): State<V2Context>,
    Path(height): Path<u64>,
    Query(params): Query<BlockParams>,
) -> Result<Json<V2Response<Block>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let with_txns = params.with_transactions.unwrap_or(false);
        let bcs_block = ctx.get_block_by_height(height, with_txns)?;
        let ledger_info = ctx.ledger_info()?;

        let block = render_block(&ctx, &ledger_info, bcs_block)?;
        Ok(Json(V2Response::new(block, &ledger_info)))
    }).await
}

/// GET /v2/blocks/latest
pub async fn get_latest_block_handler(
    State(ctx): State<V2Context>,
) -> Result<Json<V2Response<Block>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let ledger_info = ctx.ledger_info()?;
        let bcs_block = ctx.get_block_by_height(ledger_info.block_height.0, false)?;
        let block = render_block(&ctx, &ledger_info, bcs_block)?;
        Ok(Json(V2Response::new(block, &ledger_info)))
    }).await
}
```

---

## Query Parameter Types

```rust
#[derive(Deserialize, utoipa::IntoParams)]
pub struct LedgerVersionParam {
    pub ledger_version: Option<u64>,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ResourcesParams {
    pub ledger_version: Option<u64>,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct BlockParams {
    pub with_transactions: Option<bool>,
}
```

---

## Axum Error Response Integration

`V2Error` implements `axum::response::IntoResponse` so it can be returned directly
from handlers:

```rust
impl axum::response::IntoResponse for V2Error {
    fn into_response(self) -> axum::response::Response {
        let status = self.http_status();
        let body = Json(self);
        (status, body).into_response()
    }
}
```

This means every handler can return `Result<Json<T>, V2Error>` and Axum will automatically
serialize the error as JSON with the correct HTTP status code.
