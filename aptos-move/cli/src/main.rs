// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Standalone Move CLI for compiling, testing, and managing Move smart contracts.

#![forbid(unsafe_code)]

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use aptos_move_cli::{MoveEnv, MoveTool};
use clap::Parser;
use std::{process::exit, sync::Arc};

/// Move CLI for Aptos Move smart contract development
#[derive(Parser)]
#[clap(name = "move", author, version, propagate_version = true,
       styles = aptos_cli_common::aptos_cli_style())]
struct MoveArgs {
    #[clap(subcommand)]
    tool: MoveTool,
}

fn main() {
    // Register package hooks (needed for Move compilation).
    aptos_move_cli::register_package_hooks();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // Default MoveEnv has no AptosContext or debugger — network-dependent
    // commands will return an error directing users to the full `aptos` CLI.
    let env = Arc::new(MoveEnv::default());
    let result = runtime.block_on(MoveArgs::parse().tool.execute(env));

    // Short shutdown timeout to avoid hanging on background tasks.
    runtime.shutdown_timeout(std::time::Duration::from_millis(50));

    match result {
        Ok(inner) => println!("{}", inner),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        },
    }
}
