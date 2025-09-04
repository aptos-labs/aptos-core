// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    remote_executor_client::RemoteExecutorClient, test_utils,
    thread_executor_service::ThreadExecutorService,
};
use velor_config::utils;
use velor_secure_net::network_controller::NetworkController;
use velor_transaction_simulation::InMemoryStateStore;
use velor_vm::sharded_block_executor::ShardedBlockExecutor;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub fn create_thread_remote_executor_shards(
    num_shards: usize,
    num_threads: Option<usize>,
) -> (
    RemoteExecutorClient<InMemoryStateStore>,
    Vec<ThreadExecutorService>,
) {
    // First create the coordinator.
    let listen_port = utils::get_available_port();
    let coordinator_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen_port);
    let controller = NetworkController::new(
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
        RemoteExecutorClient::new(remote_shard_addresses, controller, None);
    (remote_executor_client, remote_executor_services)
}

#[test]
#[ignore]
fn test_sharded_block_executor_no_conflict() {
    use std::thread;

    let num_shards = 8;
    let (executor_client, mut executor_services) =
        create_thread_remote_executor_shards(num_shards, Some(2));
    let sharded_block_executor = ShardedBlockExecutor::new(executor_client);

    // wait for the servers to be ready before sending messages
    // TODO: We need to pass this test without this sleep
    thread::sleep(std::time::Duration::from_millis(10));

    test_utils::test_sharded_block_executor_no_conflict(sharded_block_executor);

    executor_services.iter_mut().for_each(|executor_service| {
        executor_service.shutdown();
    });
}

#[test]
#[ignore]
fn test_sharded_block_executor_with_conflict() {
    use std::thread;

    let num_shards = 8;
    let (executor_client, mut executor_services) =
        create_thread_remote_executor_shards(num_shards, Some(2));
    let sharded_block_executor = ShardedBlockExecutor::new(executor_client);

    // wait for the servers to be ready before sending messages
    // TODO: We need to pass this test without this sleep
    thread::sleep(std::time::Duration::from_millis(10));

    test_utils::sharded_block_executor_with_conflict(sharded_block_executor, 2);

    executor_services.iter_mut().for_each(|executor_service| {
        executor_service.shutdown();
    });
}
