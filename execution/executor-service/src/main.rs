// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value_t = 8080)]
    pub server_port: u16,

    #[clap(long, default_value_t = 8)]
    pub num_executor_threads: usize,
}

fn main() {
    // TODO (skedia): Uncomment this once the executor service is implemented.
    let _args = Args::parse();
    aptos_logger::Logger::new().init();

    // let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), args.server_port);
    // let executor_service =
    //     ProcessExecutorService::new(server_addr, 1000, args.num_executor_threads);
    // executor_service.run();
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
