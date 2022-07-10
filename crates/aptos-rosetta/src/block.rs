// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{check_network, handle_request, strip_hex_prefix, with_context},
    error::{ApiError, ApiResult},
    types::{Block, BlockIdentifier, BlockRequest, BlockResponse, Transaction},
    RosettaContext,
};
use aptos_crypto::HashValue;
use aptos_logger::{debug, trace};
use std::str::FromStr;
use warp::Filter;

const Y2K_SECS: u64 = 946713600000;

pub fn block_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("block")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(block))
}

/// Retrieves a block (in this case a single transaction) given it's identifier.
///
/// Our implementation allows for by `index`, which is the ledger `version` or by
/// transaction `hash`.
///
/// [API Spec](https://www.rosetta-api.org/docs/BlockApi.html#block)
async fn block(request: BlockRequest, server_context: RosettaContext) -> ApiResult<BlockResponse> {
    debug!("/block");
    trace!(
        request = ?request,
        server_context = ?server_context,
        "/block",
    );

    check_network(request.network_identifier, &server_context)?;

    let rest_client = server_context.rest_client()?;

    // Retrieve by block or by hash, both or neither is not allowed
    let (parent_transaction, transactions): (
        aptos_rest_client::Transaction,
        Vec<aptos_rest_client::Transaction>,
    ) = match (
        &request.block_identifier.index,
        &request.block_identifier.hash,
    ) {
        (Some(block_index), None) => {
            get_block_by_index(rest_client, server_context.block_size, *block_index).await?
        }
        (None, Some(hash)) => {
            // Allow 0x in front of hash
            let hash = HashValue::from_str(strip_hex_prefix(hash))
                .map_err(|err| ApiError::AptosError(err.to_string()))?;
            let response = rest_client.get_transaction(hash).await?;
            let txn = response.into_inner();
            let version = txn.version().unwrap();
            let block_index = version_to_block_index(server_context.block_size, version);

            get_block_by_index(rest_client, server_context.block_size, block_index).await?
        }
        (None, None) => {
            // Get current version
            let response = rest_client.get_ledger_information().await?;
            let version = response.state().version;
            let block_index = version_to_block_index(server_context.block_size, version) - 1;

            get_block_by_index(rest_client, server_context.block_size, block_index).await?
        }
        (_, _) => return Err(ApiError::BadBlockRequest),
    };

    // Build up the transaction, which should contain the `operations` as the change set
    let (block_identifier, timestamp) = if let Some(first) = transactions.first() {
        // note: timestamps are in microseconds, so we convert to milliseconds
        let mut timestamp = first.timestamp() / 1000;

        // Rosetta doesn't like timestamps before 2000
        if timestamp < Y2K_SECS {
            timestamp = Y2K_SECS;
        }
        (
            BlockIdentifier::from_transaction(server_context.block_size, first)?,
            timestamp,
        )
    } else {
        return Err(ApiError::BlockIncomplete);
    };

    let mut txns: Vec<Transaction> = Vec::new();
    for txn in transactions {
        txns.push(
            Transaction::from_transaction(server_context.coin_cache.clone(), rest_client, txn)
                .await?,
        )
    }

    let block = Block {
        block_identifier,
        parent_block_identifier: BlockIdentifier::from_transaction(
            server_context.block_size,
            &parent_transaction,
        )?,
        timestamp,
        transactions: txns,
    };

    let response = BlockResponse {
        block: Some(block),
        other_transactions: None,
    };

    Ok(response)
}

async fn get_block_by_index(
    rest_client: &aptos_rest_client::Client,
    block_size: u64,
    block_index: u64,
) -> ApiResult<(
    aptos_rest_client::Transaction,
    Vec<aptos_rest_client::Transaction>,
)> {
    let version = block_index_to_version(block_size, block_index);

    // For the genesis block, we populate parent_block_identifier with the
    // same genesis block. Refer to
    // https://www.rosetta-api.org/docs/common_mistakes.html#malformed-genesis-block
    if version == 0 {
        let response = rest_client.get_transaction_by_version(version).await?;
        let txn = response.into_inner();
        Ok((txn.clone(), vec![txn]))
    } else {
        let previous_version = block_index_to_version(block_size, block_index - 1);
        let parent_txn = rest_client
            .get_transaction_by_version(previous_version)
            .await?
            .into_inner();
        let txns = rest_client
            .get_transactions(Some(version), Some(block_size))
            .await?
            .into_inner();

        // We can't give an incomplete block, it'll have to be retried
        if txns.len() != block_size as usize {
            return Err(ApiError::BadBlockRequest);
        }
        Ok((parent_txn, txns))
    }
}

pub fn block_index_to_version(block_size: u64, block_index: u64) -> u64 {
    if block_index == 0 {
        0
    } else {
        ((block_index - 1) * block_size) + 1
    }
}

pub fn version_to_block_index(block_size: u64, version: u64) -> u64 {
    if version == 0 {
        0
    } else {
        ((version - 1) / block_size) + 1
    }
}
