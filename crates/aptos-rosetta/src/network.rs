// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{check_network, get_timestamp, handle_request, with_context, with_empty_request},
    error::ApiError,
    types::{
        Allow, BlockIdentifier, MetadataRequest, NetworkListResponse, NetworkOptionsResponse,
        NetworkRequest, NetworkStatusResponse, OperationStatusType, OperationType, Peer, Version,
    },
    RosettaContext, MIDDLEWARE_VERSION, NODE_VERSION, ROSETTA_VERSION,
};
use aptos_logger::{debug, trace};
use std::time::Duration;
use warp::Filter;

pub fn list_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("network" / "list")
        .and(warp::post())
        .and(with_empty_request())
        .and(with_context(server_context))
        .and_then(handle_request(network_list))
}

pub fn options_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("network" / "options")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(network_options))
}

pub fn status_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("network" / "status")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(network_status))
}

/// List [`NetworkIdentifier`]s supported by this proxy aka [`ChainId`]s
///
/// This should be able to run without a running full node connection
///
/// [API Spec](https://www.rosetta-api.org/docs/NetworkApi.html#networklist)
async fn network_list(
    _empty: MetadataRequest,
    server_context: RosettaContext,
) -> Result<NetworkListResponse, ApiError> {
    debug!("/network/list");
    trace!(
        server_context = ?server_context,
        "network_list",
    );

    let response = NetworkListResponse {
        network_identifiers: vec![server_context.chain_id.into()],
    };

    Ok(response)
}

/// Get Network options
///
/// This lists out all errors, operations, and statuses, along with versioning information.
/// This should be able to run without a running full node connection
///
/// [API Spec](https://www.rosetta-api.org/docs/NetworkApi.html#networkoptions)
async fn network_options(
    request: NetworkRequest,
    server_context: RosettaContext,
) -> Result<NetworkOptionsResponse, ApiError> {
    debug!("/network/options");
    trace!(
        request = ?request,
        server_context = ?server_context,
        "network_options",
    );

    check_network(request.network_identifier, &server_context)?;

    let version = Version {
        rosetta_version: ROSETTA_VERSION.to_string(),
        // TODO: Get from node via REST API
        node_version: NODE_VERSION.to_string(),
        // TODO: Get from the binary directly
        middleware_version: MIDDLEWARE_VERSION.to_string(),
    };

    let operation_statuses = OperationStatusType::all()
        .into_iter()
        .map(|status| status.into())
        .collect();
    let operation_types = OperationType::all()
        .into_iter()
        .map(|op| op.to_string())
        .collect();
    let errors = ApiError::all()
        .into_iter()
        .map(|err| err.into_error())
        .collect();

    let allow = Allow {
        operation_statuses,
        operation_types,
        errors,
        historical_balance_lookup: true,
        timestamp_start_index: None,
        call_methods: vec![],
        balance_exemptions: vec![],
        mempool_coins: false,
        block_hash_case: None,
        transaction_hash_case: None,
    };

    let response = NetworkOptionsResponse { version, allow };

    Ok(response)
}

/// Get network status including the latest state
///
/// This should respond with the latest ledger version, timestamp, and genesis information
///
/// [API Spec](https://www.rosetta-api.org/docs/NetworkApi.html#networkoptions)
async fn network_status(
    request: NetworkRequest,
    server_context: RosettaContext,
) -> Result<NetworkStatusResponse, ApiError> {
    debug!("/network/status");
    trace!(
        request = ?request,
        server_context = ?server_context,
        "network_status",
    );

    check_network(request.network_identifier, &server_context)?;
    let rest_client = server_context.rest_client()?;
    let block_cache = server_context.block_cache()?;
    let genesis_block_info = block_cache.get_block_info(0).await?;
    let genesis_block_identifier = BlockIdentifier::from_block_info(genesis_block_info);
    let response = rest_client.get_ledger_information().await?;
    let state = response.state();

    // Get the latest block (but be one behind)
    let latest_version = state.version;
    let mut block_info = None;
    // Try for 10 times to get the latest block
    // TODO: Improve this performance
    for _ in 1..10 {
        if let Ok(info) = block_cache.get_block_info_by_version(latest_version).await {
            block_info = Some(info);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // If we don't have the block info, fail
    let block_info = if let Some(block_info) = block_info {
        block_info
    } else {
        return Err(ApiError::BlockIncomplete);
    };

    let block_info = block_cache
        .get_block_info(block_info.block_height - 1)
        .await?;
    let current_block_identifier = BlockIdentifier::from_block_info(block_info);
    let current_block_timestamp = get_timestamp(block_info);

    let oldest_block_identifier = if let Some(version) = state.oldest_ledger_version {
        let block_info = block_cache.get_block_info_by_version(version).await?;

        Some(BlockIdentifier::from_block_info(block_info))
    } else {
        None
    };

    // TODO: add peers
    let peers: Vec<Peer> = vec![];

    let response = NetworkStatusResponse {
        current_block_identifier,
        current_block_timestamp,
        genesis_block_identifier,
        oldest_block_identifier,
        sync_status: None,
        peers,
    };

    Ok(response)
}
