// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters, hot_state_op_accumulator::BlockHotStateOpAccumulator, types::ReadWriteSummary,
};
use aptos_logger::{error, info};
use aptos_types::{
    fee_statement::FeeStatement,
    on_chain_config::BlockGasLimitType,
    state_store::TStateView,
    transaction::{
        block_epilogue::{BlockEndInfo, TBlockEndInfoExt},
        BlockExecutableTransaction as Transaction,
    },
};
use claims::{assert_le, assert_none};
use once_cell::sync::Lazy;
use std::{collections::BTreeMap, env, time::Instant};

pub static PRINT_CONFLICTS_INFO: Lazy<bool> =
    Lazy::new(|| env::var("PRINT_CONFLICTS_INFO").is_ok());

pub struct BlockGasLimitProcessor<'s, T: Transaction, S> {
    block_gas_limit_type: BlockGasLimitType,
    block_gas_limit_override: Option<u64>,
    accumulated_raw_block_gas: u64,
    accumulated_effective_block_gas: u64,
    accumulated_approx_output_size: u64,
    accumulated_fee_statement: FeeStatement,
    txn_fee_statements: Vec<FeeStatement>,
    txn_read_write_summaries: Vec<ReadWriteSummary<T>>,
    start_time: Instant,
    print_conflicts_info: bool,
    hot_state_op_accumulator: Option<BlockHotStateOpAccumulator<'s, T::Key, S>>,
}

impl<'s, T: Transaction, S: TStateView<Key = T::Key>> BlockGasLimitProcessor<'s, T, S> {
    pub fn new(
        base_view: &'s S,
        block_gas_limit_type: BlockGasLimitType,
        block_gas_limit_override: Option<u64>,
        init_size: usize,
    ) -> Self {
        let hot_state_op_accumulator = block_gas_limit_type
            .add_block_limit_outcome_onchain()
            .then(|| BlockHotStateOpAccumulator::new(base_view));
        Self {
            block_gas_limit_type,
            block_gas_limit_override,
            accumulated_raw_block_gas: 0,
            accumulated_effective_block_gas: 0,
            accumulated_approx_output_size: 0,
            accumulated_fee_statement: FeeStatement::zero(),
            txn_fee_statements: Vec::with_capacity(init_size),
            txn_read_write_summaries: Vec::with_capacity(init_size),
            start_time: Instant::now(),
            // TODO: have a configuration for it.
            print_conflicts_info: *PRINT_CONFLICTS_INFO,
            hot_state_op_accumulator,
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
            let txn_read_write_summary = txn_read_write_summary.expect(
                "txn_read_write_summary needs to be computed if conflict_penalty_window is set",
            );
            if self.print_conflicts_info {
                println!("{:?}", txn_read_write_summary);
            }
            let rw_summary = if self
                .block_gas_limit_type
                .use_granular_resource_group_conflicts()
            {
                txn_read_write_summary
            } else {
                txn_read_write_summary.collapse_resource_group_conflicts()
            };
            if let Some(x) = &mut self.hot_state_op_accumulator {
                x.add_transaction(rw_summary.keys_written(), rw_summary.keys_read());
            }
            self.txn_read_write_summaries.push(rw_summary);
            self.compute_conflict_multiplier(conflict_overlap_length as usize)
        } else {
            assert_none!(txn_read_write_summary);
            1
        };

        // When the accumulated execution and io gas of the committed txns exceeds
        // PER_BLOCK_GAS_LIMIT, early halt BlockSTM. Storage fee does not count towards
        // the per block gas limit, as we measure execution related cost here.
        let raw_gas_used = fee_statement.execution_gas_used()
            * self
                .block_gas_limit_type
                .execution_gas_effective_multiplier()
            + fee_statement.io_gas_used() * self.block_gas_limit_type.io_gas_effective_multiplier();
        self.accumulated_raw_block_gas += raw_gas_used;
        self.accumulated_effective_block_gas += conflict_multiplier * raw_gas_used;

        if self.block_gas_limit_type.block_output_limit().is_some() {
            self.accumulated_approx_output_size += approx_output_size
                .expect("approx_output_size needs to be computed if block_output_limit is set");
        } else {
            assert_none!(approx_output_size);
        }
    }

    fn block_gas_limit(&self) -> Option<u64> {
        if self.block_gas_limit_override.is_some() {
            self.block_gas_limit_override
        } else {
            self.block_gas_limit_type.block_gas_limit()
        }
    }

