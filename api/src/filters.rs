// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, handlers};

use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    index(context)
}

// GET /
pub fn index(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::end()
        .and(with_context(context))
        .and_then(handlers::index)
}

fn with_context(context: Context) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
    warp::any().map(move || context.clone())
}
