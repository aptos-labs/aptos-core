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
    let check_correctness = false;

    let mut measurements = Vec::new();
    let concurrency_level = num_cpus::get();

    for block_size in txns {
        for num_accounts in acts {
            let mut times = bencher.blockstm_benchmark(
                num_accounts,
                block_size,
                num_warmups,
                num_runs,
                concurrency_level,
                check_correctness,
            );
            times.sort();
            measurements.push(times);
        }
    }
    if check_correctness {
        println!("\nParallel execution output same as sequential!\n");
    }
    println!("concurrency_level = {}", concurrency_level);

    let mut i = 0;
    for block_size in txns {
        for num_accounts in acts {
            println!(
                "PARAMS: num_account = {}, block_size = {}",
                num_accounts, block_size
            );
            println!("TPS: {:?}", measurements[i]);
            let mut sum = 0;
            for m in &measurements[i] {
                sum += m;
            }
            println!("AVG TPS = {:?}", sum / measurements[i].len());
            i += 1;
        }
        println!();
    }
}
