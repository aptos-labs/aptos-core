// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;

use diem_api_types::views;

use anyhow::{Error, Result};
use std::sync::Arc;
use warp::{reject, Rejection, Reply};

pub async fn index(context: Arc<Context>) -> Result<impl Reply, Rejection> {
    let ledger_info = context.get_latest_ledger_info().map_err(internal_error)?;
    let chain_id = context.chain_id().id();
    let ledger_version = ledger_info.ledger_info().version();
    let ledger_timestamp = ledger_info.ledger_info().timestamp_usecs();

    let info = views::LedgerInfo {
        chain_id,
        ledger_version,
        ledger_timestamp,
    };
    Ok(warp::reply::json(&info))
}

fn internal_error(err: Error) -> Rejection {
    reject::custom(views::InternalError::from(err))
}
