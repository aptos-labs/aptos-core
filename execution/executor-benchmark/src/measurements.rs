// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::TIMER;
use aptos_block_executor::counters::{
    self as block_executor_counters, BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK, GasType,
};
use aptos_executor::metrics::{
    COMMIT_BLOCKS, GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING, OTHER_TIMERS,
    PROCESSED_TXNS_OUTPUT_SIZE, UPDATE_LEDGER,
};
use aptos_logger::info;
use aptos_metrics_core::Histogram;
use move_core_types::language_storage::StructTag;
use std::{
    collections::{BTreeMap, HashMap},
    time::Instant,
};

#[derive(Debug, Clone)]
struct GasMeasurement {
    pub gas: f64,
    pub effective_block_gas: f64,

    pub io_gas: f64,
    pub execution_gas: f64,

    pub storage_fee: f64,

    pub approx_block_output: f64,

    pub gas_count: u64,

    pub speculative_abort_count: u64,
}

impl GasMeasurement {
    pub fn sequential_gas_counter(gas_type: &str) -> Histogram {
        block_executor_counters::TXN_GAS
            .with_label_values(&[block_executor_counters::Mode::SEQUENTIAL, gas_type])
    }

    pub fn parallel_gas_counter(gas_type: &str) -> Histogram {
        block_executor_counters::TXN_GAS
            .with_label_values(&[block_executor_counters::Mode::PARALLEL, gas_type])
    }

    pub fn now() -> GasMeasurement {
        let gas = Self::sequential_gas_counter(GasType::NON_STORAGE_GAS).get_sample_sum()
            + Self::parallel_gas_counter(GasType::NON_STORAGE_GAS).get_sample_sum();

        let io_gas = Self::sequential_gas_counter(GasType::IO_GAS).get_sample_sum()
            + Self::parallel_gas_counter(GasType::IO_GAS).get_sample_sum();
        let execution_gas = Self::sequential_gas_counter(GasType::EXECUTION_GAS).get_sample_sum()
            + Self::parallel_gas_counter(GasType::EXECUTION_GAS).get_sample_sum();

        let storage_fee = Self::sequential_gas_counter(GasType::STORAGE_FEE).get_sample_sum()
            + Self::parallel_gas_counter(GasType::STORAGE_FEE).get_sample_sum()
            - (Self::sequential_gas_counter(GasType::STORAGE_FEE_REFUND).get_sample_sum()
                + Self::parallel_gas_counter(GasType::STORAGE_FEE_REFUND).get_sample_sum());

        let gas_count = Self::sequential_gas_counter(GasType::NON_STORAGE_GAS).get_sample_count()
            + Self::parallel_gas_counter(GasType::NON_STORAGE_GAS).get_sample_count();

        let effective_block_gas = block_executor_counters::EFFECTIVE_BLOCK_GAS
            .with_label_values(&[block_executor_counters::Mode::SEQUENTIAL])
            .get_sample_sum()
            + block_executor_counters::EFFECTIVE_BLOCK_GAS
                .with_label_values(&[block_executor_counters::Mode::PARALLEL])
                .get_sample_sum();

        let approx_block_output = block_executor_counters::APPROX_BLOCK_OUTPUT_SIZE
            .with_label_values(&[block_executor_counters::Mode::SEQUENTIAL])
            .get_sample_sum()
            + block_executor_counters::APPROX_BLOCK_OUTPUT_SIZE
                .with_label_values(&[block_executor_counters::Mode::PARALLEL])
                .get_sample_sum();

        let speculative_abort_count = block_executor_counters::SPECULATIVE_ABORT_COUNT.get();

        Self {
            gas,
            effective_block_gas,
            io_gas,
            execution_gas,
            storage_fee,
            approx_block_output,
            gas_count,
            speculative_abort_count,
        }
    }

