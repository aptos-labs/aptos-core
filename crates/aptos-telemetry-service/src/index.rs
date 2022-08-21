// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{auth, context::Context, custom_event, error::ServiceError, prometheus_push_metrics};
use std::convert::Infallible;
use warp::{
    body::BodyDeserializeError,
    filters::BoxedFilter,
    http::StatusCode,
    reject::{LengthRequired, MethodNotAllowed, PayloadTooLarge, UnsupportedMediaType},
    reply, Filter, Rejection, Reply,
};
pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    index(context.clone())
        .or(auth::auth(context.clone()))
        .or(custom_event::custom_event(context.clone()))
        .or(prometheus_push_metrics::metrics_ingest(context))
        .recover(handle_rejection)
}

fn index(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path::end()
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_index)
        .boxed()
}

async fn handle_index(context: Context) -> anyhow::Result<impl Reply, Rejection> {
    let resp = reply::json(&context.noise_config().public_key());
    Ok(resp)
}

pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let code;
    let body;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        body = reply::json(&ServiceError::new(code, "Not Found".to_owned()));
    } else if let Some(error) = err.find::<ServiceError>() {
        code = error.status_code();
        body = reply::json(error);
    } else if let Some(cause) = err.find::<BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<LengthRequired>() {
        code = StatusCode::LENGTH_REQUIRED;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<PayloadTooLarge>() {
        code = StatusCode::PAYLOAD_TOO_LARGE;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<UnsupportedMediaType>() {
        code = StatusCode::UNSUPPORTED_MEDIA_TYPE;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        body = reply::json(&ServiceError::new(
            code,
            format!("unexpected error: {:?}", err),
        ));
    }

    Ok(reply::with_status(body, code).into_response())
}
