// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::account_universe::P2PTransferGen;
use aptos_transaction_benchmarks::transactions::TransactionBencher;
use proptest::prelude::*;

fn main() {
    let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));

    let acts = [1000];
    let txns = [1000, 10000];
    let num_warmups = 2;
    let num_runs = 10;

    let mut measurements: Vec<Vec<(usize, usize)>> = Vec::new();
    let concurrency_level = num_cpus::get();

    for block_size in txns {
        for num_accounts in acts {
            let mut times = bencher.blockstm_benchmark(
                num_accounts,
                block_size,
                num_warmups,
                num_runs,
                concurrency_level,
            );
            times.sort();
            measurements.push(times);
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
            println!("Parallel/Sequential TPS: {:?}", measurements[i]);

            let mut par_sum = 0;
            for m in &measurements[i] {
                par_sum += m.0;
            }
            let mut seq_sum = 0;
            for m in &measurements[i] {
                seq_sum += m.1;
            }
            println!(
                "Avg Parallel TPS = {:?}, Avg Sequential TPS = {:?}, speed up {}x",
                par_sum / measurements[i].len(),
                seq_sum / measurements[i].len(),
                par_sum / seq_sum
            );
            i += 1;
        }
        println!();
    }
}
