// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{counters, types::ReadWriteSummary};
use aptos_logger::info;
use aptos_types::{
    fee_statement::FeeStatement, on_chain_config::BlockGasLimitType,
    transaction::BlockExecutableTransaction as Transaction,
};
use claims::assert_le;

pub struct BlockGasLimitProcessor<T: Transaction> {
    block_gas_limit_type: BlockGasLimitType,
    accumulated_effective_block_gas: u64,
    accumulated_approx_output_size: u64,
    accumulated_fee_statement: FeeStatement,
    txn_fee_statements: Vec<FeeStatement>,
    txn_read_write_summaries: Vec<ReadWriteSummary<T>>,
}

impl<T: Transaction> BlockGasLimitProcessor<T> {
    pub fn new(block_gas_limit_type: BlockGasLimitType, init_size: usize) -> Self {
        Self {
            block_gas_limit_type,
            accumulated_effective_block_gas: 0,
            accumulated_approx_output_size: 0,
            accumulated_fee_statement: FeeStatement::zero(),
            txn_fee_statements: Vec::with_capacity(init_size),
            txn_read_write_summaries: Vec::with_capacity(init_size),
        }
    }

    pub(crate) fn accumulate_fee_statement(
        &mut self,
        fee_statement: FeeStatement,
        txn_read_write_summary: Option<ReadWriteSummary<T>>,
        approx_output_size: Option<u64>,
    ) {
        self.accumulated_fee_statement
            .add_fee_statement(&fee_statement);
        self.txn_fee_statements.push(fee_statement);

        let conflict_multiplier = if let Some(conflict_overlap_length) =
            self.block_gas_limit_type.conflict_penalty_window()
        {
            self.txn_read_write_summaries.push(
                if self
                    .block_gas_limit_type
                    .use_granular_resource_group_conflicts()
                {
                    txn_read_write_summary.unwrap()
                } else {
                    txn_read_write_summary
                        .unwrap()
                        .collapse_resource_group_conflicts()
                },
            );
            self.compute_conflict_multiplier(conflict_overlap_length as usize)
        } else {
            1
        };

        // println!("[{}] Multiplier {}, with read/write summary: {:?}", self.txn_fee_statements.len() - 1, conflict_multiplier, self.txn_read_write_summaries.last());

        // When the accumulated execution and io gas of the committed txns exceeds
        // PER_BLOCK_GAS_LIMIT, early halt BlockSTM. Storage fee does not count towards
        // the per block gas limit, as we measure execution related cost here.
        self.accumulated_effective_block_gas += conflict_multiplier
            * (fee_statement.execution_gas_used()
                * self
                    .block_gas_limit_type
                    .execution_gas_effective_multiplier()
                + fee_statement.io_gas_used()
                    * self.block_gas_limit_type.io_gas_effective_multiplier());

        self.accumulated_approx_output_size += approx_output_size.unwrap_or(0);
    }

    fn should_end_block(&self, is_parallel: bool) -> bool {
        let mode = if is_parallel {
            counters::Mode::PARALLEL
        } else {
            counters::Mode::SEQUENTIAL
        };
        if let Some(per_block_gas_limit) = self.block_gas_limit_type.block_gas_limit() {
            // When the accumulated block gas of the committed txns exceeds
            // PER_BLOCK_GAS_LIMIT, early halt BlockSTM.
            let accumulated_block_gas = self.get_effective_accumulated_block_gas();
            if accumulated_block_gas >= per_block_gas_limit {
                counters::EXCEED_PER_BLOCK_GAS_LIMIT_COUNT
                    .with_label_values(&[mode])
                    .inc();
                info!(
                    "[BlockSTM]: execution (is_parallel = {}) early halted due to \
                    accumulated_block_gas {} >= PER_BLOCK_GAS_LIMIT {}",
                    is_parallel, accumulated_block_gas, per_block_gas_limit,
                );

                return true;
            }
        }

        if let Some(per_block_output_limit) = self.block_gas_limit_type.block_output_limit() {
            let accumulated_output = self.get_accumulated_approx_output_size();
            if accumulated_output >= per_block_output_limit {
                counters::EXCEED_PER_BLOCK_GAS_LIMIT_COUNT
                    .with_label_values(&[mode])
                    .inc();
                info!(
                    "[BlockSTM]: execution (is_parallel = {}) early halted due to \
                    accumulated_output {} >= PER_BLOCK_OUTPUT_LIMIT {}",
                    is_parallel, accumulated_output, per_block_output_limit,
                );

                return true;
            }
        }

        false
    }

    pub(crate) fn should_end_block_parallel(&self) -> bool {
        self.should_end_block(true)
    }

    pub(crate) fn should_end_block_sequential(&self) -> bool {
        self.should_end_block(false)
    }

    fn get_effective_accumulated_block_gas(&self) -> u64 {
        self.accumulated_effective_block_gas
    }

    fn get_accumulated_approx_output_size(&self) -> u64 {
        self.accumulated_approx_output_size
    }

    fn compute_conflict_multiplier(&self, conflict_overlap_length: usize) -> u64 {
        let start = self
            .txn_read_write_summaries
            .len()
            .saturating_sub(conflict_overlap_length);
        let end = self.txn_read_write_summaries.len() - 1;

        let mut conflict_count = 0;
        let current = &self.txn_read_write_summaries[end];
        for prev in &self.txn_read_write_summaries[start..end] {
            if current.conflicts_with_previous(prev) {
                conflict_count += 1;
            }
        }
        assert_le!(conflict_count + 1, conflict_overlap_length);
        (conflict_count + 1) as u64
    }

    fn finish_update_counters_and_log_info(
        &self,
        is_parallel: bool,
        num_committed: u32,
        num_total: u32,
    ) {
        let accumulated_effective_block_gas = self.get_effective_accumulated_block_gas();
        let accumulated_approx_output_size = self.get_accumulated_approx_output_size();

        counters::update_block_gas_counters(
            &self.accumulated_fee_statement,
            accumulated_effective_block_gas,
            accumulated_approx_output_size,
            num_committed as usize,
            is_parallel,
        );
        counters::update_txn_gas_counters(&self.txn_fee_statements, is_parallel);

        info!(
            "[BlockSTM]: {} execution completed. {} out of {} txns committed. \
            accumulated_effective_block_gas = {}, limit = {:?}",
            if is_parallel {
                "Parallel"
            } else {
                "Sequential"
            },
            num_committed,
            num_total,
            accumulated_effective_block_gas,
            self.block_gas_limit_type,
        );
    }

    pub(crate) fn finish_parallel_update_counters_and_log_info(
        &self,
        num_committed: u32,
        num_total: u32,
    ) {
        self.finish_update_counters_and_log_info(true, num_committed, num_total)
    }

    pub(crate) fn finish_sequential_update_counters_and_log_info(
        &self,
        num_committed: u32,
        num_total: u32,
    ) {
        self.finish_update_counters_and_log_info(false, num_committed, num_total)
    }
}

// TODO: add tests for accumulate_fee_statement / compute_conflict_multiplier for different BlockGasLimitType configs
