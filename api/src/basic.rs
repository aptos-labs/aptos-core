// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    context::Context,
    response_axum::{AptosErrorResponse, AptosResponse},
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::AptosErrorCode;
use serde::{Deserialize, Serialize};
use std::{
    ops::Sub,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Representation of a successful healthcheck
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HealthCheckSuccess {
    message: String,
}

impl HealthCheckSuccess {
    pub fn new() -> Self {
        Self {
            message: "aptos-node:ok".to_string(),
        }
    }
}

/// Framework-agnostic business logic for the healthy endpoint.
/// Called by the Axum handler directly, bypassing the Poem bridge.
pub fn healthy_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    duration_secs: Option<u32>,
) -> Result<AptosResponse<HealthCheckSuccess>, AptosErrorResponse> {
    let ledger_info = context.get_latest_ledger_info::<AptosErrorResponse>()?;

    // If we have a duration, check that it's close to the current time, otherwise it's ok
    if let Some(max_skew) = duration_secs {
        let ledger_timestamp = Duration::from_micros(ledger_info.timestamp());
        let skew_threshold = SystemTime::now()
            .sub(Duration::from_secs(max_skew as u64))
            .duration_since(UNIX_EPOCH)
            .context("Failed to determine absolute unix time based on given duration")
            .map_err(|err| {
                AptosErrorResponse::internal(err, AptosErrorCode::InternalError, Some(&ledger_info))
            })?;

        if ledger_timestamp < skew_threshold {
            return Err(AptosErrorResponse::service_unavailable(
                format!(
                    "The latest ledger info timestamp is {:?}, which is beyond the allowed skew ({}s).",
                    ledger_timestamp, max_skew
                ),
                AptosErrorCode::HealthCheckFailed,
                Some(&ledger_info),
            ));
        }
    }
    AptosResponse::try_from_rust_value(HealthCheckSuccess::new(), &ledger_info, accept_type)
}
