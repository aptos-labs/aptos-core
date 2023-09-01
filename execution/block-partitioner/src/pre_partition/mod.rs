// Copyright © Aptos Foundation

use crate::v2::state::PartitionState;
use connected_component::config::ConnectedComponentPartitionerConfig;
use std::fmt::Debug;

/// The initial partitioning phase for `ShardedBlockPartitioner`/`PartitionerV2` to divide a block into `num_shards` sub-blocks.
/// See `PartitionerV2::partition()` for more details.
///
/// TODO: the exact parameter set needed by a generic PrePartitioner is currently a moving part, since more PrePartitioner are to be experimented and they may need different preprocessing.
/// Currently passing in the whole `PartitionState`, where:
/// some states that are available for implementations to leverage:
/// - `state.write_sets`
/// - `state.read_sets`
/// - `state.sender_idxs`
/// - `state.num_executor_shards`
/// - `state.txns`
/// Also an implementation must populate the following states:
/// - `state.idx1_to_idx0`
/// - `state.start_txn_idxs_by_shard`
/// - `state.pre_partitioned`
pub trait PrePartitioner: Send {
    fn pre_partition(&self, state: &mut PartitionState);
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
