// Copyright © Aptos Foundation

use aptos_block_partitioner::BlockPartitioner;
use aptos_block_partitioner::sharded_block_partitioner::ShardedBlockPartitioner;
use aptos_block_partitioner::v2::V2Partitioner;
use aptos_transaction_orderer::block_partitioner::OrderedRoundRobinPartitioner;
use aptos_transaction_orderer::v3::V3Partitioner;

pub fn build_partitioner(maybe_num_shards: Option<usize>) -> Box<dyn BlockPartitioner> {
    match std::env::var("APTOS_BLOCK_PARTITIONER_IMPL").ok() {
        Some(v) if v.to_uppercase().as_str() == "V3" => {
            Box::new(V3Partitioner::new())
        }
        Some(v) if v.to_uppercase().as_str() == "V2" => {
            Box::new(V2Partitioner::new())
        }
        _ => {
            Box::new(ShardedBlockPartitioner::new(maybe_num_shards.unwrap()))
        }
    }
}
