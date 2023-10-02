// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use clap::Parser;
use aptos_executor_service::process_executor_service::ProcessExecutorService;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value_t = 8)]
    pub num_executor_threads: usize,

    #[clap(long)]
    pub shard_id: usize,

    #[clap(long)]
    pub num_shards: usize,

    #[clap(long, num_args = 1..)]
    pub remote_executor_addresses: Vec<SocketAddr>,

    #[clap(long)]
    pub coordinator_address: SocketAddr,
}

fn main() {
    // TODO (skedia): Uncomment this once the executor service is implemented.
    let args = Args::parse();
    aptos_logger::Logger::new().init();

    ProcessExecutorService::new(args.shard_id, args.num_shards, args.num_executor_threads, args.coordinator_address, args.remote_executor_addresses);
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