    pub fn elapsed_delta(self) -> Self {
        let end = Self::now();

        Self {
            gas: end.gas - self.gas,
            effective_block_gas: end.effective_block_gas - self.effective_block_gas,
            io_gas: end.io_gas - self.io_gas,
            execution_gas: end.execution_gas - self.execution_gas,
            storage_fee: end.storage_fee - self.storage_fee,
            approx_block_output: end.approx_block_output - self.approx_block_output,
            gas_count: end.gas_count - self.gas_count,
            speculative_abort_count: end.speculative_abort_count - self.speculative_abort_count,
        }
    }
}

static OTHER_LABELS: &[(&str, bool, &str)] = &[
    ("1.", true, "verified_state_view"),
    ("2.", true, "state_checkpoint"),
    ("2.1.", false, "sort_transactions"),
    ("2.2.", false, "calculate_for_transaction_block"),
    ("2.2.1.", false, "get_sharded_state_updates"),
    ("2.2.2.", false, "calculate_block_state_updates"),
    ("2.2.3.", false, "calculate_usage"),
    ("2.2.4.", false, "make_checkpoint"),
];

#[derive(Debug, Clone)]
struct ExecutionTimeMeasurement {
    output_size: f64,

    sig_verify_total_time: f64,
    partitioning_total_time: f64,
    execution_total_time: f64,
    block_executor_total_time: f64,
    block_executor_inner_total_time: f64,
    by_other: HashMap<&'static str, f64>,
    ledger_update_total: f64,
    commit_total_time: f64,
}

impl ExecutionTimeMeasurement {
    pub fn now() -> Self {
        let output_size = PROCESSED_TXNS_OUTPUT_SIZE
            .with_label_values(&["execution"])
            .get_sample_sum();

        let sig_verify_total = TIMER.with_label_values(&["sig_verify"]).get_sample_sum();
        let partitioning_total = TIMER.with_label_values(&["partition"]).get_sample_sum();
        let execution_total = TIMER.with_label_values(&["execute"]).get_sample_sum();
        let block_executor_total = GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING.get_sample_sum();
        let block_executor_inner_total = BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK.get_sample_sum();

        let by_other = OTHER_LABELS
            .iter()
            .map(|(_prefix, _top_level, other_label)| {
                (
                    *other_label,
                    OTHER_TIMERS
                        .with_label_values(&[other_label])
                        .get_sample_sum(),
                )
            })
            .collect::<HashMap<_, _>>();
        let ledger_update_total = UPDATE_LEDGER.get_sample_sum();
        let commit_total = COMMIT_BLOCKS.get_sample_sum();

        Self {
            output_size,
            sig_verify_total_time: sig_verify_total,
            partitioning_total_time: partitioning_total,
            execution_total_time: execution_total,
            block_executor_total_time: block_executor_total,
            block_executor_inner_total_time: block_executor_inner_total,
            by_other,
            ledger_update_total,
            commit_total_time: commit_total,
        }
    }

