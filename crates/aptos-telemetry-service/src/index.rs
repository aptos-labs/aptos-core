// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{auth, context::Context, custom_event, error};
use std::convert::Infallible;
use warp::{Filter, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    auth::auth(context.clone())
        .or(custom_event::custom_event(context))
        .recover(error::handle_rejection)
}
