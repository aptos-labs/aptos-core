// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    error::V2Error,
    types::V2Response,
};
use aptos_api_types::GasEstimation;
use axum::{extract::State, Json};

/// GET /v2/estimate_gas_price -- estimate current gas unit price.
///
/// Returns deprioritized, regular, and prioritized gas price estimates.
#[utoipa::path(
    get,
    path = "/v2/estimate_gas_price",
    tag = "Transactions",
    responses(
        (status = 200, description = "Gas price estimation", body = Object),
        (status = 500, description = "Internal error", body = V2Error),
    )
)]
pub async fn estimate_gas_price_handler(
    State(ctx): State<V2Context>,
) -> Result<Json<V2Response<GasEstimation>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let ledger_info = ctx.ledger_info()?;
        let gas_estimation = ctx
            .inner()
            .estimate_gas_price(&ledger_info)
            .map_err(|e: crate::response::BasicError| V2Error::internal(format!("{}", e)))?;

        Ok(Json(V2Response::new(gas_estimation, &ledger_info)))
    })
    .await
}
