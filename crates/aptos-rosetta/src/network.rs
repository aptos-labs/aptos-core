// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::{block_index_to_version, version_to_block_index},
    common::{
        check_network, get_timestamp, handle_request, with_context, with_empty_request,
        EmptyRequest,
    },
    error::ApiError,
    types::{
        Allow, BlockIdentifier, NetworkListResponse, NetworkOptionsResponse, NetworkRequest,
        NetworkStatusResponse, OperationStatusType, OperationType, Peer, Version,
    },
    RosettaContext, MIDDLEWARE_VERSION, NODE_VERSION, ROSETTA_VERSION,
};
use aptos_logger::{debug, trace};
use warp::Filter;

pub fn routes(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(
            warp::path!("network" / "list")
                .and(with_empty_request())
                .and(with_context(server_context.clone()))
                .and_then(handle_request(network_list)),
        )
        .or(warp::path!("network" / "options")
            .and(warp::body::json())
            .and(with_context(server_context.clone()))
            .and_then(handle_request(network_options)))
        .or(warp::path!("network" / "status")
            .and(warp::body::json())
            .and(with_context(server_context))
            .and_then(handle_request(network_status)))
}

/// List [`NetworkIdentifier`]s supported by this proxy aka [`ChainId`]s
///
/// This should be able to run without a running full node connection
///
/// [API Spec](https://www.rosetta-api.org/docs/NetworkApi.html#networklist)
async fn network_list(
    _empty: EmptyRequest,
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
    let block_size = server_context.block_size;

    let rest_client = server_context.rest_client()?;
    let genesis_txn = BlockIdentifier::genesis_txn();
    let response = rest_client.get_ledger_information().await?;
    let state = response.state();

    // Get the last "block"
    let previous_block = version_to_block_index(block_size, state.version) - 1;
    let block_version = block_index_to_version(block_size, previous_block);
    let response = rest_client
        .get_transaction_by_version(block_version)
        .await?;
    let transaction = response.inner();
    let latest_txn = BlockIdentifier::from_transaction(block_size, transaction)?;

    let current_block_timestamp = get_timestamp(&response);

    let oldest_block_identifier = if let Some(mut version) = state.oldest_ledger_version {
        // For non-genesis versions we have to ensure that really the next "block" is the oldest
        if version != 0 {
            let block_index = version_to_block_index(block_size, version);
            // If the txn is the first in the block include it, otherwise, return the next block
            if block_index_to_version(block_size, block_index) != version {
                version = block_index_to_version(block_size, block_index + 1);
            }
        }

        Some(BlockIdentifier::from_transaction(
            block_size,
            rest_client
                .get_transaction_by_version(version)
                .await?
                .inner(),
        )?)
    } else {
        None
    };

    // TODO: add peers
    let peers: Vec<Peer> = vec![];

    let response = NetworkStatusResponse {
        current_block_identifier: latest_txn,
        current_block_timestamp,
        genesis_block_identifier: genesis_txn,
        oldest_block_identifier,
        sync_status: None,
        peers,
    };

    Ok(response)
}
