// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{accounts, context::Context};
use diem_api_types::{Error, LedgerInfo};

use std::convert::Infallible;
use warp::{http::StatusCode, reply, Filter, Rejection, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    index(context.clone())
        .or(accounts::routes(context))
        .recover(handle_rejection)
}

// GET /
pub fn index(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::end()
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_index)
}

pub async fn handle_index(context: Context) -> Result<impl Reply, Rejection> {
    let ledger_info = context.get_latest_ledger_info().map_err(Error::internal)?;
    let chain_id = context.chain_id().id();
    let ledger_version = ledger_info.ledger_info().version().into();
    let ledger_timestamp = ledger_info.ledger_info().timestamp_usecs().into();

    let info = LedgerInfo {
        chain_id,
        ledger_version,
        ledger_timestamp,
    };
    Ok(warp::reply::json(&info))
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let body;
    if let Some(error) = err.find::<Error>() {
        code = error.status_code();
        body = reply::json(error);
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        body = reply::json(&Error::new(code, format!("unexpected error: {:?}", err)));
    }
    Ok(reply::with_status(body, code))
}

#[cfg(test)]
mod test {
    use crate::test_utils::{new_test_context, send_request};
    use serde_json::json;

    #[tokio::test]
    async fn test_get_ledger_info() {
        let context = new_test_context();
        let resp = send_request(context.clone(), "GET", "/", 200).await;

        let ledger_info = context.get_latest_ledger_info().unwrap();
        let expected = json!({
            "chain_id": 4,
            "ledger_version": ledger_info.ledger_info().version().to_string(),
            "ledger_timestamp": ledger_info.ledger_info().timestamp_usecs().to_string(),
        });

        assert_eq!(expected, resp);
    }
}
