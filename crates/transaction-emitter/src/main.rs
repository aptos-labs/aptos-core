// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, format_err, Result};
use aptos_config::config::DEFAULT_PORT;
use aptos_sdk::{transaction_builder::TransactionFactory, types::chain_id::ChainId};
use futures::future::join_all;
use itertools::zip;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rand_core::OsRng;
use std::{
    cmp::min,
    process,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use termion::color;
use transaction_emitter::{
    cluster::Cluster, instance::Instance, query_sequence_numbers, EmitJobRequest, EmitThreadParams,
    TxnEmitter,
};

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(short = "p", long, use_delimiter = true)]
    peers: Vec<String>,

    #[structopt(long, help = "If set, tries to use public peers instead of localhost")]
    vasp: bool,

    #[structopt(long)]
    emit_tx: bool,
    #[structopt(long)]
    diag: bool,

    // emit_tx options
    #[structopt(long, default_value = "15")]
    accounts_per_client: usize,
    #[structopt(long)]
    workers_per_ac: Option<usize>,
    #[structopt(long, default_value = "0")]
    wait_millis: u64,
    #[structopt(long)]
    burst: bool,
    #[structopt(long, default_value = "mint.key")]
    mint_file: String,
    #[structopt(long, default_value = "TESTING")]
    chain_id: ChainId,
    #[structopt(
        long,
        help = "Time to run --emit-tx for in seconds",
        default_value = "60"
    )]
    duration: u64,
    #[structopt(long, help = "Percentage of invalid txs", default_value = "0")]
    invalid_tx: usize,
}

#[tokio::main]
pub async fn main() {
    let args = Args::from_args();

    if !args.emit_tx && !args.diag {
        panic!("Can only use --emit-tx or --diag mode");
    }

    let util = BasicSwarmUtil::setup(&args);
    if args.diag {
        exit_on_error(util.diag(args.vasp).await);
        return;
    } else if args.emit_tx {
        exit_on_error(emit_tx(&util.cluster, &args).await);
        return;
    }

    let util = BasicSwarmUtil::setup(&args);
    exit_on_error(util.diag(args.vasp).await);
}

async fn emit_tx(cluster: &Cluster, args: &Args) -> Result<()> {
    let thread_params = EmitThreadParams {
        wait_millis: args.wait_millis,
        wait_committed: !args.burst,
    };
    let duration = Duration::from_secs(args.duration);
    let client = cluster.random_instance().rest_client();
    let mut root_account = cluster.load_aptos_root_account(&client).await?;
    let mut emitter = TxnEmitter::new(
        &mut root_account,
        client,
        TransactionFactory::new(cluster.chain_id).with_gas_unit_price(1),
        StdRng::from_seed(OsRng.gen()),
    );
    let mut emit_job_request =
        EmitJobRequest::new(cluster.all_instances().map(Instance::rest_client).collect())
            .accounts_per_client(args.accounts_per_client)
            .thread_params(thread_params)
            .invalid_transaction_ratio(args.invalid_tx)
            .gas_price(1);
    if let Some(workers_per_endpoint) = args.workers_per_ac {
        emit_job_request = emit_job_request.workers_per_endpoint(workers_per_endpoint);
    }
    if args.vasp {
        emit_job_request = emit_job_request.vasp();
    }
    let stats = emitter
        .emit_txn_for_with_stats(duration, emit_job_request, 10)
        .await?;
    println!("Total stats: {}", stats);
    println!("Average rate: {}", stats.rate(duration));
    Ok(())
}

fn parse_host_port(s: &str) -> Result<(String, u32, Option<u32>)> {
    let v = s.split(':').collect::<Vec<&str>>();
    if v.len() == 1 {
        let default_port = DEFAULT_PORT as u32;
        return Ok((v[0].to_string(), default_port, None));
    }
    if v.len() != 2 && v.len() != 3 {
        return Err(format_err!(
            "Failed to parse {:?} in host:port or host:port:debug_interface_port format",
            s
        ));
    }
    let host = v[0].to_string();
    let port = v[1].parse::<u32>()?;
    if v.len() == 3 {
        let debug_interface_port = v[2].parse::<u32>()?;
        return Ok((host, port, Some(debug_interface_port)));
    }
    Ok((host, port, None))
}

struct BasicSwarmUtil {
    cluster: Cluster,
}

impl BasicSwarmUtil {
    pub fn setup(args: &Args) -> Self {
        if args.peers.is_empty() {
            panic!("Peers not set in args");
        }
        let parsed_peers: Vec<_> = args
            .peers
            .iter()
            .map(|peer| parse_host_port(peer).expect("Failed to parse host_port"))
            .collect();

        let cluster =
            Cluster::from_host_port(parsed_peers, &args.mint_file, args.chain_id, args.vasp);
        Self { cluster }
    }

    pub async fn diag(&self, _vasp: bool) -> Result<()> {
        let client = self.cluster.random_instance().rest_client();
        let mut root_account = self.cluster.load_aptos_root_account(&client).await?;
        let mut faucet_account = self.cluster.load_aptos_root_account(&client).await?;
        let emitter = TxnEmitter::new(
            &mut root_account,
            client,
            TransactionFactory::new(self.cluster.chain_id).with_gas_unit_price(1),
            StdRng::from_seed(OsRng.gen()),
        );
        let faucet_account_address = faucet_account.address();
        let instances: Vec<_> = self.cluster.all_instances().collect();
        for instance in &instances {
            print!("Submitting txn through {}...", instance);
            let deadline = emitter
                .submit_single_transaction(
                    &instance.rest_client(),
                    &mut faucet_account,
                    &faucet_account_address,
                    10,
                )
                .await
                .map_err(|e| format_err!("Failed to submit txn through {}: {}", instance, e))?;
            println!("seq={}", faucet_account.sequence_number());
            println!(
                "Waiting all full nodes to get to seq {}",
                faucet_account.sequence_number()
            );
            loop {
                let addresses = &[faucet_account_address];
                let clients = instances
                    .iter()
                    .map(|instance| instance.rest_client())
                    .collect::<Vec<_>>();
                let futures = clients
                    .iter()
                    .map(|client| query_sequence_numbers(client, addresses));
                let results = join_all(futures).await;
                let mut all_good = true;
                for (instance, result) in zip(instances.iter(), results) {
                    let seq = result.map_err(|e| {
                        format_err!("Failed to query sequence number from {}: {}", instance, e)
                    })?[0];
                    let ip = instance.ip();
                    let color = if seq != faucet_account.sequence_number() {
                        all_good = false;
                        color::Fg(color::Red).to_string()
                    } else {
                        color::Fg(color::Green).to_string()
                    };
                    print!(
                        "[{}{}:{}{}]  ",
                        color,
                        &ip[..min(ip.len(), 10)],
                        seq,
                        color::Fg(color::Reset)
                    );
                }
                println!();
                if all_good {
                    break;
                }
                if Instant::now() > deadline {
                    bail!("Not all full nodes were updated and transaction expired");
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
        println!("Looks like all full nodes are healthy!");
        Ok(())
    }
}

fn exit_on_error<T>(r: Result<T>) -> T {
    match r {
        Ok(r) => r,
        Err(err) => {
            println!("{}", err);
            process::exit(1)
        }
    }
}
