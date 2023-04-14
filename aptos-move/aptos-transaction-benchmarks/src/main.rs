// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::account_universe::P2PTransferGen;
use aptos_transaction_benchmarks::transactions::TransactionBencher;
use proptest::prelude::*;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (run_par, run_seq) = if args.len() == 4 {
        let bool1 = args[2].parse().unwrap();
        let bool2 = args[3].parse().unwrap();
        (bool1, bool2)
    } else {
        println!("Usage: cargo run --release main <bool1: run parallel execution> <bool2: run sequential execution>");
        println!("Will run both parallel & sequential by default\n");
        (true, true)
    };

    let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));

    let acts = [10000];
    let txns = [1000, 10000, 50000];
    let num_warmups = 2;
    let num_runs = 10;

    let mut par_measurements: Vec<Vec<usize>> = Vec::new();
    let mut seq_measurements: Vec<Vec<usize>> = Vec::new();

    let concurrency_level = num_cpus::get();

    for block_size in txns {
        for num_accounts in acts {
            let (mut par_tps, mut seq_tps) = bencher.blockstm_benchmark(
                num_accounts,
                block_size,
                run_par,
                run_seq,
                num_warmups,
                num_runs,
                concurrency_level,
            );
            par_tps.sort();
            seq_tps.sort();
            par_measurements.push(par_tps);
            seq_measurements.push(seq_tps);
        }
    }

    println!("\nconcurrency_level = {}\n", concurrency_level);

    let mut i = 0;
    for block_size in txns {
        for num_accounts in acts {
            println!(
                "PARAMS: num_account = {}, block_size = {}",
                num_accounts, block_size
            );

            let mut seq_tps = 1;
            if run_seq {
                println!("Sequential TPS: {:?}", seq_measurements[i]);
                let mut seq_sum = 0;
                for m in &seq_measurements[i] {
                    seq_sum += m;
                }
                seq_tps = seq_sum / seq_measurements[i].len();
                println!("Avg Sequential TPS = {:?}", seq_tps,);
            }

            if run_par {
                println!("Parallel TPS: {:?}", par_measurements[i]);
                let mut par_sum = 0;
                for m in &par_measurements[i] {
                    par_sum += m;
                }
                let par_tps = par_sum / par_measurements[i].len();
                println!("Avg Parallel TPS = {:?}", par_tps,);
                if run_seq {
                    println!("Speed up {}x over sequential", par_tps / seq_tps);
                }
            }
            i += 1;
        }
        println!();
    }
}
