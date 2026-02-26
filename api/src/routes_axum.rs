// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Axum route handlers for the Aptos Node API.
//!
//! Each handler calls a framework-agnostic `_inner` function that returns
//! `AptosResponse<T>` / `AptosErrorResponse`, then converts to an Axum `Response`.
//! No Poem types are used in the request or response path.

use crate::{accept_type::AcceptType, context::Context, response_axum::AptosErrorResponse};
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;

type Ctx = State<Arc<Context>>;

/// A Path extractor wrapper that converts Axum's default `PathRejection`
/// into the `AptosError` JSON format, ensuring consistent error responses.
pub struct AptosPath<T>(pub T);

#[axum::async_trait]
impl<S, T> axum::extract::FromRequestParts<S> for AptosPath<T>
where
    T: serde::de::DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = AptosErrorResponse;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        match Path::<T>::from_request_parts(parts, state).await {
            Ok(Path(value)) => Ok(AptosPath(value)),
            Err(rejection) => Err(AptosErrorResponse::new(
                StatusCode::BAD_REQUEST,
                aptos_api_types::AptosError::new_with_error_code(
                    rejection.body_text(),
                    aptos_api_types::AptosErrorCode::WebFrameworkError,
                ),
                None,
            )),
        }
    }
}

fn unsupported_content_type(content_type: &str) -> AptosErrorResponse {
    AptosErrorResponse::new(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        aptos_api_types::AptosError::new_with_error_code(
            format!(
                "the `Content-Type` requested by the client is not supported: {}",
                content_type
            ),
            aptos_api_types::AptosErrorCode::WebFrameworkError,
        ),
        None,
    )
}

// ---- Query parameter types ----

