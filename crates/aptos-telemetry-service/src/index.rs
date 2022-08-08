// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::convert::Infallible;

use crate::{auth, context::Context, error};
use warp::{Filter, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    auth::auth(context).recover(error::handle_rejection)
}
