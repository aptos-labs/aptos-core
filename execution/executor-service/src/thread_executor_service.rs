// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::remote_executor_service::ExecutorService;
use aptos_types::block_executor::partitioner::ShardId;
use std::{net::SocketAddr, thread, thread::JoinHandle};

/// This is a simple implementation of RemoteExecutorService that runs the executor service in a
/// separate thread. This should be used for testing only.
pub struct ThreadExecutorService {
    _child: JoinHandle<()>,
    _self_address: SocketAddr,
}

impl ThreadExecutorService {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        coordinator_address: SocketAddr,
        remote_shard_addresses: Vec<SocketAddr>,
    ) -> Self {
        let self_address = remote_shard_addresses[shard_id];
        let mut executor_service = ExecutorService::new(
            shard_id,
            num_shards,
            num_threads,
            self_address,
            coordinator_address,
            remote_shard_addresses,
        );

        let thread_name = format!("ThreadExecutorService-{}", shard_id);
        let builder = thread::Builder::new().name(thread_name);

        let child = builder
            .spawn(move || {
                executor_service.start();
            })
            .expect("Failed to spawn thread");

        Self {
            _child: child,
            _self_address: self_address,
        }
    }
}