#[derive(Debug, Deserialize, Default)]
pub struct LedgerVersionQuery {
    pub ledger_version: Option<aptos_api_types::U64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PaginationQuery {
    pub start: Option<aptos_api_types::U64>,
    pub limit: Option<u16>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ResourcePaginationQuery {
    pub ledger_version: Option<aptos_api_types::U64>,
    pub start: Option<aptos_api_types::StateKeyWrapper>,
    pub limit: Option<u16>,
}

#[derive(Debug, Deserialize, Default)]
pub struct BlockQuery {
    pub with_transactions: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct HealthCheckQuery {
    pub duration_secs: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
pub struct EventPaginationQuery {
    pub start: Option<aptos_api_types::U64>,
    pub limit: Option<u16>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TxnAuxInfoQuery {
    pub start_version: Option<aptos_api_types::U64>,
    pub limit: Option<u16>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TxnSummaryQuery {
    pub start_version: Option<aptos_api_types::U64>,
    pub end_version: Option<aptos_api_types::U64>,
    pub limit: Option<u16>,
}

// ---- Root handler ----

pub async fn root_handler() -> Html<&'static str> {
    Html(
        "<html>
<head>
    <title>Aptos Node API</title>
</head>
<body>
    <p>
        Welcome! The latest node API can be found at <a href=\"/v1\">/v1<a/>.
    </p>
    <p>
        Learn more about the v1 node API here: <a href=\"/v1/spec\">/v1/spec<a/>.
    </p>
</body>
</html>",
    )
}

// ========================================================================
// Route handlers — each calls a framework-agnostic inner function.
// ========================================================================

// ---- BasicApi ----

const OPEN_API_HTML: &str = include_str!("../doc/spec.html");

pub async fn spec_handler() -> Html<String> {
    Html(OPEN_API_HTML.to_string())
}

pub async fn info_handler(State(context): Ctx) -> impl IntoResponse {
    use aptos_config::config::NodeType;
    use std::collections::HashMap;

    let mut info = HashMap::new();
    info.insert(
        "chain_id".to_string(),
        serde_json::to_value(format!("{:?}", context.chain_id()))
            .expect("chain_id serialization cannot fail"),
    );
    let node_type = NodeType::extract_from_config(&context.node_config);
    info.insert(
        "node_type".to_string(),
        serde_json::to_value(node_type).expect("node_type serialization cannot fail"),
    );
    info.insert(
        "bootstrapping_mode".to_string(),
        serde_json::to_value(
            context
                .node_config
                .state_sync
                .state_sync_driver
                .bootstrapping_mode,
        )
        .expect("bootstrapping_mode serialization cannot fail"),
    );
    info.insert(
        "continuous_syncing_mode".to_string(),
        serde_json::to_value(
            context
                .node_config
                .state_sync
                .state_sync_driver
                .continuous_syncing_mode,
        )
        .expect("continuous_syncing_mode serialization cannot fail"),
    );
    info.insert(
        "new_storage_format".to_string(),
        serde_json::to_value(
            context
                .node_config
                .storage
                .rocksdb_configs
                .enable_storage_sharding,
        )
        .expect("new_storage_format serialization cannot fail"),
    );
    info.insert(
        "internal_indexer_config".to_string(),
        serde_json::to_value(&context.node_config.indexer_db_config)
            .expect("internal_indexer_config serialization cannot fail"),
    );
    if let Some(validator_network) = &context.node_config.validator_network {
        info.insert(
            "validator_network_peer_id".to_string(),
            serde_json::to_value(validator_network.peer_id())
                .expect("validator_network_peer_id serialization cannot fail"),
        );
    }
    for fullnode_network in &context.node_config.full_node_networks {
        info.insert(
            format!("fullnode_network_peer_id_{}", fullnode_network.network_id),
            serde_json::to_value(fullnode_network.peer_id())
                .expect("fullnode_network_peer_id serialization cannot fail"),
        );
    }

    axum::Json(info)
}

pub async fn healthy_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    Query(query): Query<HealthCheckQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    let ctx = context.clone();
    let at = accept_type.clone();
    let dur = query.duration_secs;
    let resp = api_spawn_blocking(move || crate::basic::healthy_inner(&ctx, &at, dur)).await?;
    Ok(resp.into_response())
}

// ---- IndexApi ----

pub async fn get_ledger_info_handler(
    State(context): Ctx,
    accept_type: AcceptType,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    let ctx = context.clone();
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || crate::index::get_ledger_info_inner(&ctx, &at)).await?;
    Ok(resp.into_response())
}

// ---- AccountsApi ----

pub async fn get_account_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(address): AptosPath<aptos_api_types::Address>,
    Query(query): Query<LedgerVersionQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_account")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Get account", &accept_type)?;
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let account =
            crate::accounts::Account::new(context, address, query.ledger_version, None, None)?;
        account.account(&at)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_account_resources_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(address): AptosPath<aptos_api_types::Address>,
    Query(query): Query<ResourcePaginationQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    use aptos_types::state_store::state_key::StateKey;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_account_resources")?;
    context
        .check_api_output_enabled::<AptosErrorResponse>("Get account resources", &accept_type)?;
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let account = crate::accounts::Account::new(
            context,
            address,
            query.ledger_version,
            query.start.map(StateKey::from),
            query.limit,
        )?;
        account.resources(&at)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_account_balance_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath((address, asset_type)): AptosPath<(
        aptos_api_types::Address,
        aptos_api_types::AssetType,
    )>,
    Query(query): Query<LedgerVersionQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_account_balance")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Get account balance", &accept_type)?;
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let account =
            crate::accounts::Account::new(context, address, query.ledger_version, None, None)?;
        account.balance(asset_type, &at)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_account_modules_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(address): AptosPath<aptos_api_types::Address>,
    Query(query): Query<ResourcePaginationQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    use aptos_types::state_store::state_key::StateKey;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_account_modules")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Get account modules", &accept_type)?;
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let account = crate::accounts::Account::new(
            context,
            address,
            query.ledger_version,
            query.start.map(StateKey::from),
            query.limit,
        )?;
        account.modules(&at)
    })
    .await?;
    Ok(resp.into_response())
}

