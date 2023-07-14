// Copyright © Aptos Foundation

use crate::remote_executor_service::ExecutorService;
use aptos_logger::info;
use aptos_types::block_executor::partitioner::ShardId;
use std::net::SocketAddr;

/// An implementation of the remote executor service that runs in a standalone process.
pub struct ProcessExecutorService {
    _executor_service: ExecutorService,
}

impl ProcessExecutorService {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        coordinator_address: SocketAddr,
        remote_shard_addresses: Vec<SocketAddr>,
    ) -> Self {
        let self_address = remote_shard_addresses[shard_id];
        info!(
            "Starting process remote executor service on {}",
            self_address
        );
        let mut executor_service = ExecutorService::new(
            shard_id,
            num_shards,
            num_threads,
            self_address,
            coordinator_address,
            remote_shard_addresses,
        );
        executor_service.start();
        Self {
            _executor_service: executor_service,
        }
    }
}
