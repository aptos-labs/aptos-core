// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::V2Context,
    error::V2Error,
    types::{HealthResponse, LedgerMetadata, NodeInfo, V2Response},
};
use axum::{extract::State, Json};

/// GET /v2/health
#[utoipa::path(
    get,
    path = "/v2/health",
    tag = "Health",
    responses(
        (status = 200, description = "Node is healthy", body = HealthResponse),
        (status = 500, description = "Node is unhealthy", body = V2Error),
    )
)]
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
#[utoipa::path(
    get,
    path = "/v2/info",
    tag = "Health",
    responses(
        (status = 200, description = "Node info with ledger metadata", body = V2Response<NodeInfo>),
        (status = 500, description = "Internal error", body = V2Error),
    )
)]
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
