// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{accounts, context::Context, log, transactions};
use diem_api_types::{Error, Response};

use std::convert::Infallible;
use warp::{http::StatusCode, reply, Filter, Rejection, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    index(context.clone())
        .or(accounts::routes(context.clone()))
        .or(transactions::routes(context))
        .recover(handle_rejection)
        .with(log::logger())
}

// GET /
pub fn index(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::end()
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_index)
}

pub async fn handle_index(context: Context) -> Result<impl Reply, Rejection> {
    let info = context.get_latest_ledger_info()?;
    Ok(Response::new(info.clone(), &info)?)
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let body;
    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        body = reply::json(&Error::new(code, "Not Found".to_owned()));
    } else if let Some(error) = err.find::<Error>() {
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
    use crate::test_utils::new_test_context;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_ledger_info() {
        let context = new_test_context();
        let ledger_info = context.get_latest_ledger_info();
        let resp = context.get("/").await;

        let expected = json!({
            "chain_id": 4,
            "ledger_version": ledger_info.version().to_string(),
            "ledger_timestamp": ledger_info.timestamp().to_string(),
        });

        assert_eq!(expected, resp);
    }

    #[tokio::test]
    async fn test_returns_not_found_for_the_invalid_path() {
        let context = new_test_context();
        let resp = context.expect_status_code(404).get("/invalid_path").await;
        assert_eq!(json!({"code": 404, "message": "Not Found"}), resp)
    }
}
