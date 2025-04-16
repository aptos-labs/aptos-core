// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// // Copyright © Aptos Foundation
// // SPDX-License-Identifier: Apache-2.0
// use crate::remote_executor_service::ExecutorService;
// use aptos_types::block_executor::partitioner::ShardId;
// use std::net::SocketAddr;

// /// This is a simple implementation of RemoteExecutorService that runs the executor service in a
// /// separate thread. This should be used for testing only.
// pub struct ThreadExecutorService {
//     _self_address: SocketAddr,
//     executor_service: ExecutorService,
// }

// impl ThreadExecutorService {
//     pub fn new(
//         shard_id: ShardId,
//         num_shards: usize,
//         num_threads: usize,
//         coordinator_address: SocketAddr,
//         remote_shard_addresses: Vec<SocketAddr>,
//     ) -> Self {
//         let self_address = remote_shard_addresses[shard_id];
//         let mut executor_service = ExecutorService::new(
//             shard_id,
//             num_shards,
//             num_threads,
//             self_address,
//             coordinator_address,
//             remote_shard_addresses,
//         );
//         executor_service.start();
//         Self {
//             _self_address: self_address,
//             executor_service,
//         }
//     }

//     pub fn shutdown(&mut self) {
//         self.executor_service.shutdown()
//     }
// }
