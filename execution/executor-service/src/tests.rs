// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    remote_executor_client::RemoteExecutorClient, test_utils,
    thread_executor_service::ThreadExecutorService,
};
use aptos_config::utils;
use aptos_language_e2e_tests::data_store::FakeDataStore;
use aptos_secure_net::network_controller::NetworkController;
use aptos_vm::sharded_block_executor::ShardedBlockExecutor;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub fn create_thread_remote_executor_shards(
    num_shards: usize,
    num_threads: Option<usize>,
) -> (
    NetworkController,
    RemoteExecutorClient<FakeDataStore>,
    Vec<ThreadExecutorService>,
) {
    // First create the coordinator.
    let listen_port = utils::get_available_port();
    let coordinator_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen_port);
    let mut controller = NetworkController::new(
        "remote-executor-coordinator".to_string(),
        coordinator_address,
        5000,
    );
    let remote_shard_addresses = (0..num_shards)
        .map(|_| {
            let listen_port = utils::get_available_port();
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen_port)
        })
        .collect::<Vec<_>>();

    let num_threads =
        num_threads.unwrap_or_else(|| (num_cpus::get() as f64 / num_shards as f64).ceil() as usize);

    let remote_executor_services = (0..num_shards)
        .map(|shard_id| {
            ThreadExecutorService::new(
                shard_id,
                num_shards,
                num_threads,
                coordinator_address,
                remote_shard_addresses.clone(),
            )
        })
        .collect::<Vec<_>>();

    let remote_executor_client =
        RemoteExecutorClient::new(remote_shard_addresses, &mut controller, None);
    (controller, remote_executor_client, remote_executor_services)
}

#[test]
fn test_sharded_block_executor_no_conflict() {
    let num_shards = 8;
    let (mut controller, executor_client, _executor_services) =
        create_thread_remote_executor_shards(num_shards, Some(2));
    controller.start();
    let sharded_block_executor = ShardedBlockExecutor::new(executor_client);
    test_utils::test_sharded_block_executor_no_conflict(sharded_block_executor);
}
