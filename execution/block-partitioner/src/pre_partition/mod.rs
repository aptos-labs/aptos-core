// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::v2::{
    state::PartitionState,
    types::{OriginalTxnIdx, PrePartitionedTxnIdx},
};
use connected_component::config::ConnectedComponentPartitionerConfig;
use std::fmt::Debug;

/// The initial partitioning phase for `ShardedBlockPartitioner`/`PartitionerV2` to divide a block into `num_shards` sub-blocks.
/// See `PartitionerV2::partition()` for more details.
///
/// TODO: the exact parameter set needed by a generic PrePartitioner is currently a moving part, since more PrePartitioners are to be experimented and they may need different indices.
/// Currently passing in the whole `PartitionState`.
///
/// NOTES for new implementations.
///
/// The following states that are available and can be useful. (see comments on `PartitionState` for a full list of available resources).
/// - `state.txns`: the original block.
/// - `state.sender_idxs`: maps a txn index to its sender index.
/// - `state.read_sets`: maps a txn index to its read set (a state key index set).
/// - `state.write_sets`: maps a txn index to its write set (a state key index set).
/// - `state.num_executor_shards`: the number of shards.
///
/// Implementations are responsible to create the following state items.
/// - `ori_idxs_by_pre_partitioned`: maps a txn's new index to its original index.
/// - `start_txn_idxs_by_shard`: maps a shard to the starting new index of the txns assigned to itself.
/// - `pre_partitioned`: maps a shard to the new indices of the txns assigned to itself.
pub trait PrePartitioner: Send {
    fn pre_partition(
        &self,
        state: &PartitionState,
    ) -> (
        Vec<OriginalTxnIdx>,
        Vec<PrePartitionedTxnIdx>,
        Vec<Vec<PrePartitionedTxnIdx>>,
    );
}

pub mod connected_component;
pub mod uniform_partitioner;

pub trait PrePartitionerConfig: Debug {
    fn build(&self) -> Box<dyn PrePartitioner>;
}

/// Create a default `PrePartitionerConfig`.
pub fn default_pre_partitioner_config() -> Box<dyn PrePartitionerConfig> {
    Box::<ConnectedComponentPartitionerConfig>::default()
}
