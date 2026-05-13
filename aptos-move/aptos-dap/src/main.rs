// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_dap::server::{DapServer, RunCommand};
use clap::{Args, Parser, Subcommand};
use std::{
    collections::BTreeMap,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

#[derive(Parser)]
#[command(name = "aptos-dap", about = "Debug Adapter Protocol server for Move")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Debug a Move unit test
    Test(TestArgs),
    /// Replay a committed transaction
    Replay(ReplayArgs),
}

#[derive(Args)]
struct CommonArgs {
    /// TCP port to listen on (if omitted, uses stdin/stdout)
    #[arg(long)]
    port: Option<u16>,

    /// Skip fetching latest git dependencies
    #[arg(long)]
    skip_fetch_latest_git_deps: bool,
}

#[derive(Args)]
struct TestArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Test function name filter
    #[arg(long, default_value = "")]
    filter: String,

    /// Path to the Move package
    #[arg(long)]
    package_path: PathBuf,
}

#[derive(Args)]
struct ReplayArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Transaction version to replay
    #[arg(long)]
    txn_id: u64,

    /// Network to replay on (mainnet, testnet, or a REST endpoint URL)
    #[arg(long)]
    network: String,

    /// Path to a local Move package for source-level debugging (can be repeated)
    #[arg(long = "use-local-package")]
    use_local_packages: Vec<PathBuf>,

    /// Named address mapping as NAME=ADDRESS (can be repeated)
    #[arg(long = "named-address", value_parser = parse_named_address)]
    named_addresses: Vec<(String, String)>,
}

fn parse_named_address(s: &str) -> Result<(String, String), String> {
    let (name, addr) = s
        .split_once('=')
        .ok_or_else(|| format!("expected NAME=ADDRESS, got '{s}'"))?;
    Ok((name.to_string(), addr.to_string()))
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let (mode, common) = match cli.command {
        Command::Test(args) => {
            let mode = RunCommand::Test {
                filter: args.filter,
                package_path: args.package_path,
                skip_fetch_latest_git_deps: args.common.skip_fetch_latest_git_deps,
            };
            (mode, args.common)
        },
        Command::Replay(args) => {
            let named_addresses: BTreeMap<String, aptos_types::account_address::AccountAddress> =
                args.named_addresses
                    .into_iter()
                    .map(|(name, addr)| {
                        let parsed = addr
                            .parse()
                            .map_err(|e| anyhow::anyhow!("invalid address for '{name}': {e}"))?;
                        Ok((name, parsed))
                    })
                    .collect::<anyhow::Result<_>>()?;
            let mode = RunCommand::Replay {
                txn_id: args.txn_id,
                network: args.network,
                local_packages: args.use_local_packages,
                prebuilt_packages: vec![],
                named_addresses,
                skip_fetch_latest_git_deps: args.common.skip_fetch_latest_git_deps,
            };
            (mode, args.common)
        },
    };

    if let Some(port) = common.port {
        run_tcp(port, mode)
    } else {
        run_stdio(mode)
    }
}

fn run_stdio(mode: RunCommand) -> anyhow::Result<()> {
    let input = BufReader::new(std::io::stdin().lock());
    let output = BufWriter::new(std::io::stdout().lock());
    let mut server = DapServer::new(input, output, mode)?;
    server.run()
}

fn run_tcp(port: u16, mode: RunCommand) -> anyhow::Result<()> {
    use std::net::TcpListener;

    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let actual_port = listener.local_addr()?.port();
    eprintln!("aptos-dap: listening on 127.0.0.1:{actual_port}, waiting for connection...");
    let stream = loop {
        let (stream, addr) = listener.accept()?;
        eprintln!("aptos-dap: connection from {addr}");
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        let mut peek = [0u8; 1];
        match stream.peek(&mut peek) {
            Ok(n) if n > 0 => {
                stream.set_read_timeout(None)?;
                eprintln!("aptos-dap: client connected from {addr}");
                break stream;
            },
            _ => {
                eprintln!("aptos-dap: probe connection from {addr}, ignoring");
                continue;
            },
        }
    };
    let input = BufReader::new(stream.try_clone()?);
    let output = BufWriter::new(stream);
    let mut server = DapServer::new(input, output, mode)?;
    server.run()
}