    fn should_end_block(&mut self, mode: &str) -> bool {
        if let Some(per_block_gas_limit) = self.block_gas_limit() {
            // When the accumulated block gas of the committed txns exceeds
            // PER_BLOCK_GAS_LIMIT, early halt BlockSTM.
            let accumulated_block_gas = self.get_effective_accumulated_block_gas();
            if accumulated_block_gas >= per_block_gas_limit {
                counters::EXCEED_PER_BLOCK_GAS_LIMIT_COUNT
                    .with_label_values(&[mode])
                    .inc();
                info!(
                    "[BlockSTM]: execution ({}) early halted due to \
                    accumulated_block_gas {} >= PER_BLOCK_GAS_LIMIT {}",
                    mode, accumulated_block_gas, per_block_gas_limit,
                );
                return true;
            }
        }

        if let Some(per_block_output_limit) = self.block_gas_limit_type.block_output_limit() {
            let accumulated_output = self.get_accumulated_approx_output_size();
            if accumulated_output >= per_block_output_limit {
                counters::EXCEED_PER_BLOCK_OUTPUT_LIMIT_COUNT
                    .with_label_values(&[mode])
                    .inc();
                info!(
                    "[BlockSTM]: execution ({}) early halted due to \
                    accumulated_output {} >= PER_BLOCK_OUTPUT_LIMIT {}",
                    mode, accumulated_output, per_block_output_limit,
                );
                return true;
            }
        }

        false
    }

    pub(crate) fn should_end_block_parallel(&mut self) -> bool {
        self.should_end_block(counters::Mode::PARALLEL)
    }

