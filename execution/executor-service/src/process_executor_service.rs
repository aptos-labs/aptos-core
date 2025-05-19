// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::remote_executor_service::ExecutorService;
use aptos_logger::info;
use aptos_push_metrics::MetricsPusher;
use aptos_types::block_executor::partitioner::ShardId;
use aptos_vm::AptosVM;
use std::net::SocketAddr;

/// An implementation of the remote executor service that runs in a standalone process.
pub struct ProcessExecutorService {
    executor_service: ExecutorService,
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
            "Starting process remote executor service on {}; coordinator address: {}, other shard addresses: {:?}; num threads: {}",
            self_address, coordinator_address, remote_shard_addresses, num_threads
        );
        aptos_node_resource_metrics::register_node_metrics_collector(None);
        let _mp = MetricsPusher::start_for_local_run(
            &("remote-executor-service-".to_owned() + &shard_id.to_string()),
        );

        AptosVM::set_concurrency_level_once(num_threads);
        let mut executor_service = ExecutorService::new(
            shard_id,
            num_shards,
            num_threads,
            self_address,
            coordinator_address,
            remote_shard_addresses,
        );
        executor_service.start();
        Self { executor_service }
    }

    pub fn shutdown(&mut self) {
        self.executor_service.shutdown()
    }
}

impl Drop for ProcessExecutorService {
    fn drop(&mut self) {
        self.shutdown();
    }
}
