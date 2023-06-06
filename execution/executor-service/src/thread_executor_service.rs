// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{remote_executor_service, remote_executor_service::RemoteExecutorService};
use aptos_config::utils;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    thread::{self, JoinHandle},
};

/// This is a simple implementation of RemoteExecutorService that runs the executor service in a
/// separate thread. This should be used for testing only.
pub struct ThreadExecutorService {
    _child: JoinHandle<()>,
    server_addr: SocketAddr,
    network_timeout: u64,
    num_executor_threads: usize,
}

impl ThreadExecutorService {
    pub fn new(timeout: u64, num_executor_threads: usize) -> Self {
        let listen_port = utils::get_available_port();
        let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen_port);
        let server_addr = listen_addr;

        let child = thread::spawn(move || {
            remote_executor_service::execute(listen_addr, timeout, num_executor_threads)
        });

        Self {
            _child: child,
            server_addr,
            network_timeout: timeout,
            num_executor_threads,
        }
    }
}

impl RemoteExecutorService for ThreadExecutorService {
    fn server_address(&self) -> SocketAddr {
        self.server_addr
    }

    fn network_timeout_ms(&self) -> u64 {
        self.network_timeout
    }

    fn executor_threads(&self) -> usize {
        self.num_executor_threads
    }
}
