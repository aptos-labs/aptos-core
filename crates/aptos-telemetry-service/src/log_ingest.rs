// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{auth::with_auth, context::Context, types::auth::Claims};
use aptos_config::config::PeerRole;
use aptos_logger::error;
use bytes::{Buf, Bytes};
use flate2::read::GzDecoder;
use std::io::Read;
use warp::{filters::BoxedFilter, reply, Filter, Rejection, Reply};

pub fn log_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("log_ingest")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(
            context,
            vec![PeerRole::Validator, PeerRole::Unknown],
        ))
        .and(warp::body::bytes())
        .and_then(handle_log_ingest)
        .boxed()
}

pub async fn handle_log_ingest(
    _context: Context,
    claims: Claims,
    bytes: Bytes,
) -> anyhow::Result<impl Reply, Rejection> {
    let mut decoder = GzDecoder::new(bytes.reader());

    let mut s = String::new();
    decoder.read_to_string(&mut s).unwrap();

    error!("BCHO handle_log_ingest {} from {}", s, claims.peer_id);

    Ok(reply::reply())
}
