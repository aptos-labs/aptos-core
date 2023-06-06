// Copyright © Aptos Foundation

use crate::{remote_executor_service, remote_executor_service::RemoteExecutorService};
use std::net::SocketAddr;

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
        remote_executor_service::execute(
            self.server_addr,
            self.network_timeout_ms,
            self.num_executor_threads,
        );
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
