// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    remote_executor_service,
    remote_executor_service::{ExecutorService, RemoteExecutorService},
};
use aptos_config::utils;
use aptos_logger::info;
use aptos_secure_net::NetworkServer;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    thread,
    thread::JoinHandle,
};

/// This is a simple implementation of RemoteExecutorService that runs the executor service in a
/// separate thread. This should be used for testing only.
pub struct ThreadExecutorService {
    _child: JoinHandle<()>,
    server_addr: SocketAddr,
    network_timeout_ms: u64,
    num_executor_threads: usize,
}

impl ThreadExecutorService {
    pub fn new(network_timeout_ms: u64, num_executor_threads: usize) -> Self {
        let listen_port = utils::get_available_port();
        let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen_port);
        let server_addr = listen_addr;
        info!("Starting thread remote executor service on {}", listen_addr);

        let network_server =
            NetworkServer::new("thread-executor-service", listen_addr, network_timeout_ms);

        let executor_service = ExecutorService::new(num_executor_threads);

        let child = thread::spawn(move || {
            remote_executor_service::execute(network_server, executor_service);
        });

        Self {
            _child: child,
            server_addr,
            network_timeout_ms,
            num_executor_threads,
        }
    }
}

impl RemoteExecutorService for ThreadExecutorService {
    fn server_address(&self) -> SocketAddr {
        self.server_addr
    }

    fn network_timeout_ms(&self) -> u64 {
        self.network_timeout_ms
    }

    fn executor_threads(&self) -> usize {
        self.num_executor_threads
    }
}