// ---- BlocksApi ----

pub async fn get_block_by_height_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(block_height): AptosPath<u64>,
    Query(query): Query<BlockQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_block_by_height")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Get block by height", &accept_type)?;
    let ctx = context.clone();
    let at = accept_type.clone();
    let with_txns = query.with_transactions.unwrap_or_default();
    let resp = api_spawn_blocking(move || {
        crate::blocks::get_block_by_height_inner(&ctx, &at, block_height, with_txns)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_block_by_version_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(version): AptosPath<u64>,
    Query(query): Query<BlockQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_block_by_version")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Get block by version", &accept_type)?;
    let ctx = context.clone();
    let at = accept_type.clone();
    let with_txns = query.with_transactions.unwrap_or_default();
    let resp = api_spawn_blocking(move || {
        crate::blocks::get_block_by_version_inner(&ctx, &at, version, with_txns)
    })
    .await?;
    Ok(resp.into_response())
}

// ---- EventsApi ----

pub async fn get_events_by_creation_number_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath((address, creation_number)): AptosPath<(
        aptos_api_types::Address,
        aptos_api_types::U64,
    )>,
    Query(query): Query<EventPaginationQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::{context::api_spawn_blocking, page::Page};
    use aptos_types::event::EventKey;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_events_by_event_key")?;
    context
        .check_api_output_enabled::<AptosErrorResponse>("Get events by event key", &accept_type)?;
    let page = Page::new(
        query.start.map(|v| v.0),
        query.limit,
        context.max_events_page_size(),
    );
    let ctx = context.clone();
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let account = crate::accounts::Account::new(ctx.clone(), address, None, None, None)?;
        crate::events::list_events_inner(
            &ctx,
            account.latest_ledger_info,
            at,
            page,
            EventKey::new(creation_number.0, address.into()),
        )
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_events_by_event_handle_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath((address, event_handle, field_name)): AptosPath<(
        aptos_api_types::Address,
        aptos_api_types::MoveStructTag,
        aptos_api_types::IdentifierWrapper,
    )>,
    Query(query): Query<EventPaginationQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::{context::api_spawn_blocking, page::Page};
    use anyhow::Context as AnyhowContext;
    use aptos_api_types::VerifyInputWithRecursion;
    event_handle
        .verify(0)
        .context("'event_handle' invalid")
        .map_err(|err| {
            AptosErrorResponse::bad_request(
                err,
                aptos_api_types::AptosErrorCode::InvalidInput,
                None,
            )
        })?;
    aptos_api_types::verify_field_identifier(field_name.as_str())
        .context("'field_name' invalid")
        .map_err(|err| {
            AptosErrorResponse::bad_request(
                err,
                aptos_api_types::AptosErrorCode::InvalidInput,
                None,
            )
        })?;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_events_by_event_handle")?;
    context.check_api_output_enabled::<AptosErrorResponse>(
        "Get events by event handle",
        &accept_type,
    )?;
    let page = Page::new(
        query.start.map(|v| v.0),
        query.limit,
        context.max_events_page_size(),
    );
    let ctx = context.clone();
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let account = crate::accounts::Account::new(ctx.clone(), address, None, None, None)?;
        let key = account.find_event_key(event_handle, field_name.0)?;
        crate::events::list_events_inner(&ctx, account.latest_ledger_info, at, page, key)
    })
    .await?;
    Ok(resp.into_response())
}

// ---- StateApi ----

