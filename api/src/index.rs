// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{accounts, context::Context, events, log, transactions};
use diem_api_types::{Error, Response};

use std::convert::Infallible;
use warp::{http::StatusCode, reject::MethodNotAllowed, reply, Filter, Rejection, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    index(context.clone())
        .or(openapi_spec())
        .or(accounts::routes(context.clone()))
        .or(transactions::routes(context.clone()))
        .or(events::routes(context.clone()))
        .or(context.health_check_route())
        // jsonrpc routes must before `recover` and after `index`
        // so that POST '/' can be handled by jsonrpc routes instead of `index` route
        .or(context.jsonrpc_routes())
        .recover(handle_rejection)
        .with(log::logger())
}

// GET /openapi.yaml
// GET /spec.html
// GET /redoc.standalone.js
pub fn openapi_spec() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let spec = warp::path!("openapi.yaml")
        .and(warp::get())
        .map(|| include_str!("static/openapi.yaml"));
    let renderer = warp::path!("redoc.standalone.js").and(warp::get()).map(|| {
        warp::http::Response::builder()
            .header("Content-Type", "text/javascript")
            .body(include_str!("static/redoc.standalone.js"))
    });
    let html = warp::path!("spec.html")
        .and(warp::get())
        .map(|| reply::html(include_str!("static/spec.html")));
    spec.or(renderer).or(html)
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
    } else if err.find::<MethodNotAllowed>().is_some() {
        code = StatusCode::BAD_REQUEST;
        body = reply::json(&Error::new(
            code,
            "Method not allowed or request body is invalid.".to_owned(),
        ));
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        body = reply::json(&Error::new(code, format!("unexpected error: {:?}", err)));
    }
    Ok(reply::with_status(body, code))
}
