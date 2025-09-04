// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    remote_cordinator_client::RemoteCoordinatorClient,
    remote_cross_shard_client::RemoteCrossShardClient, remote_state_view::RemoteStateViewClient,
};
use velor_secure_net::network_controller::NetworkController;
use velor_types::block_executor::partitioner::ShardId;
use velor_vm::sharded_block_executor::sharded_executor_service::ShardedExecutorService;
use std::{net::SocketAddr, sync::Arc, thread};

/// A service that provides support for remote execution. Essentially, it reads a request from
/// the remote executor client and executes the block locally and returns the result.
pub struct ExecutorService {
    shard_id: ShardId,
    controller: NetworkController,
    executor_service: Arc<ShardedExecutorService<RemoteStateViewClient>>,
}

impl ExecutorService {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        self_address: SocketAddr,
        coordinator_address: SocketAddr,
        remote_shard_addresses: Vec<SocketAddr>,
    ) -> Self {
        let service_name = format!("executor_service-{}", shard_id);
        let mut controller = NetworkController::new(service_name, self_address, 5000);
        let coordinator_client = Arc::new(RemoteCoordinatorClient::new(
            shard_id,
            &mut controller,
            coordinator_address,
        ));
        let cross_shard_client = Arc::new(RemoteCrossShardClient::new(
            &mut controller,
            remote_shard_addresses,
        ));

        let executor_service = Arc::new(ShardedExecutorService::new(
            shard_id,
            num_shards,
            num_threads,
            coordinator_client,
            cross_shard_client,
        ));

        Self {
            shard_id,
            controller,
            executor_service,
        }
    }

    pub fn start(&mut self) {
        self.controller.start();
        let thread_name = format!("ExecutorService-{}", self.shard_id);
        let builder = thread::Builder::new().name(thread_name);
        let executor_service_clone = self.executor_service.clone();
        builder
            .spawn(move || {
                executor_service_clone.start();
            })
            .expect("Failed to spawn thread");
    }

    pub fn shutdown(&mut self) {
        self.controller.shutdown();
    }
}
