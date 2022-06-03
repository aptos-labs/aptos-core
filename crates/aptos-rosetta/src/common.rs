// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{error::ApiError, types::NetworkIdentifier, Network, RosettaContext};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, future::Future, str::FromStr};
use warp::Filter;

pub const BLOCKCHAIN: &str = "aptos";

/// Checks the request network matches the server network
pub fn check_network(
    network_identifier: NetworkIdentifier,
    server_context: &RosettaContext,
) -> Result<(), ApiError> {
    if network_identifier.blockchain == BLOCKCHAIN
        || Network::from_str(network_identifier.network.trim())? == server_context.network
    {
        Ok(())
    } else {
        Err(ApiError::BadNetwork)
    }
}

/// Attaches RosettaContext to warp paths
pub fn with_context(
    context: RosettaContext,
) -> impl Filter<Extract = (RosettaContext,), Error = Infallible> + Clone {
    warp::any().map(move || context.clone())
}

/// Handles a generic request to warp
pub fn handle_request<'a, F, R, Req, Resp>(
    handler: F,
) -> impl Fn(
    Req,
    RosettaContext,
) -> BoxFuture<'static, Result<warp::reply::WithStatus<warp::reply::Json>, Infallible>>
       + Clone
where
    F: FnOnce(Req, RosettaContext) -> R + Clone + Copy + Send + 'static,
    R: Future<Output = Result<Resp, ApiError>> + Send,
    Req: Deserialize<'a> + Send + 'static,
    Resp: Serialize,
{
    move |request, options| {
        let fut = async move {
            match handler(request, options).await {
                Ok(response) => Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    warp::http::StatusCode::OK,
                )),
                Err(api_error) => {
                    let status = api_error.status_code();
                    Ok(warp::reply::with_status(
                        warp::reply::json(&api_error.into_error()),
                        status,
                    ))
                }
            }
        };
        Box::pin(fut)
    }
}
