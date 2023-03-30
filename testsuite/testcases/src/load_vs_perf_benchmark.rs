// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use aptos_forge::{
    EmitJobMode, EmitJobRequest, EntryPoints, NetworkContext, NetworkTest, Result, Test,
    TransactionType, TxnStats,
};
use aptos_logger::info;
use rand::SeedableRng;
use std::{
    fmt::{self, Debug, Display},
    time::Duration,
};
use tokio::runtime::Runtime;

pub struct SingleRunStats {
    name: String,
    stats: TxnStats,
    ledger_transactions: u64,
    actual_duration: Duration,
}

pub enum Workloads {
    TPS(&'static [usize]),
    TRANSACTIONS(&'static [TransactinWorkload]),
}

impl Workloads {
    fn len(&self) -> usize {
        match self {
            Self::TPS(tpss) => tpss.len(),
            Self::TRANSACTIONS(workloads) => workloads.len(),
        }
    }

    fn name(&self, index: usize) -> String {
        match self {
            Self::TPS(tpss) => tpss[index].to_string(),
            Self::TRANSACTIONS(workloads) => workloads[index].to_string(),
        }
    }

    fn configure(&self, index: usize, request: EmitJobRequest) -> EmitJobRequest {
        match self {
            Self::TPS(tpss) => request.mode(EmitJobMode::ConstTps { tps: tpss[index] }),
            Self::TRANSACTIONS(workloads) => workloads[index].configure(request),
        }
    }
}

#[derive(Debug)]
pub enum TransactinWorkload {
    NoOp,
    NoOpUnique,
    LargeModuleWorkingSet,
    WriteResourceSmall,
    WriteResourceBig,
    PublishPackages,
    CoinTransfer,
    CoinTransferUnique,
    NftMint,
}

impl TransactinWorkload {
    fn configure(&self, request: EmitJobRequest) -> EmitJobRequest {
        let account_creation_type = TransactionType::AccountGeneration {
            add_created_accounts_to_pool: true,
            max_account_working_set: 10_000_000,
            creation_balance: 200_000_000,
        };

        match self {
            Self::NoOp => request.transaction_type(TransactionType::CallCustomModules {
                entry_point: EntryPoints::Nop,
                num_modules: 1,
                use_account_pool: false,
            }),
            Self::LargeModuleWorkingSet => {
                request.transaction_type(TransactionType::CallCustomModules {
                    entry_point: EntryPoints::Nop,
                    num_modules: 1000,
                    use_account_pool: false,
                })
            },
            Self::WriteResourceSmall | Self::WriteResourceBig => {
                let write_type = TransactionType::CallCustomModules {
                    entry_point: EntryPoints::BytesMakeOrChange {
                        data_length: Some(
                            if let Self::WriteResourceBig = self {
                                1024
                            } else {
                                32
                            },
                        ),
                    },
                    num_modules: 1,
                    use_account_pool: true,
                };
                request.transaction_mix_per_phase(vec![
                    // warmup
                    vec![(account_creation_type, 1)],
                    vec![(account_creation_type, 1)],
                    vec![(write_type, 1)],
                    // cooldown
                    vec![(write_type, 1)],
                ])
            },
            Self::PublishPackages => {
                let write_type = TransactionType::PublishPackage {
                    use_account_pool: true,
                };
                request.transaction_mix_per_phase(vec![
                    // warmup
                    vec![(account_creation_type, 1)],
                    vec![(account_creation_type, 1)],
                    vec![(write_type, 1)],
                    // cooldown
                    vec![(write_type, 1)],
                ])
            },
            Self::CoinTransfer => {
                request.transaction_type(TransactionType::default_coin_transfer())
            },
            Self::NoOpUnique | Self::CoinTransferUnique => {
                let write_type = if let Self::CoinTransferUnique = self {
                    TransactionType::CoinTransfer {
                        invalid_transaction_ratio: 0,
                        sender_use_account_pool: true,
                    }
                } else {
                    TransactionType::CallCustomModules {
                        entry_point: EntryPoints::Nop,
                        num_modules: 1,
                        use_account_pool: true,
                    }
                };
                request.transaction_mix_per_phase(vec![
                    // warmup
                    vec![(account_creation_type, 1)],
                    vec![(account_creation_type, 1)],
                    vec![(write_type, 1)],
                    // cooldown
                    vec![(write_type, 1)],
                ])
            },
            Self::NftMint => request.transaction_type(TransactionType::NftMintAndTransfer),
        }
    }
}

impl Display for TransactinWorkload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

pub struct LoadVsPerfBenchmark {
    pub test: &'static dyn NetworkLoadTest,
    pub workloads: Workloads,
}

impl Test for LoadVsPerfBenchmark {
    fn name(&self) -> &'static str {
        "continuous progress test"
    }
}

impl LoadVsPerfBenchmark {
    fn evaluate_single(
        &self,
        ctx: &mut NetworkContext<'_>,
        workloads: &Workloads,
        index: usize,
        duration: Duration,
    ) -> Result<Vec<SingleRunStats>> {
        let rng = SeedableRng::from_rng(ctx.core().rng())?;
        let emit_job_request = workloads.configure(index, ctx.emit_job.clone());
        let (stats, actual_duration, ledger_transactions, stats_by_phase) =
            self.test.network_load_test(
                ctx,
                emit_job_request,
                duration,
                // add larger warmup, as when we are exceeding the max load,
                // it takes more time to fill mempool.
                0.2,
                0.05,
                rng,
            )?;

        let mut result = vec![SingleRunStats {
            name: workloads.name(index),
            stats,
            ledger_transactions,
            actual_duration,
        }];

        if stats_by_phase.len() > 1 {
            for (i, (phase_stats, phase_duration)) in stats_by_phase.into_iter().enumerate() {
                result.push(SingleRunStats {
                    name: format!("{}_phase_{}", workloads.name(index), i),
                    stats: phase_stats,
                    ledger_transactions,
                    actual_duration: phase_duration,
                });
            }
        }

        Ok(result)
    }
}

impl NetworkTest for LoadVsPerfBenchmark {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let _runtime = Runtime::new().unwrap();
        let individual_with_buffer = ctx
            .global_duration
            .checked_div(self.workloads.len() as u32)
            .unwrap();
        let individual_duration = individual_with_buffer.mul_f32(0.8);
        let buffer = individual_with_buffer - individual_duration;