    pub fn elapsed_delta(self) -> Self {
        let end = Self::now();

        Self {
            output_size: end.output_size - self.output_size,
            sig_verify_total_time: end.sig_verify_total_time - self.sig_verify_total_time,
            partitioning_total_time: end.partitioning_total_time - self.partitioning_total_time,
            execution_total_time: end.execution_total_time - self.execution_total_time,
            block_executor_total_time: end.block_executor_total_time
                - self.block_executor_total_time,
            block_executor_inner_total_time: end.block_executor_inner_total_time
                - self.block_executor_inner_total_time,
            by_other: end
                .by_other
                .into_iter()
                .map(|(k, v)| (k, v - self.by_other.get(&k).unwrap()))
                .collect::<HashMap<_, _>>(),
            ledger_update_total: end.ledger_update_total - self.ledger_update_total,
            commit_total_time: end.commit_total_time - self.commit_total_time,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OverallMeasuring {
    pub(crate) start_time: Instant,
    start_execution: ExecutionTimeMeasurement,
    start_gas: GasMeasurement,
}

impl OverallMeasuring {
    pub fn start() -> Self {
        Self {
            start_time: Instant::now(),
            start_execution: ExecutionTimeMeasurement::now(),
            start_gas: GasMeasurement::now(),
        }
    }

    pub fn elapsed(self, prefix: String, metadata: String, num_txns: u64) -> OverallMeasurement {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let delta_execution = self.start_execution.elapsed_delta();
        let delta_gas = self.start_gas.elapsed_delta();

        OverallMeasurement {
            prefix,
            metadata,
            elapsed,
            num_txns,
            delta_execution,
            delta_gas,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OverallMeasurement {
    prefix: String,
    metadata: String,
    elapsed: f64,
    num_txns: u64,
    delta_execution: ExecutionTimeMeasurement,
    delta_gas: GasMeasurement,
}

impl OverallMeasurement {
    pub fn get_tps(&self) -> f64 {
        self.num_txns as f64 / self.elapsed
    }

    pub fn get_gps(&self) -> f64 {
        self.delta_gas.gas / self.elapsed
    }

    pub fn get_effective_gps(&self) -> f64 {
        self.delta_gas.effective_block_gas / self.elapsed
    }

    pub fn get_effective_conflict_multiplier(&self) -> f64 {
        self.delta_gas.effective_block_gas / self.delta_gas.gas
    }

    pub fn get_speculative_abort_rate(&self) -> f64 {
        self.delta_gas.speculative_abort_count as f64 / self.num_txns as f64
    }

    pub fn get_io_gps(&self) -> f64 {
        self.delta_gas.io_gas / self.elapsed
    }

    pub fn get_execution_gps(&self) -> f64 {
        self.delta_gas.execution_gas / self.elapsed
    }

    pub fn get_gpt(&self) -> f64 {
        self.delta_gas.gas / (self.delta_gas.gas_count as f64).max(1.0)
    }

    pub fn get_io_gpt(&self) -> f64 {
        self.delta_gas.io_gas / (self.delta_gas.gas_count as f64).max(1.0)
    }

    pub fn get_execution_gpt(&self) -> f64 {
        self.delta_gas.execution_gas / (self.delta_gas.gas_count as f64).max(1.0)
    }

    pub fn get_storage_fee_per_txn(&self) -> f64 {
        self.delta_gas.storage_fee / (self.delta_gas.gas_count as f64).max(1.0)
    }

    pub fn get_approx_output_per_s(&self) -> f64 {
        self.delta_gas.approx_block_output / self.elapsed
    }

    pub fn get_output_per_s(&self) -> f64 {
        self.delta_execution.output_size / self.elapsed
    }

    pub fn print_end(&self) {
        let num_txns = self.num_txns as f64;

        info!("{}: {}", self.prefix, self.metadata);

        info!(
            "{} TPS: {} txn/s (over {} txns, in {} s)",
            self.prefix,
            self.get_tps(),
            num_txns,
            self.elapsed
        );
        info!("{} GPS: {} gas/s", self.prefix, self.get_gps());
        info!(
            "{} effectiveGPS: {} gas/s ({} effective block gas, in {} s)",
            self.prefix,
            self.get_effective_gps(),
            self.delta_gas.effective_block_gas,
            self.elapsed
        );
        info!(
            "{} effective conflict multiplier: {}",
            self.prefix,
            self.get_effective_conflict_multiplier()
        );
        info!(
            "{} speculative aborts: {} aborts/txn ({} aborts over {} txns)",
            self.prefix,
            self.get_speculative_abort_rate(),
            self.delta_gas.speculative_abort_count,
            self.num_txns
        );
        info!("{} ioGPS: {} gas/s", self.prefix, self.get_io_gps());
        info!(
            "{} executionGPS: {} gas/s",
            self.prefix,
            self.get_execution_gps()
        );
        info!("{} GPT: {} gas/txn", self.prefix, self.get_gpt());
        info!(
            "{} Storage fee: {} octas/txn",
            self.prefix,
            self.get_storage_fee_per_txn()
        );
        info!(
            "{} approx_output: {} bytes/s",
            self.prefix,
            self.get_approx_output_per_s()
        );
        info!(
            "{} output: {} bytes/s",
            self.prefix,
            self.get_output_per_s()
        );

        info!(
            "{} fraction of total: {:.4} in signature verification (component TPS: {:.1})",
            self.prefix,
            self.delta_execution.sig_verify_total_time / self.elapsed,
            num_txns / self.delta_execution.sig_verify_total_time
        );
        info!(
            "{} fraction of total: {:.4} in partitioning (component TPS: {:.1})",
            self.prefix,
            self.delta_execution.partitioning_total_time / self.elapsed,
            num_txns / self.delta_execution.partitioning_total_time
        );
        info!(
            "{} fraction of total: {:.4} in execution (component TPS: {:.1})",
            self.prefix,
            self.delta_execution.execution_total_time / self.elapsed,
            num_txns / self.delta_execution.execution_total_time
        );
        info!(
            "{} fraction of execution {:.4} in get execution output by executing (component TPS: {:.1})",
            self.prefix,
            self.delta_execution.block_executor_total_time
                / self.delta_execution.execution_total_time,
            num_txns / self.delta_execution.block_executor_total_time
        );
        info!(
            "{} fraction of execution {:.4} in inner block executor (component TPS: {:.1})",
            self.prefix,
            self.delta_execution.block_executor_inner_total_time
                / self.delta_execution.execution_total_time,
            num_txns / self.delta_execution.block_executor_inner_total_time
        );
        for (label_prefix, top_level, other_label) in OTHER_LABELS {
            let time_in_label = self.delta_execution.by_other.get(other_label).unwrap();
            if *top_level || time_in_label / self.delta_execution.execution_total_time > 0.01 {
                info!(
                    "{} fraction of execution {:.4} in {} {} (component TPS: {:.1})",
                    self.prefix,
                    time_in_label / self.delta_execution.execution_total_time,
                    label_prefix,
                    other_label,
                    num_txns / time_in_label
                );
            }
        }

        info!(
            "{} fraction of total: {:.4} in ledger update (component TPS: {:.1})",
            self.prefix,
            self.delta_execution.ledger_update_total / self.elapsed,
            num_txns / self.delta_execution.ledger_update_total
        );

        info!(
            "{} fraction of total: {:.4} in commit (component TPS: {:.1})",
            self.prefix,
            self.delta_execution.commit_total_time / self.elapsed,
            num_txns / self.delta_execution.commit_total_time
        );
    }

    pub fn print_end_table(stages: &[Self], overall: &Self) {
        for v in stages.iter().chain(std::iter::once(overall)) {
            println!("{}  {}", v.prefix, v.metadata);
        }
        fn print_one(
            stages: &[OverallMeasurement],
            overall: &OverallMeasurement,
            name: &str,
            fun: impl Fn(&OverallMeasurement) -> String,
        ) {
            println!(
                "{: <12}{}",
                name,
                stages
                    .iter()
                    .chain(std::iter::once(overall))
                    .map(fun)
                    .collect::<String>()
            );
        }

        print_one(stages, overall, "", |v| {
            format!("{: >12}", v.prefix.replace("Staged execution: ", ""))
        });
        print_one(stages, overall, "TPS", |v| {
            format!("{: >12.2}", v.get_tps())
        });
        print_one(stages, overall, "GPS", |v| {
            format!("{: >12.2}", v.get_gps())
        });
        print_one(stages, overall, "effGPS", |v| {
            format!("{: >12.2}", v.get_effective_gps())
        });
        print_one(stages, overall, "GPT", |v| {
            format!("{: >12.2}", v.get_gpt())
        });
        print_one(stages, overall, "ioGPT", |v| {
            format!("{: >12.2}", v.get_io_gpt())
        });
        print_one(stages, overall, "exeGPT", |v| {
            format!("{: >12.2}", v.get_execution_gpt())
        });
    }
}

pub struct EventMeasurements {
    pub staged_events: BTreeMap<(usize, StructTag), usize>,
}

impl EventMeasurements {
    pub fn print_end_table(&self) {
        println!("Events:");
        for ((stage, tag), count) in &self.staged_events {
            println!(
                "stage{: <5}{: >12}     {}::{}::{}",
                stage,
                count,
                if tag.address.is_special() {
                    tag.address.to_standard_string()
                } else {
                    "custom".to_string()
                },
                tag.module,
                tag.name
            );
        }
    }
}
