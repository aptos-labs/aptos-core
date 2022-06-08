// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{check_network, get_timestamp, handle_request, with_context},
    error::{ApiError, ApiResult},
    types::{
        Block, BlockIdentifier, BlockRequest, BlockResponse, Transaction, TransactionIdentifier,
    },
    RosettaContext,
};
use aptos_crypto::HashValue;
use aptos_logger::{debug, trace};
use std::str::FromStr;
use warp::Filter;

pub fn routes(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post().and(
        warp::path!("block")
            .and(warp::body::json())
            .and(with_context(server_context))
            .and_then(handle_request(block)),
    )
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
        "block",
    );

    check_network(request.network_identifier, &server_context)?;

    let rest_client = &server_context.rest_client;

    // Retrieve by block or by hash, both or neither is not allowed
    let transaction = match (
        &request.block_identifier.index,
        &request.block_identifier.hash,
    ) {
        (Some(version), None) => rest_client
            .get_transaction_by_version(*version)
            .await?
            .into_inner(),
        (None, Some(hash)) => {
            // Allow 0x in front of hash
            let hash = HashValue::from_str(hash.strip_prefix("0x").unwrap())
                .map_err(|err| ApiError::AptosError(err.to_string()))?;
            rest_client.get_transaction(hash).await?.into_inner()
        }
        (None, None) => {
            // Get current version
            let txn = rest_client.get_transactions(None, Some(1)).await?;
            txn.into_inner().first().unwrap()
        }
        (_, _) => return Err(ApiError::BadBlockRequest),
    };

    // Build up the transaction, which should contain the `operations` as the change set
    let transaction_info = response.inner().transaction_info()?;
    let transactions = vec![Transaction {
        transaction_identifier: TransactionIdentifier {
            hash: transaction_info.hash.to_string(),
        },
        // TODO: Add operations
        operations: vec![],
        related_transactions: None,
    }];

    let block_identifier: BlockIdentifier = transaction_info.into();
    // For the genesis block, we populate parent_block_identifier with the
    // same genesis block. Refer to
    // https://www.rosetta-api.org/docs/common_mistakes.html#malformed-genesis-block
    // TODO: Retrieve the previous block? (if not genesis)
    let parent_block_identifier = block_identifier.clone();

    let block = Block {
        block_identifier,
        parent_block_identifier,
        timestamp: get_timestamp(&response),
        transactions,
    };

    let response = BlockResponse {
        block: Some(block),
        other_transactions: None,
    };

    Ok(response)
}
