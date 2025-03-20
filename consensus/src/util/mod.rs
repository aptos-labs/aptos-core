// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure};
use aptos_consensus_types::{block::Block, common::Round, pipelined_block::PipelinedBlock};
use aptos_crypto::HashValue;
use aptos_types::{
    on_chain_config::{OnChainJWKConsensusConfig, OnChainRandomnessConfig},
    validator_txn::ValidatorTransaction,
};
use std::sync::Arc;

pub mod db_tool;
#[cfg(any(test, feature = "fuzzing"))]
pub mod mock_time_service;
pub mod time_service;

pub fn is_vtxn_expected(
    randomness_config: &OnChainRandomnessConfig,
    jwk_consensus_config: &OnChainJWKConsensusConfig,
    vtxn: &ValidatorTransaction,
) -> bool {
    match vtxn {
        ValidatorTransaction::DKGResult(_) => randomness_config.randomness_enabled(),
        ValidatorTransaction::ObservedJWKUpdate(_) => jwk_consensus_config.jwk_consensus_enabled(),
    }
}

pub fn calculate_window_start_round(current_round: Round, window_size: u64) -> Round {
    assert!(window_size > 0);
    (current_round + 1).saturating_sub(window_size)
}

/// A simple trait that provides a way to get blocks by their ID (i.e., hash).
/// This allows us to abstract the block window logic from the underlying storage mechanism.
pub trait BlockStorage {
    fn get_pipelined_block(&self, block_id: &HashValue) -> Option<Arc<PipelinedBlock>>;
}

/// Retrieves a Window of Recent Blocks from Storage
///
/// Returns an [`OrderedBlockWindow`](OrderedBlockWindow) containing the previous `window_size`
/// blocks, EXCLUDING the provided `current_block`. Returns an `OrderedBlockWindow` containing
/// the recent blocks in ascending order by round (oldest -> newest).
///
/// # Parameters
/// - `current_block`: The reference block to base the window on.
/// - `window_size`: The number of recent blocks to include in the window, excluding the `current_block`.
///
/// # Example
/// Given a `current_block` with `round: 30` and a `window_size` of 3:
///
/// ```text
/// get_block_window(current_block, window_size)
/// // returns vec![
/// //     Block { BlockData { round: 28 } },
/// //     Block { BlockData { round: 29 } }
/// // ]
/// ```
///
/// *Note*: The output vector in this example contains 2 blocks, not 3, as only blocks with rounds
/// preceding `current_block.round()` are included.
pub fn get_block_window_from_storage(
    block_store: Arc<&dyn BlockStorage>,
    block: &Block,
    window_size: u64,
) -> anyhow::Result<Vec<Arc<PipelinedBlock>>> {
    let round = block.round();
    let window_start_round = calculate_window_start_round(round, window_size);
    let window_size = round - window_start_round + 1;
    ensure!(window_size > 0, "window_size must be greater than 0");

    let mut window = vec![];
    let mut current_block = block.clone();

    // Add each block to the window until you reach the start round
    while !current_block.is_genesis_block()
        && current_block.quorum_cert().certified_block().round() >= window_start_round
    {
        if let Some(current_pipelined_block) =
            block_store.get_pipelined_block(&current_block.parent_id())
        {
            current_block = current_pipelined_block.block().clone();
            window.push(current_pipelined_block);
        } else {
            bail!("Parent block not found for block {}", current_block.id());
        }
    }

    // The window order is lower round -> higher round
    window.reverse();
    ensure!(window.len() < window_size as usize);
    Ok(window)
}