    pub(crate) fn should_end_block_sequential(&mut self) -> bool {
        self.should_end_block(counters::Mode::SEQUENTIAL)
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
                if self.print_conflicts_info {
                    println!(
                        "Conflicts with previous: {:?}",
                        current.find_conflicts(prev)
                    );
                }
                conflict_count += 1;
            }
        }
        if self.print_conflicts_info {
            println!(
                "Number of conflicts: {} out of {}",
                conflict_count, conflict_overlap_length
            );
        }
        assert_le!(conflict_count + 1, conflict_overlap_length);
        (conflict_count + 1) as u64
    }

    fn finish_update_counters_and_log_info(
        &self,
        is_parallel: bool,
        num_committed: u32,
        num_total: u32,
        num_workers: usize,
    ) {
        let accumulated_effective_block_gas = self.get_effective_accumulated_block_gas();
        let accumulated_approx_output_size = self.get_accumulated_approx_output_size();
        let accumulated_raw_block_gas = self.accumulated_raw_block_gas;

        counters::update_block_gas_counters(
            &self.accumulated_fee_statement,
            accumulated_effective_block_gas,
            accumulated_approx_output_size,
            num_committed as usize,
            is_parallel,
        );
        counters::update_txn_gas_counters(&self.txn_fee_statements, is_parallel);

        info!(
            effective_block_gas = accumulated_effective_block_gas,
            raw_block_gas = accumulated_raw_block_gas,
            block_gas_limit = self.block_gas_limit_type.block_gas_limit().unwrap_or(0),
            block_gas_limit_override = self.block_gas_limit_override.unwrap_or(0),
            block_gas_limit_exceeded = self
                .block_gas_limit()
                .is_some_and(|limit| accumulated_effective_block_gas >= limit),
            approx_output_size = accumulated_approx_output_size,
            block_output_limit = self.block_gas_limit_type.block_output_limit().unwrap_or(0),
            block_output_limit_exceeded = self
                .block_gas_limit_type
                .block_output_limit()
                .is_some_and(|limit| accumulated_approx_output_size >= limit),
            elapsed_ms = self.start_time.elapsed().as_millis(),
            num_committed = num_committed,
            num_total = num_total,
            num_workers = num_workers,
            "[BlockSTM]: {} execution completed. {} out of {} txns committed",
            if is_parallel {
                format!("Parallel[{}]", num_workers)
            } else {
                "Sequential".to_string()
            },
            num_committed,
            num_total,
        );
    }

    pub(crate) fn finish_parallel_update_counters_and_log_info(
        &self,
        num_committed: u32,
        num_total: u32,
        num_workers: usize,
    ) {
        self.finish_update_counters_and_log_info(true, num_committed, num_total, num_workers)
    }

    pub(crate) fn finish_sequential_update_counters_and_log_info(
        &self,
        num_committed: u32,
        num_total: u32,
    ) {
        self.finish_update_counters_and_log_info(false, num_committed, num_total, 1)
    }

    pub(crate) fn get_block_end_info(&self) -> TBlockEndInfoExt<T::Key> {
        let inner = BlockEndInfo::V0 {
            block_gas_limit_reached: self
                .block_gas_limit()
                .map(|per_block_gas_limit| {
                    self.get_effective_accumulated_block_gas() >= per_block_gas_limit
                })
                .unwrap_or(false),
            block_output_limit_reached: self
                .block_gas_limit_type
                .block_output_limit()
                .map(|per_block_output_limit| {
                    self.get_accumulated_approx_output_size() >= per_block_output_limit
                })
                .unwrap_or(false),
            block_effective_block_gas_units: self.get_effective_accumulated_block_gas(),
            block_approx_output_size: self.get_accumulated_approx_output_size(),
        };
        let slots_to_make_hot = if let Some(x) = &self.hot_state_op_accumulator {
            x.get_slots_to_make_hot()
        } else {
            error!("BlockHotStateOpAccumulator is not set.");
            BTreeMap::new()
        };

        TBlockEndInfoExt::new(inner, slots_to_make_hot)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        proptest_types::types::{KeyType, MockEvent, MockTransaction},
        types::InputOutputKey,
    };
    use aptos_types::state_store::{
        state_storage_usage::StateStorageUsage, state_value::StateValue, StateViewResult,
    };
    use std::collections::HashSet;
    // TODO: add tests for accumulate_fee_statement / compute_conflict_multiplier for different BlockGasLimitType configs

    const DEFAULT_COMPLEX_LIMIT: BlockGasLimitType = BlockGasLimitType::ComplexLimitV1 {
        effective_block_gas_limit: 1000000,
        execution_gas_effective_multiplier: 1,
        io_gas_effective_multiplier: 1,
        conflict_penalty_window: 1,
        use_module_publishing_block_conflict: false,
        block_output_limit: None,
        include_user_txn_size_in_block_output: true,
        add_block_limit_outcome_onchain: false,
        use_granular_resource_group_conflicts: false,
    };

    type TestTxn = MockTransaction<KeyType<u64>, MockEvent>;

    struct MockStateView;
    const EMPTY_STATE_VIEW: MockStateView = MockStateView;

    impl TStateView for MockStateView {
        type Key = KeyType<u64>;

        fn get_state_value(&self, _key: &Self::Key) -> StateViewResult<Option<StateValue>> {
            Ok(None)
        }

        fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
            Ok(StateStorageUsage::zero())
        }
    }

    type TestProcessor<'s> = BlockGasLimitProcessor<'s, TestTxn, MockStateView>;

    #[test]
    fn test_output_limit_not_used() {
        let mut processor = TestProcessor::new(&EMPTY_STATE_VIEW, DEFAULT_COMPLEX_LIMIT, None, 10);
        // Assert passing none here doesn't panic.
        processor.accumulate_fee_statement(FeeStatement::zero(), None, None);
        assert!(!processor.should_end_block_parallel());
    }

    fn execution_fee(execution_gas: u64) -> FeeStatement {
        FeeStatement::new(execution_gas, execution_gas, 0, 0, 0)
    }

    #[test]
    fn test_gas_limit() {
        let block_gas_limit = BlockGasLimitType::ComplexLimitV1 {
            effective_block_gas_limit: 100,
            execution_gas_effective_multiplier: 1,
            io_gas_effective_multiplier: 1,
            conflict_penalty_window: 1,
            use_module_publishing_block_conflict: false,
            block_output_limit: None,
            include_user_txn_size_in_block_output: true,
            add_block_limit_outcome_onchain: false,
            use_granular_resource_group_conflicts: false,
        };

        let mut processor = TestProcessor::new(&EMPTY_STATE_VIEW, block_gas_limit, None, 10);

        processor.accumulate_fee_statement(execution_fee(10), None, None);
        assert!(!processor.should_end_block_parallel());
        processor.accumulate_fee_statement(execution_fee(50), None, None);
        assert!(!processor.should_end_block_parallel());
        processor.accumulate_fee_statement(execution_fee(40), None, None);
        assert!(processor.should_end_block_parallel());
    }

    #[test]
    fn test_output_limit_used() {
        let block_gas_limit = BlockGasLimitType::ComplexLimitV1 {
            effective_block_gas_limit: 1000000,
            execution_gas_effective_multiplier: 1,
            io_gas_effective_multiplier: 1,
            conflict_penalty_window: 1,
            use_module_publishing_block_conflict: false,
            block_output_limit: Some(100),
            include_user_txn_size_in_block_output: true,
            add_block_limit_outcome_onchain: false,
            use_granular_resource_group_conflicts: false,
        };

        let mut processor = TestProcessor::new(&EMPTY_STATE_VIEW, block_gas_limit, None, 10);

        processor.accumulate_fee_statement(FeeStatement::zero(), None, Some(10));
        assert_eq!(processor.accumulated_approx_output_size, 10);
        assert!(!processor.should_end_block_parallel());
        processor.accumulate_fee_statement(FeeStatement::zero(), None, Some(50));
        assert_eq!(processor.accumulated_approx_output_size, 60);
        assert!(!processor.should_end_block_parallel());
        processor.accumulate_fee_statement(FeeStatement::zero(), None, Some(40));
        assert_eq!(processor.accumulated_approx_output_size, 100);
        assert!(processor.should_end_block_parallel());
    }

    fn to_map(reads: &[InputOutputKey<u64, u32>]) -> HashSet<InputOutputKey<KeyType<u64>, u32>> {
        reads
            .iter()
            .map(|key| match key {
                InputOutputKey::Resource(k) => InputOutputKey::Resource(KeyType(*k, false)),
                InputOutputKey::Group(k, t) => InputOutputKey::Group(KeyType(*k, false), *t),
                InputOutputKey::DelayedField(i) => InputOutputKey::DelayedField(*i),
            })
            .collect()
    }

    #[test]
    fn test_conflict_limit_coarse_resource_groups() {
        let block_gas_limit = BlockGasLimitType::ComplexLimitV1 {
            effective_block_gas_limit: 1000,
            execution_gas_effective_multiplier: 1,
            io_gas_effective_multiplier: 1,
            conflict_penalty_window: 8,
            use_module_publishing_block_conflict: false,
            block_output_limit: None,
            include_user_txn_size_in_block_output: true,
            add_block_limit_outcome_onchain: false,
            use_granular_resource_group_conflicts: false,
        };

        let mut processor = TestProcessor::new(&EMPTY_STATE_VIEW, block_gas_limit, None, 10);

        processor.accumulate_fee_statement(
            execution_fee(10),
            Some(ReadWriteSummary::new(
                to_map(&[InputOutputKey::Resource(1)]),
                to_map(&[InputOutputKey::Resource(1)]),
            )),
            None,
        );
        assert_eq!(1, processor.compute_conflict_multiplier(8));
        assert_eq!(processor.accumulated_effective_block_gas, 10);
        assert!(!processor.should_end_block_parallel());
        processor.accumulate_fee_statement(
            execution_fee(10),
            Some(ReadWriteSummary::new(
                to_map(&[InputOutputKey::Resource(1)]),
                to_map(&[InputOutputKey::Group(1, 1)]),
            )),
            None,
        );
        assert_eq!(2, processor.compute_conflict_multiplier(8));
        assert_eq!(processor.accumulated_effective_block_gas, 30);
        assert!(!processor.should_end_block_parallel());
        processor.accumulate_fee_statement(
            execution_fee(10),
            Some(ReadWriteSummary::new(
                to_map(&[InputOutputKey::Group(2, 1)]),
                to_map(&[InputOutputKey::Group(2, 1)]),
            )),
            None,
        );
        assert_eq!(1, processor.compute_conflict_multiplier(8));
        assert_eq!(processor.accumulated_effective_block_gas, 40);
        assert!(!processor.should_end_block_parallel());
        processor.accumulate_fee_statement(
            execution_fee(10),
            Some(ReadWriteSummary::new(
                to_map(&[InputOutputKey::Group(2, 2)]),
                to_map(&[InputOutputKey::Group(2, 2)]),
            )),
            None,
        );
        assert_eq!(2, processor.compute_conflict_multiplier(8));
        assert_eq!(processor.accumulated_effective_block_gas, 60);
        assert!(!processor.should_end_block_parallel());
    }

    #[test]
    fn test_conflict_limit_granular_resource_groups() {
        let block_gas_limit = BlockGasLimitType::ComplexLimitV1 {
            effective_block_gas_limit: 1000,
            execution_gas_effective_multiplier: 1,
            io_gas_effective_multiplier: 1,
            conflict_penalty_window: 8,
            use_module_publishing_block_conflict: false,
            block_output_limit: None,
            include_user_txn_size_in_block_output: true,
            add_block_limit_outcome_onchain: false,
            use_granular_resource_group_conflicts: true,
        };

        let mut processor = TestProcessor::new(&EMPTY_STATE_VIEW, block_gas_limit, None, 10);

        assert!(!processor.should_end_block_parallel());
        processor.accumulate_fee_statement(
            execution_fee(10),
            Some(ReadWriteSummary::new(
                to_map(&[InputOutputKey::Group(2, 1)]),
                to_map(&[InputOutputKey::Group(2, 1)]),
            )),
            None,
        );
        assert_eq!(1, processor.compute_conflict_multiplier(8));
        assert_eq!(processor.accumulated_effective_block_gas, 10);
        assert!(!processor.should_end_block_parallel());
        processor.accumulate_fee_statement(
            execution_fee(10),
            Some(ReadWriteSummary::new(
                to_map(&[InputOutputKey::Group(2, 2)]),
                to_map(&[InputOutputKey::Group(2, 2)]),
            )),
            None,
        );
        assert_eq!(1, processor.compute_conflict_multiplier(8));
        assert_eq!(processor.accumulated_effective_block_gas, 20);
        assert!(!processor.should_end_block_parallel());
    }
}
