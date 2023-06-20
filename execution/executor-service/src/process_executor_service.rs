// Copyright © Aptos Foundation

use crate::{
    remote_executor_service,
    remote_executor_service::{ExecutorService, RemoteExecutorService},
};
use aptos_logger::info;
use aptos_secure_net::NetworkServer;
use std::net::SocketAddr;

/// An implementation of the remote executor service that runs in a standalone process.
pub struct ProcessExecutorService {
    server_addr: SocketAddr,
    network_timeout_ms: u64,
    num_executor_threads: usize,
}

impl ProcessExecutorService {
    pub fn new(server_addr: SocketAddr, network_timeout: u64, num_executor_threads: usize) -> Self {
        Self {
            server_addr,
            network_timeout_ms: network_timeout,
            num_executor_threads,
        }
    }

    pub fn run(&self) {
        info!(
            "Starting process remote executor service on {}",
            self.server_addr
        );
        let network_server = NetworkServer::new(
            "process-executor-service",
            self.server_addr,
            self.network_timeout_ms,
        );
        let executor_service = ExecutorService::new(self.num_executor_threads);
        remote_executor_service::execute(network_server, executor_service);
    }
}

impl RemoteExecutorService for ProcessExecutorService {
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