        let mut results = Vec::new();
        for index in 0..self.workloads.len() {
            if index != 0 {
                info!("Sleeping in between loadtests, for {}s", buffer.as_secs());
                std::thread::sleep(buffer);
            }

            info!("Starting for {}", self.workloads.name(index));
            results.append(&mut self.evaluate_single(
                ctx,
                &self.workloads,
                index,
                individual_duration,
            )?);

            // Note: uncomment below to perform reconfig during a test
            // let mut aptos_info = ctx.swarm().aptos_public_info();
            // runtime.block_on(aptos_info.reconfig());

            println!(
                "{: <30} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
                "workload",
                "submitted/s",
                "committed/s",
                "expired/s",
                "rejected/s",
                "chain txn/s",
                "latency",
                "p50 lat",
                "p90 lat",
                "p99 lat",
                "actual dur"
            );
            for result in &results {
                let rate = result.stats.rate();
                println!(
                    "{: <30} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
                    result.name,
                    rate.submitted,
                    rate.committed,
                    rate.expired,
                    rate.failed_submission,
                    result.ledger_transactions / result.actual_duration.as_secs(),
                    rate.latency,
                    rate.p50_latency,
                    rate.p90_latency,
                    rate.p99_latency,
                    result.actual_duration.as_secs()
                )
            }
        }
        Ok(())
    }
}
