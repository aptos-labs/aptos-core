// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use crate::{
    common::{handle_request, with_context},
    RosettaContext,
};
use warp::Filter;

pub fn combine_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "combine")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_combine))
}

pub fn derive_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "derive")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_derive))
}

pub fn hash_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "hash")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_hash))
}

pub fn metadata_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "metadata")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_metadata))
}

pub fn parse_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "parse")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_parse))
}

pub fn payloads_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "payloads")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_payloads))
}

pub fn preprocess_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "preprocess")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_preprocess))
}

pub fn submit_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "submit")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_submit))
}
