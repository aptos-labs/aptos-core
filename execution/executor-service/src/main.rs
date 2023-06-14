// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_executor_service::process_executor_service::ProcessExecutorService;
use clap::Parser;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value = "8080")]
    pub server_port: u16,

    #[clap(long, default_value = "8")]
    pub num_executor_threads: usize,
}

fn main() {
    let args = Args::parse();
    aptos_logger::Logger::new().init();

    let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), args.server_port);
    let executor_service =
        ProcessExecutorService::new(server_addr, 1000, args.num_executor_threads);
    executor_service.run();
}
