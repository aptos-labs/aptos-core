// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    realistic_environment::realistic_env_sweep_wrap, ungrouped::background_traffic_for_sweep,
};
use velor_forge::{
    args::TransactionTypeArg,
    prometheus_metrics::LatencyBreakdownSlice,
    success_criteria::{LatencyBreakdownThreshold, SuccessCriteria},
    ForgeConfig,
};
use velor_testcases::{
    load_vs_perf_benchmark::{LoadVsPerfBenchmark, TransactionWorkload, Workloads},
    performance_test::PerformanceBenchmark,
};

/// Attempts to match the test name to an indexer test
pub fn get_indexer_test(test_name: &str) -> Option<ForgeConfig> {
    let test = match test_name {
        "indexer_test" => indexer_test(),
        _ => return None, // The test name does not match an indexer test
    };
    Some(test)
}

/// Workload sweep with multiple stressful workloads for indexer
fn indexer_test() -> ForgeConfig {
    // Define all the workloads and their corresponding success criteria upfront
    // The TransactionTypeArg is the workload per phase
    // The structure of the success criteria is generally (min_tps, latencies...). See below for the exact definition.
    // NOTES on tuning these criteria:
    // * The blockchain's TPS criteria are generally lower than that of other tests. This is because we want to only capture indexer performance regressions. Other tests with higher TPS requirements
    //   for the blockchain would catch an earlier regression
    // * The indexer latencies are inter-component within the indexer stack, but not that of the e2e wall-clock time vs the blockchain
    let workloads_and_criteria = vec![
        (
            TransactionWorkload::new(TransactionTypeArg::CoinTransfer, 20000),
            (7000, 0.5, 1.0, 0.1),
        ),
        (
            TransactionWorkload::new(TransactionTypeArg::NoOp, 20000).with_num_modules(100),
            (8500, 0.5, 1.0, 0.1),
        ),
        (
            TransactionWorkload::new(TransactionTypeArg::ModifyGlobalResource, 6000)
                .with_transactions_per_account(1),
            (2000, 0.5, 0.5, 0.1),
        ),
        (
            TransactionWorkload::new(TransactionTypeArg::TokenV2AmbassadorMint, 20000)
                .with_unique_senders(),
            (2000, 1.0, 1.0, 0.5),
        ),
        (
            TransactionWorkload::new(TransactionTypeArg::PublishPackage, 200)
                .with_transactions_per_account(1),
            (28, 0.5, 1.0, 0.1),
        ),
        (
            TransactionWorkload::new(TransactionTypeArg::VectorPicture30k, 100),
            (100, 0.5, 1.0, 0.1),
        ),
        (
            TransactionWorkload::new(TransactionTypeArg::SmartTablePicture30KWith200Change, 100),
            (100, 0.5, 1.0, 0.1),
        ),
        (
            TransactionWorkload::new(
                TransactionTypeArg::TokenV1NFTMintAndTransferSequential,
                1000,
            ),
            (500, 0.5, 1.0, 0.1),
        ),
        (
            TransactionWorkload::new(TransactionTypeArg::TokenV1FTMintAndTransfer, 1000),
            (500, 0.5, 0.5, 0.1),
        ),
    ];
    let num_sweep = workloads_and_criteria.len();

    let workloads = Workloads::TRANSACTIONS(
        workloads_and_criteria
            .iter()
            .map(|(w, _)| w.clone())
            .collect(),
    );
    let criteria = workloads_and_criteria
        .iter()
        .map(|(_, c)| {
            let (
                min_tps,
                indexer_fullnode_processed_batch,
                indexer_cache_worker_processed_batch,
                indexer_data_service_all_chunks_sent,
            ) = c.to_owned();
            SuccessCriteria::new(min_tps).add_latency_breakdown_threshold(
                LatencyBreakdownThreshold::new_strict(vec![
                    (
                        LatencyBreakdownSlice::IndexerFullnodeProcessedBatch,
                        indexer_fullnode_processed_batch,
                    ),
                    (
                        LatencyBreakdownSlice::IndexerCacheWorkerProcessedBatch,
                        indexer_cache_worker_processed_batch,
                    ),
                    (
                        LatencyBreakdownSlice::IndexerDataServiceAllChunksSent,
                        indexer_data_service_all_chunks_sent,
                    ),
                ]),
            )
        })
        .collect::<Vec<_>>();

    realistic_env_sweep_wrap(4, 4, LoadVsPerfBenchmark {
        test: Box::new(PerformanceBenchmark),
        workloads,
        criteria,
        background_traffic: background_traffic_for_sweep(num_sweep),
    })
}