pub async fn get_account_resource_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath((address, resource_type)): AptosPath<(
        aptos_api_types::Address,
        aptos_api_types::MoveStructTag,
    )>,
    Query(query): Query<LedgerVersionQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    use anyhow::Context as AnyhowContext;
    use aptos_api_types::VerifyInputWithRecursion;
    resource_type
        .verify(0)
        .context("'resource_type' invalid")
        .map_err(|err| {
            AptosErrorResponse::bad_request(
                err,
                aptos_api_types::AptosErrorCode::InvalidInput,
                None,
            )
        })?;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_account_resource")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Get account resource", &accept_type)?;
    let ctx = context.clone();
    let at = accept_type.clone();
    let lv = query.ledger_version.map(|v| v.0);
    let resp = api_spawn_blocking(move || {
        crate::state::resource_inner(&ctx, &at, address, resource_type, lv)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_account_module_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath((address, module_name)): AptosPath<(
        aptos_api_types::Address,
        aptos_api_types::IdentifierWrapper,
    )>,
    Query(query): Query<LedgerVersionQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    use anyhow::Context as AnyhowContext;
    aptos_api_types::verify_module_identifier(module_name.0.as_str())
        .context("'module_name' invalid")
        .map_err(|err| {
            AptosErrorResponse::bad_request(
                err,
                aptos_api_types::AptosErrorCode::InvalidInput,
                None,
            )
        })?;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_account_module")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Get account module", &accept_type)?;
    let ctx = context.clone();
    let at = accept_type.clone();
    let lv = query.ledger_version;
    let resp = api_spawn_blocking(move || {
        crate::state::module_inner(&ctx, &at, address, module_name, lv.map(|v| v.0))
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_table_item_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(table_handle): AptosPath<aptos_api_types::Address>,
    Query(query): Query<LedgerVersionQuery>,
    raw_body: Bytes,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    use anyhow::Context as AnyhowContext;
    use aptos_api_types::VerifyInput;
    let body: aptos_api_types::TableItemRequest =
        serde_json::from_slice(&raw_body).map_err(|e| {
            AptosErrorResponse::new(
                StatusCode::BAD_REQUEST,
                aptos_api_types::AptosError::new_with_error_code(
                    format!("parse request payload error: {}", e),
                    aptos_api_types::AptosErrorCode::WebFrameworkError,
                ),
                None,
            )
        })?;
    body.verify()
        .context("'table_item_request' invalid")
        .map_err(|err| {
            AptosErrorResponse::bad_request(
                err,
                aptos_api_types::AptosErrorCode::InvalidInput,
                None,
            )
        })?;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_table_item")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Get table item", &accept_type)?;
    let ctx = context.clone();
    let at = accept_type.clone();
    let lv = query.ledger_version;
    let resp = api_spawn_blocking(move || {
        crate::state::table_item_inner(&ctx, &at, table_handle, body, lv.map(|v| v.0))
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_raw_table_item_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(table_handle): AptosPath<aptos_api_types::Address>,
    Query(query): Query<LedgerVersionQuery>,
    raw_body: Bytes,
) -> Result<Response, AptosErrorResponse> {
    let body: aptos_api_types::RawTableItemRequest =
        serde_json::from_slice(&raw_body).map_err(|e| {
            AptosErrorResponse::new(
                StatusCode::BAD_REQUEST,
                aptos_api_types::AptosError::new_with_error_code(
                    format!("parse request payload error: {}", e),
                    aptos_api_types::AptosErrorCode::WebFrameworkError,
                ),
                None,
            )
        })?;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_table_item")?;
    if AcceptType::Json == accept_type {
        return Err(crate::response_axum::api_forbidden(
            "Get raw table item",
            "Only BCS is supported as an AcceptType.",
        ));
    }
    context.check_api_output_enabled::<AptosErrorResponse>("Get raw table item", &accept_type)?;
    use crate::context::api_spawn_blocking;
    let ctx = context.clone();
    let at = accept_type.clone();
    let lv = query.ledger_version;
    let resp = api_spawn_blocking(move || {
        crate::state::raw_table_item_inner(&ctx, &at, table_handle, body, lv.map(|v| v.0))
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_raw_state_value_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    Query(query): Query<LedgerVersionQuery>,
    raw_body: Bytes,
) -> Result<Response, AptosErrorResponse> {
    let body: aptos_api_types::RawStateValueRequest =
        serde_json::from_slice(&raw_body).map_err(|e| {
            AptosErrorResponse::new(
                StatusCode::BAD_REQUEST,
                aptos_api_types::AptosError::new_with_error_code(
                    format!("parse request payload error: {}", e),
                    aptos_api_types::AptosErrorCode::WebFrameworkError,
                ),
                None,
            )
        })?;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_raw_state_value")?;
    if AcceptType::Json == accept_type {
        return Err(crate::response_axum::api_forbidden(
            "Get raw state value",
            "Only BCS is supported as an AcceptType.",
        ));
    }
    context.check_api_output_enabled::<AptosErrorResponse>("Get raw state value", &accept_type)?;
    use crate::context::api_spawn_blocking;
    let ctx = context.clone();
    let at = accept_type.clone();
    let lv = query.ledger_version;
    let resp =
        api_spawn_blocking(move || crate::state::raw_value_inner(&ctx, &at, body, lv.map(|v| v.0)))
            .await?;
    Ok(resp.into_response())
}

// ---- TransactionsApi ----

pub async fn get_transactions_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    Query(query): Query<PaginationQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::{context::api_spawn_blocking, page::Page};
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_transactions")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Get transactions", &accept_type)?;
    let page = Page::new(
        query.start.map(|v| v.0),
        query.limit,
        context.max_transactions_page_size(),
    );
    let ctx = context.clone();
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let api = crate::transactions::TransactionsApi { context: ctx };
        api.list(&at, page)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_transaction_by_hash_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(txn_hash): AptosPath<aptos_api_types::HashValue>,
) -> Result<Response, AptosErrorResponse> {
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_transaction_by_hash")?;
    context
        .check_api_output_enabled::<AptosErrorResponse>("Get transactions by hash", &accept_type)?;
    let txn_api = crate::transactions::TransactionsApi {
        context: context.clone(),
    };
    let resp = txn_api
        .get_transaction_by_hash_inner(&accept_type, txn_hash)
        .await?;
    Ok(resp.into_response())
}

pub async fn wait_transaction_by_hash_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(txn_hash): AptosPath<aptos_api_types::HashValue>,
) -> Result<Response, AptosErrorResponse> {
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_wait_transaction_by_hash")?;
    context
        .check_api_output_enabled::<AptosErrorResponse>("Get transactions by hash", &accept_type)?;

    if context
        .wait_for_hash_active_connections
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        >= context.node_config.api.wait_by_hash_max_active_connections
    {
        context
            .wait_for_hash_active_connections
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        crate::metrics::WAIT_TRANSACTION_POLL_TIME
            .with_label_values(&["short"])
            .observe(0.0);
        let txn_api = crate::transactions::TransactionsApi {
            context: context.clone(),
        };
        let resp = txn_api
            .get_transaction_by_hash_inner(&accept_type, txn_hash)
            .await?;
        return Ok(resp.into_response());
    }

    let start_time = std::time::Instant::now();
    crate::metrics::WAIT_TRANSACTION_GAUGE.inc();

    let txn_api = crate::transactions::TransactionsApi {
        context: context.clone(),
    };
    let result = txn_api
        .wait_transaction_by_hash_inner(
            &accept_type,
            txn_hash,
            context.node_config.api.wait_by_hash_timeout_ms,
            context.node_config.api.wait_by_hash_poll_interval_ms,
        )
        .await;

    crate::metrics::WAIT_TRANSACTION_GAUGE.dec();
    context
        .wait_for_hash_active_connections
        .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    crate::metrics::WAIT_TRANSACTION_POLL_TIME
        .with_label_values(&["long"])
        .observe(start_time.elapsed().as_secs_f64());

    let resp = result?;
    Ok(resp.into_response())
}

pub async fn get_transaction_by_version_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(txn_version): AptosPath<aptos_api_types::U64>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_transaction_by_version")?;
    context.check_api_output_enabled::<AptosErrorResponse>(
        "Get transactions by version",
        &accept_type,
    )?;
    let ctx = context.clone();
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let api = crate::transactions::TransactionsApi { context: ctx };
        api.get_transaction_by_version_inner(&at, txn_version)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_transactions_auxiliary_info_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    Query(query): Query<TxnAuxInfoQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::{context::api_spawn_blocking, page::Page};
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_transactions_auxiliary_info")?;
    context.check_api_output_enabled::<AptosErrorResponse>(
        "Get transactions auxiliary info",
        &accept_type,
    )?;
    let page = Page::new(
        query.start_version.map(|v| v.0),
        query.limit,
        context.max_transactions_page_size(),
    );
    let ctx = context.clone();
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let api = crate::transactions::TransactionsApi { context: ctx };
        api.list_auxiliary_infos(&at, page)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_accounts_transactions_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(address): AptosPath<aptos_api_types::Address>,
    Query(query): Query<PaginationQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::{context::api_spawn_blocking, page::Page};
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_get_accounts_transactions")?;
    context
        .check_api_output_enabled::<AptosErrorResponse>("Get account transactions", &accept_type)?;
    let page = Page::new(
        query.start.map(|v| v.0),
        query.limit,
        context.max_transactions_page_size(),
    );
    let ctx = context.clone();
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let api = crate::transactions::TransactionsApi { context: ctx };
        api.list_ordered_txns_by_account(&at, page, address)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn get_accounts_transaction_summaries_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    AptosPath(address): AptosPath<aptos_api_types::Address>,
    Query(query): Query<TxnSummaryQuery>,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    use std::cmp::min;
    crate::failpoint::fail_point::<AptosErrorResponse>(
        "endpoint_get_accounts_transaction_summaries",
    )?;
    context.check_api_output_enabled::<AptosErrorResponse>(
        "Get account transaction summaries",
        &accept_type,
    )?;
    let limit = if let Some(limit) = query.limit {
        min(limit, context.max_transactions_page_size())
    } else {
        context.max_transactions_page_size()
    };
    let ctx = context.clone();
    let at = accept_type.clone();
    let start_version = query.start_version;
    let end_version = query.end_version;
    let resp = api_spawn_blocking(move || {
        let api = crate::transactions::TransactionsApi { context: ctx };
        api.list_txn_summaries_by_account(&at, address, start_version, end_version, limit)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn submit_transaction_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AptosErrorResponse> {
    use aptos_api_types::VerifyInput;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_submit_transaction")?;
    if !context.node_config.api.transaction_submission_enabled {
        return Err(crate::response_axum::api_disabled("Submit transaction"));
    }
    context.check_api_output_enabled::<AptosErrorResponse>("Submit transaction", &accept_type)?;

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");
    let txn_api = crate::transactions::TransactionsApi {
        context: context.clone(),
    };
    let ledger_info: aptos_api_types::LedgerInfo =
        context.get_latest_ledger_info::<AptosErrorResponse>()?;

    let data = if content_type.contains("application/x.aptos.signed_transaction+bcs") {
        crate::transactions::SubmitTransactionPost::Bcs(body.to_vec())
    } else if content_type.contains("application/json") || content_type.is_empty() {
        let req: aptos_api_types::SubmitTransactionRequest = serde_json::from_slice(&body)
            .map_err(|e| {
                AptosErrorResponse::new(
                    StatusCode::BAD_REQUEST,
                    aptos_api_types::AptosError::new_with_error_code(
                        format!("parse request payload error: {}", e),
                        aptos_api_types::AptosErrorCode::WebFrameworkError,
                    ),
                    None,
                )
            })?;
        crate::transactions::SubmitTransactionPost::Json(req)
    } else {
        return Err(unsupported_content_type(content_type));
    };
    data.verify().map_err(|err| {
        AptosErrorResponse::bad_request(err, aptos_api_types::AptosErrorCode::InvalidInput, None)
    })?;
    let signed_txn = txn_api.get_signed_transaction(&ledger_info, data)?;
    let resp = txn_api
        .create(&accept_type, &ledger_info, signed_txn)
        .await?;
    Ok(resp.into_response())
}

pub async fn submit_transactions_batch_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AptosErrorResponse> {
    use aptos_api_types::VerifyInput;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_submit_transactions_batch")?;
    if !context.node_config.api.transaction_submission_enabled {
        return Err(crate::response_axum::api_disabled(
            "Submit transactions batch",
        ));
    }
    context.check_api_output_enabled::<AptosErrorResponse>(
        "Submit transactions batch",
        &accept_type,
    )?;

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");
    let txn_api = crate::transactions::TransactionsApi {
        context: context.clone(),
    };
    let ledger_info: aptos_api_types::LedgerInfo =
        context.get_latest_ledger_info::<AptosErrorResponse>()?;

    let data = if content_type.contains("application/x.aptos.signed_transaction+bcs") {
        crate::transactions::SubmitTransactionsBatchPost::Bcs(body.to_vec())
    } else if content_type.contains("application/json") || content_type.is_empty() {
        let reqs: Vec<aptos_api_types::SubmitTransactionRequest> = serde_json::from_slice(&body)
            .map_err(|e| {
                AptosErrorResponse::new(
                    StatusCode::BAD_REQUEST,
                    aptos_api_types::AptosError::new_with_error_code(
                        format!("parse request payload error: {}", e),
                        aptos_api_types::AptosErrorCode::WebFrameworkError,
                    ),
                    None,
                )
            })?;
        crate::transactions::SubmitTransactionsBatchPost::Json(reqs)
    } else {
        return Err(unsupported_content_type(content_type));
    };
    data.verify().map_err(|err| {
        AptosErrorResponse::bad_request(err, aptos_api_types::AptosErrorCode::InvalidInput, None)
    })?;
    let signed_txns = txn_api.get_signed_transactions_batch(&ledger_info, data)?;
    let resp = txn_api
        .create_batch(&accept_type, &ledger_info, signed_txns)
        .await?;
    Ok(resp.into_response())
}

pub async fn simulate_transaction_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AptosErrorResponse> {
    use aptos_api_types::VerifyInput;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_simulate_transaction")?;
    if !context.node_config.api.transaction_simulation_enabled {
        return Err(crate::response_axum::api_disabled("Simulate transaction"));
    }
    context.check_api_output_enabled::<AptosErrorResponse>("Simulate transaction", &accept_type)?;

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");
    let txn_api = crate::transactions::TransactionsApi {
        context: context.clone(),
    };
    let ledger_info: aptos_api_types::LedgerInfo =
        context.get_latest_ledger_info::<AptosErrorResponse>()?;

    let data = if content_type.contains("application/x.aptos.signed_transaction+bcs") {
        crate::transactions::SubmitTransactionPost::Bcs(body.to_vec())
    } else if content_type.contains("application/json") || content_type.is_empty() {
        let req: aptos_api_types::SubmitTransactionRequest = serde_json::from_slice(&body)
            .map_err(|e| {
                AptosErrorResponse::new(
                    StatusCode::BAD_REQUEST,
                    aptos_api_types::AptosError::new_with_error_code(
                        format!("parse request payload error: {}", e),
                        aptos_api_types::AptosErrorCode::WebFrameworkError,
                    ),
                    None,
                )
            })?;
        crate::transactions::SubmitTransactionPost::Json(req)
    } else {
        return Err(unsupported_content_type(content_type));
    };
    data.verify().map_err(|err| {
        AptosErrorResponse::bad_request(err, aptos_api_types::AptosErrorCode::InvalidInput, None)
    })?;
    let signed_txn = txn_api.get_signed_transaction(&ledger_info, data)?;
    let resp = txn_api.simulate(&accept_type, ledger_info, signed_txn)?;
    Ok(resp.into_response())
}

pub async fn encode_submission_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    headers: HeaderMap,
    raw_body: Bytes,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");
    if !content_type.contains("application/json") && !content_type.is_empty() {
        return Err(unsupported_content_type(content_type));
    }
    let request: aptos_api_types::EncodeSubmissionRequest = serde_json::from_slice(&raw_body)
        .map_err(|e| {
            AptosErrorResponse::new(
                StatusCode::BAD_REQUEST,
                aptos_api_types::AptosError::new_with_error_code(
                    format!("parse request payload error: {}", e),
                    aptos_api_types::AptosErrorCode::WebFrameworkError,
                ),
                None,
            )
        })?;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_encode_submission")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Encode submission", &accept_type)?;
    let ctx = context.clone();
    let at = accept_type.clone();
    let resp = api_spawn_blocking(move || {
        let api = crate::transactions::TransactionsApi { context: ctx };
        api.get_signing_message(&at, request)
    })
    .await?;
    Ok(resp.into_response())
}

pub async fn estimate_gas_price_handler(
    State(context): Ctx,
    accept_type: AcceptType,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_estimate_gas_price")?;
    context.check_api_output_enabled::<AptosErrorResponse>("Estimate gas price", &accept_type)?;
    let context = context.clone();
    let resp = api_spawn_blocking(move || {
        crate::transactions::estimate_gas_price_inner(&context, &accept_type)
    })
    .await?;
    Ok(resp.into_response())
}

// ---- ViewFunctionApi ----

pub async fn view_function_handler(
    State(context): Ctx,
    accept_type: AcceptType,
    Query(query): Query<LedgerVersionQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AptosErrorResponse> {
    use crate::context::api_spawn_blocking;
    crate::failpoint::fail_point::<AptosErrorResponse>("endpoint_view_function")?;
    context.check_api_output_enabled::<AptosErrorResponse>("View function", &accept_type)?;

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let request = if content_type.contains("application/x.aptos.view_function+bcs") {
        crate::view_function::ViewFunctionRequest::Bcs(body.to_vec())
    } else {
        let view_request: aptos_api_types::ViewRequest =
            serde_json::from_slice(&body).map_err(|e| {
                AptosErrorResponse::new(
                    StatusCode::BAD_REQUEST,
                    aptos_api_types::AptosError::new_with_error_code(
                        format!("parse request payload error: {}", e),
                        aptos_api_types::AptosErrorCode::WebFrameworkError,
                    ),
                    None,
                )
            })?;
        crate::view_function::ViewFunctionRequest::Json(view_request)
    };

    let lv = query.ledger_version;
    let resp = api_spawn_blocking(move || {
        crate::view_function::view_request_inner(context, accept_type, request, lv)
    })
    .await?;
    Ok(resp.into_response())
}

// ---- Failpoints ----

pub async fn set_failpoint_handler(
    State(context): Ctx,
    Query(query): Query<crate::set_failpoints::FailpointConf>,
) -> Result<String, AptosErrorResponse> {
    #[cfg(feature = "failpoints")]
    {
        if context.failpoints_enabled() {
            fail::cfg(&query.name, &query.actions).map_err(|e| {
                AptosErrorResponse::internal(
                    format!("{}", e),
                    aptos_api_types::AptosErrorCode::InternalError,
                    None,
                )
            })?;
            Ok(format!("Set failpoint {}", query.name))
        } else {
            Err(AptosErrorResponse::internal(
                "Failpoints are not enabled at a config level",
                aptos_api_types::AptosErrorCode::InternalError,
                None,
            ))
        }
    }
    #[cfg(not(feature = "failpoints"))]
    {
        let _ = (context, query);
        Err(AptosErrorResponse::internal(
            "Failpoints are not enabled at a feature level",
            aptos_api_types::AptosErrorCode::InternalError,
            None,
        ))
    }
}
