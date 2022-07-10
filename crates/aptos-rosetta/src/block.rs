// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{check_network, get_block_index_from_request, handle_request, with_context},
    error::{ApiError, ApiResult},
    types::{Block, BlockIdentifier, BlockRequest, BlockResponse, Transaction},
    RosettaContext,
};
use aptos_logger::{debug, trace};
use warp::Filter;

/// The year 2000 in seconds, as this is the lower limit for Rosetta API implementations
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
    let block_size = server_context.block_size;

    // Retrieve by block or by hash, both or neither is not allowed
    let block_index =
        get_block_index_from_request(rest_client, Some(request.block_identifier), block_size)
            .await?;

    let (parent_transaction, transactions): (
        aptos_rest_client::Transaction,
        Vec<aptos_rest_client::Transaction>,
    ) = get_block_by_index(rest_client, block_size, block_index).await?;

    let block = build_block(server_context, parent_transaction, transactions).await?;

    Ok(BlockResponse {
        block: Some(block),
        other_transactions: None,
    })
}

/// Build up the transaction, which should contain the `operations` as the change set
async fn build_block(
    server_context: RosettaContext,
    parent_transaction: aptos_rest_client::Transaction,
    transactions: Vec<aptos_rest_client::Transaction>,
) -> ApiResult<Block> {
    let (block_identifier, timestamp) =
        get_block_id_and_timestamp(server_context.block_size, &transactions)?;

    // Convert the transactions and build the block
    let mut txns: Vec<Transaction> = Vec::new();
    for txn in transactions {
        txns.push(
            Transaction::from_transaction(
                server_context.coin_cache.clone(),
                server_context.rest_client()?,
                txn,
            )
            .await?,
        )
    }

    Ok(Block {
        block_identifier,
        parent_block_identifier: BlockIdentifier::from_transaction(
            server_context.block_size,
            &parent_transaction,
        )?,
        timestamp,
        transactions: txns,
    })
}

/// Retrieves the block id and the timestamp from the first transaction in the block
fn get_block_id_and_timestamp(
    block_size: u64,
    transactions: &[aptos_rest_client::Transaction],
) -> ApiResult<(BlockIdentifier, u64)> {
    if let Some(first) = transactions.first() {
        // note: timestamps are in microseconds, so we convert to milliseconds
        let mut timestamp = first.timestamp() / 1000;

        // Rosetta doesn't like timestamps before 2000
        if timestamp < Y2K_SECS {
            timestamp = Y2K_SECS;
        }
        Ok((
            BlockIdentifier::from_transaction(block_size, first)?,
            timestamp,
        ))
    } else {
        Err(ApiError::BlockIncomplete)
    }
}

/// Retrieves a block by its index
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
            return Err(ApiError::BlockIncomplete);
        }
        Ok((parent_txn, txns))
    }
}

/// Converts block index to its associated version
pub fn block_index_to_version(block_size: u64, block_index: u64) -> u64 {
    if block_index == 0 {
        0
    } else {
        ((block_index - 1) * block_size) + 1
    }
}

/// Converts ledger version to its associated block index
pub fn version_to_block_index(block_size: u64, version: u64) -> u64 {
    if version == 0 {
        0
    } else {
        ((version - 1) / block_size) + 1
    }
}
