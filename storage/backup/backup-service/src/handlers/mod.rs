// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod bytes_sender;
mod utils;

use crate::handlers::utils::{
    reply_with_bcs_bytes, reply_with_bytes_sender, unwrap_or_500, LATENCY_HISTOGRAM,
};
use axum::{
    extract::{Path, State},
    http::Uri,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use aptos_crypto::hash::HashValue;
use aptos_db::backup::backup_handler::BackupHandler;
use aptos_metrics_core::TimerHelper;
use aptos_types::transaction::Version;
use std::time::Instant;

static DB_STATE: &str = "db_state";
static STATE_RANGE_PROOF: &str = "state_range_proof";
static STATE_SNAPSHOT: &str = "state_snapshot";
static STATE_ITEM_COUNT: &str = "state_item_count";
static STATE_SNAPSHOT_CHUNK: &str = "state_snapshot_chunk";
static STATE_ROOT_PROOF: &str = "state_root_proof";
static EPOCH_ENDING_LEDGER_INFOS: &str = "epoch_ending_ledger_infos";
static TRANSACTIONS: &str = "transactions";
static TRANSACTION_RANGE_PROOF: &str = "transaction_range_proof";
const KNOWN_ENDPOINTS: [&str; 9] = [
    DB_STATE,
    STATE_RANGE_PROOF,
    STATE_SNAPSHOT,
    STATE_ITEM_COUNT,
    STATE_SNAPSHOT_CHUNK,
    STATE_ROOT_PROOF,
    EPOCH_ENDING_LEDGER_INFOS,
    TRANSACTIONS,
    TRANSACTION_RANGE_PROOF,
];

#[derive(Clone)]
struct BackupState {
    backup_handler: BackupHandler,
}

pub(crate) fn get_routes(backup_handler: BackupHandler) -> Router {
    let state = BackupState { backup_handler };

    Router::new()
        .route(&format!("/{DB_STATE}"), get(get_db_state))
        .route(
            &format!("/{STATE_RANGE_PROOF}/:version/:end_key"),
            get(get_state_range_proof),
        )
        .route(&format!("/{STATE_SNAPSHOT}/:version"), get(get_state_snapshot))
        .route(
            &format!("/{STATE_ITEM_COUNT}/:version"),
            get(get_state_item_count),
        )
        .route(
            &format!("/{STATE_SNAPSHOT_CHUNK}/:version/:start_idx/:limit"),
            get(get_state_snapshot_chunk),
        )
        .route(
            &format!("/{STATE_ROOT_PROOF}/:version"),
            get(get_state_root_proof),
        )
        .route(
            &format!("/{EPOCH_ENDING_LEDGER_INFOS}/:start_epoch/:end_epoch"),
            get(get_epoch_ending_ledger_infos),
        )
        .route(
            &format!("/{TRANSACTIONS}/:start_version/:num_transactions"),
            get(get_transactions),
        )
        .route(
            &format!("/{TRANSACTION_RANGE_PROOF}/:first_version/:last_version"),
            get(get_transaction_range_proof),
        )
        .with_state(state)
        .fallback(fallback_handler)
        .layer(middleware::from_fn(track_latency))
}

async fn fallback_handler(uri: Uri) -> StatusCode {
    let endpoint = uri.path().split('/').nth(1).unwrap_or("");
    if KNOWN_ENDPOINTS.contains(&endpoint) {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn track_latency(
    request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let path = request.uri().path().to_owned();
    let endpoint = path.split('/').nth(1).unwrap_or("-").to_owned();
    let start = Instant::now();
    let response = next.run(request).await;
    LATENCY_HISTOGRAM.observe_with(
        &[endpoint.as_str(), response.status().as_str()],
        start.elapsed().as_secs_f64(),
    );
    response
}

async fn get_db_state(State(state): State<BackupState>) -> Response {
    unwrap_or_500(
        state
            .backup_handler
            .get_db_state()
            .and_then(|db_state| reply_with_bcs_bytes(DB_STATE, &db_state)),
    )
}

async fn get_state_range_proof(
    State(state): State<BackupState>,
    Path((version, end_key)): Path<(Version, String)>,
) -> Response {
    match HashValue::from_hex(end_key.as_str()) {
        Ok(end_key) => unwrap_or_500(
            state
                .backup_handler
                .get_account_state_range_proof(end_key, version)
                .and_then(|proof| reply_with_bcs_bytes(STATE_RANGE_PROOF, &proof)),
        ),
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn get_state_snapshot(
    State(state): State<BackupState>,
    Path(version): Path<Version>,
) -> Response {
    reply_with_bytes_sender(&state.backup_handler, STATE_SNAPSHOT, move |bh, sender| {
        bh.get_state_item_iter(version, 0, usize::MAX)?
            .try_for_each(|record_res| sender.send_size_prefixed_bcs_bytes(record_res?))
    })
}

async fn get_state_item_count(
    State(state): State<BackupState>,
    Path(version): Path<Version>,
) -> Response {
    unwrap_or_500(
        state
            .backup_handler
            .get_state_item_count(version)
            .map(|value| value as u64)
            .and_then(|count| reply_with_bcs_bytes(STATE_ITEM_COUNT, &count)),
    )
}

async fn get_state_snapshot_chunk(
    State(state): State<BackupState>,
    Path((version, start_idx, limit)): Path<(Version, usize, usize)>,
) -> Response {
    reply_with_bytes_sender(
        &state.backup_handler,
        STATE_SNAPSHOT_CHUNK,
        move |bh, sender| {
            bh.get_state_item_iter(version, start_idx, limit)?
                .try_for_each(|record_res| sender.send_size_prefixed_bcs_bytes(record_res?))
        },
    )
}

async fn get_state_root_proof(
    State(state): State<BackupState>,
    Path(version): Path<Version>,
) -> Response {
    unwrap_or_500(
        state
            .backup_handler
            .get_state_root_proof(version)
            .and_then(|proof| reply_with_bcs_bytes(STATE_ROOT_PROOF, &proof)),
    )
}

async fn get_epoch_ending_ledger_infos(
    State(state): State<BackupState>,
    Path((start_epoch, end_epoch)): Path<(u64, u64)>,
) -> Response {
    reply_with_bytes_sender(
        &state.backup_handler,
        EPOCH_ENDING_LEDGER_INFOS,
        move |bh, sender| {
            bh.get_epoch_ending_ledger_info_iter(start_epoch, end_epoch)?
                .try_for_each(|record_res| sender.send_size_prefixed_bcs_bytes(record_res?))
        },
    )
}

async fn get_transactions(
    State(state): State<BackupState>,
    Path((start_version, num_transactions)): Path<(Version, usize)>,
) -> Response {
    reply_with_bytes_sender(&state.backup_handler, TRANSACTIONS, move |bh, sender| {
        bh.get_transaction_iter(start_version, num_transactions)?
            .try_for_each(|record_res| sender.send_size_prefixed_bcs_bytes(record_res?))
    })
}

async fn get_transaction_range_proof(
    State(state): State<BackupState>,
    Path((first_version, last_version)): Path<(Version, Version)>,
) -> Response {
    unwrap_or_500(
        state
            .backup_handler
            .get_transaction_range_proof(first_version, last_version)
            .and_then(|proof| reply_with_bcs_bytes(TRANSACTION_RANGE_PROOF, &proof)),
    )
}
