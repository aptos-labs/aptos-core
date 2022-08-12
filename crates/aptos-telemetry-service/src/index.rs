// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{auth, context::Context, custom_event, error};
use std::convert::Infallible;
use warp::{filters::BoxedFilter, reply, Filter, Rejection, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    index(context.clone())
        .or(auth::auth(context.clone()))
        .or(custom_event::custom_event(context))
        .recover(error::handle_rejection)
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
