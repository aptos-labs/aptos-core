// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    types::{InputOutputKey, ReadWriteSummary},
};
use aptos_logger::info;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    fee_statement::FeeStatement,
    on_chain_config::BlockGasLimitType,
    transaction::{block_epilogue::BlockEndInfo, BlockExecutableTransaction as Transaction},
    validator_txn::Topic,
};
use claims::{assert_le, assert_none};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    time::Instant,
};

pub enum ObjectStatus {
    Empty,
    Written,
    Waiting,
}

pub struct ObjectData {
    status: ObjectStatus,
    reader_txns: Vec<TxnIndex>,
    waiting_txns: Vec<TxnIndex>,
}

#[derive(Clone)]
pub enum TxnStatus {
    Fresh,
    Failed,
    Finished,
    Waiting,
}

impl ObjectData {
    pub fn new() -> Self {
        Self {
            status: ObjectStatus::Empty,
            reader_txns: Vec::new(),
            waiting_txns: Vec::new(),
        }
    }
}

#[derive(Copy)]
pub enum Event {
    END,
    READ(u32),
    WRITE(u32),
}

pub struct Simulation<T: Transaction> {
    accumulated_effective_block_gas: u64,

    num_objects: u32,
    objects: Vec<ObjectData>,

    object_indexes: HashMap<InputOutputKey<T::Key, T::Tag, T::Identifier>, u32>,
    // each transaction keeps vector of all events, sorted by gas
    events: Vec<Vec<(u64, Event)>>,
    // need to compute events from the following reads/writes, maybe be forced to keep them
    // reads: Vec<HashSet<(InputOutputKey<T::Key, T::Tag, T::Identifier>, TxnIndex)>>,
    // writes: Vec<HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>>>,
    transaction_pq: BinaryHeap<TxnIndex>,
    // this allows to track by how much should gas be shifted by the current transaction
    offset_gas: Vec<u64>,
    //next event to be added to PQ
    cur_event: Vec<usize>,
    // are we really using incarnations at all?
    // whenever element is removed of txn_pq,
    // insert its first event in the event_pq
    txn_pq: BinaryHeap<TxnIndex>,
    // only keep  at most one event from each transaction
    // whenever removed, use offset_gas and cur_event to insert next event from the same transaction
    event_pq: BinaryHeap<Reverse<(u64, TxnIndex)>>,

    txn_status: Vec<TxnStatus>,
    first_write: Vec<usize>,
    total_num_workers: usize,
    total_num_txns: u32,
    cur_num_txns: u32,
}

impl<T: Transaction> Simulation<T> {
    fn new(init_size: usize, num_workers: usize) -> Self {
        Self {
            // versioned_objects: HashMap<(InputOutputKey<T::Key, T::Tag, T::Identifier>, TxnIndex), ObjectData>::new(),
            accumulated_effective_block_gas: 0,
            num_objects: 0,
            objects: Vec::new(),
            object_indexes: HashMap::new(),
            events: Vec::new(),
            transaction_pq: BinaryHeap::new(),
            offset_gas: vec![0u64; init_size],
            cur_event: vec![0; init_size],
            txn_pq: BinaryHeap::new(),
            event_pq: BinaryHeap::new(),
            txn_status: vec![TxnStatus::Fresh; init_size],
            first_write: Vec::with_capacity(init_size),
            total_num_workers: num_workers,
            total_num_txns: init_size as u32,
            cur_num_txns: 0,
        }
    }

    fn compute_objects_and_events(
        &mut self,
        fee_statement: FeeStatement,
        txn_read_write_summary: ReadWriteSummary<T>,
        txn_idx: TxnIndex,
    ) {
        let (reads, writes) = txn_read_write_summary.get_summary();

        self.events.push(Vec::new());
        for key in reads {
            if let Some(object_index) = self.object_indexes.get(&key) {
                self.events[txn_idx as usize].push((0, Event::READ(*object_index)));
            }
        }

        self.first_write[txn_idx as usize] = self.events.len();
        let exec_gas = fee_statement.execution_gas_used();
        for key in writes {
            self.object_indexes.insert(key, self.num_objects);
            self.events[txn_idx as usize].push((exec_gas, Event::WRITE(self.num_objects)));
            self.objects.push(ObjectData::new());
            self.num_objects += 1;
        }

        self.events[txn_idx as usize].push((exec_gas, Event::END));
    }

    fn run_simulation(&mut self) {
        loop {
            if let Some(Reverse((gas_value, txn_idx))) = self.event_pq.pop() {
                let (cur_gas, cur_event) =
                    self.events[txn_idx as usize][self.cur_event[txn_idx as usize]];
                match cur_event {
                    Event::END => match self.txn_status[txn_idx as usize] {
                        TxnStatus::Fresh => {
                            self.txn_status[txn_idx as usize] = TxnStatus::Finished;
                        },
                    },
                }
            }
        }
    }

    fn update(
        &mut self,
        fee_statement: FeeStatement,
        txn_read_write_summary: ReadWriteSummary<T>,
    ) -> u64 {
        assert!(self.cur_num_txns < self.total_num_txns);
        let txn_idx = self.cur_num_txns;
        self.cur_num_txns += 1;

        self.compute_objects_and_events(fee_statement, txn_read_write_summary, txn_idx);

        self.txn_pq.push(txn_idx);
        if self.cur_num_txns < self.total_num_workers as u32 {
            return 0;
        }

        if self.cur_num_txns == self.total_num_workers as u32 {
            //initial phase, populate event_pq
            for i in 0..self.total_num_workers {
                self.event_pq.push(Reverse((self.events[i][0].0, txn_idx)));
            }
        } else {
            //insert new transacation in txn_pq
            self.txn_pq.push(txn_idx);
        }

        self.run_simulation();
        self.accumulated_effective_block_gas
    }
}

struct Heuristics<T: Transaction> {
    accumulated_effective_block_gas: u64,
    txn_read_write_summaries: Vec<ReadWriteSummary<T>>,
}

impl<T: Transaction> Heuristics<T> {
    fn new(init_size: usize) -> Self {
        Self {
            // versioned_objects: HashMap<(InputOutputKey<T::Key, T::Tag, T::Identifier>, TxnIndex), ObjectData>::new(),
            accumulated_effective_block_gas: 0,
            txn_read_write_summaries: Vec::with_capacity(init_size),
        }
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

    fn update(
        &mut self,
        block_gas_limit_type: &BlockGasLimitType,
        fee_statement: FeeStatement,
        txn_read_write_summary: Option<ReadWriteSummary<T>>,
        module_rw_conflict: bool,
    ) -> u64 {
        let conflict_multiplier =
            if let Some(conflict_overlap_length) = block_gas_limit_type.conflict_penalty_window() {
                let txn_read_write_summary = txn_read_write_summary.expect(
                    "txn_read_write_summary needs to be computed if conflict_penalty_window is set",
                );
                self.txn_read_write_summaries.push(
                    if block_gas_limit_type.use_granular_resource_group_conflicts() {
                        txn_read_write_summary
                    } else {
                        txn_read_write_summary.collapse_resource_group_conflicts()
                    },
                );
                if module_rw_conflict {
                    conflict_overlap_length as u64
                } else {
                    self.compute_conflict_multiplier(conflict_overlap_length as usize)
                }
            } else {
                assert_none!(txn_read_write_summary);
                1
            };

        // When the accumulated execution and io gas of the committed txns exceeds
        // PER_BLOCK_GAS_LIMIT, early halt BlockSTM. Storage fee does not count towards
        // the per block gas limit, as we measure execution related cost here.
        self.accumulated_effective_block_gas += conflict_multiplier
            * (fee_statement.execution_gas_used()
                * block_gas_limit_type.execution_gas_effective_multiplier()
                + fee_statement.io_gas_used() * block_gas_limit_type.io_gas_effective_multiplier());

        self.accumulated_effective_block_gas
    }
}

enum AccumulatedEffectiveBlockGasCalculator<T: Transaction> {
    UseHeuristics(Heuristics<T>),
    UseSimulation(Simulation<T>),
}

impl<T: Transaction> AccumulatedEffectiveBlockGasCalculator<T> {
    fn new_simulation(init_size: usize, num_workers: usize) -> Self {
        Self::UseSimulation(Simulation::new(init_size, num_workers))
    }

    fn new_heuristics(init_size: usize) -> Self {
        Self::UseHeuristics(Heuristics::new(init_size))
    }
}

pub struct BlockGasLimitProcessor<T: Transaction> {
    block_gas_limit_type: BlockGasLimitType,
    accumulated_effective_block_gas: u64,
    accumulated_effective_block_gas_calculator: AccumulatedEffectiveBlockGasCalculator<T>,
    accumulated_approx_output_size: u64,
    accumulated_fee_statement: FeeStatement,
    txn_fee_statements: Vec<FeeStatement>,
    module_rw_conflict: bool,
    start_time: Instant,
}

impl<T: Transaction> BlockGasLimitProcessor<T> {
    pub fn new(
        block_gas_limit_type: BlockGasLimitType,
        init_size: usize,
        num_workers: usize,
    ) -> Self {
        let is_simulation = block_gas_limit_type.is_simulation();
        Self {
            block_gas_limit_type,
            accumulated_effective_block_gas: 0,
            accumulated_effective_block_gas_calculator: if is_simulation {
                AccumulatedEffectiveBlockGasCalculator::new_simulation(init_size, num_workers)
            } else {
                AccumulatedEffectiveBlockGasCalculator::new_heuristics(init_size)
            },
            accumulated_approx_output_size: 0,
            accumulated_fee_statement: FeeStatement::zero(),
            txn_fee_statements: Vec::with_capacity(init_size),
            module_rw_conflict: false,
            start_time: Instant::now(),
        }
    }

    #[cfg(test)]
    fn compute_conflict_multiplier(&self, conflict_overlap_length: usize) -> u64 {
        match &self.accumulated_effective_block_gas_calculator {
            AccumulatedEffectiveBlockGasCalculator::UseHeuristics(heuristics) => {
                heuristics.compute_conflict_multiplier(conflict_overlap_length)
            },
            AccumulatedEffectiveBlockGasCalculator::UseSimulation(_) => {
                unreachable!("simulation does not use conflict multiplier");
            },
        }
    }

    fn update_accumulated_effective_block_gas(
        &mut self,
        fee_statement: FeeStatement,
        txn_read_write_summary: Option<ReadWriteSummary<T>>,
    ) {
        self.accumulated_effective_block_gas = match &mut self
            .accumulated_effective_block_gas_calculator
        {
            AccumulatedEffectiveBlockGasCalculator::UseHeuristics(heuristics) => heuristics.update(
                &self.block_gas_limit_type,
                fee_statement,
                txn_read_write_summary,
                self.module_rw_conflict,
            ),
            AccumulatedEffectiveBlockGasCalculator::UseSimulation(simulation) => {
                assert!(
                    txn_read_write_summary.is_some(),
                    "simulation needs read/write summary"
                );
                if self
                    .block_gas_limit_type
                    .use_granular_resource_group_conflicts()
                {
                    simulation.update(fee_statement, txn_read_write_summary.unwrap())
                } else {
                    simulation.update(
                        fee_statement,
                        txn_read_write_summary
                            .unwrap()
                            .collapse_resource_group_conflicts(),
                    )
                }
            },
        };
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

        self.update_accumulated_effective_block_gas(fee_statement, txn_read_write_summary);

        if self.block_gas_limit_type.block_output_limit().is_some() {
            self.accumulated_approx_output_size += approx_output_size
                .expect("approx_output_size needs to be computed if block_output_limit is set");
        } else {
            assert_none!(approx_output_size);
        }
    }

    pub(crate) fn process_module_rw_conflict(&mut self) {
        if self.block_gas_limit_type.is_simulation() {
            return;
        }

        if self.module_rw_conflict
            || !self
                .block_gas_limit_type
                .use_module_publishing_block_conflict()
        {
            return;
        }

        let conflict_multiplier = if let Some(conflict_overlap_length) =
            self.block_gas_limit_type.conflict_penalty_window()
        {
            conflict_overlap_length
        } else {
            return;
        };

        self.accumulated_effective_block_gas = conflict_multiplier as u64
            * (self.accumulated_fee_statement.execution_gas_used()
                * self
                    .block_gas_limit_type
                    .execution_gas_effective_multiplier()
                + self.accumulated_fee_statement.io_gas_used()
                    * self.block_gas_limit_type.io_gas_effective_multiplier());
        self.module_rw_conflict = true;
    }

    fn should_end_block(&mut self, mode: &str) -> bool {
        if let Some(per_block_gas_limit) = self.block_gas_limit_type.block_gas_limit() {
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

    fn finish_update_counters_and_log_info(
        &self,
        is_parallel: bool,
        num_committed: u32,
        num_total: u32,
        num_workers: usize,
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
            effective_block_gas = accumulated_effective_block_gas,
            block_gas_limit = self.block_gas_limit_type.block_gas_limit().unwrap_or(0),
            block_gas_limit_exceeded = self
                .block_gas_limit_type
                .block_gas_limit()
                .map_or(false, |limit| accumulated_effective_block_gas >= limit),
            approx_output_size = accumulated_approx_output_size,
            block_output_limit = self.block_gas_limit_type.block_output_limit().unwrap_or(0),
            block_output_limit_exceeded = self
                .block_gas_limit_type
                .block_output_limit()
                .map_or(false, |limit| accumulated_approx_output_size >= limit),
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

    pub(crate) fn get_block_end_info(&self) -> BlockEndInfo {
        BlockEndInfo::V0 {
            block_gas_limit_reached: self
                .block_gas_limit_type
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
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        proptest_types::types::{KeyType, MockEvent, MockTransaction},
        types::InputOutputKey,
    };
    use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
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

    #[test]
    fn test_output_limit_not_used() {
        let mut processor = BlockGasLimitProcessor::<TestTxn>::new(DEFAULT_COMPLEX_LIMIT, 10, 1);
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

        let mut processor = BlockGasLimitProcessor::<TestTxn>::new(block_gas_limit, 10, 1);

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

        let mut processor = BlockGasLimitProcessor::<TestTxn>::new(block_gas_limit, 10, 1);

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

    fn to_map(
        reads: &[InputOutputKey<u64, u32, u64>],
    ) -> HashSet<InputOutputKey<KeyType<u64>, u32, DelayedFieldID>> {
        reads
            .iter()
            .map(|key| match key {
                InputOutputKey::Resource(k) => InputOutputKey::Resource(KeyType(*k, false)),
                InputOutputKey::Group(k, t) => InputOutputKey::Group(KeyType(*k, false), *t),
                InputOutputKey::DelayedField(i) => InputOutputKey::DelayedField((*i).into()),
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

        let mut processor = BlockGasLimitProcessor::<TestTxn>::new(block_gas_limit, 10, 1);

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

        let mut processor = BlockGasLimitProcessor::<TestTxn>::new(block_gas_limit, 10, 1);

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

    #[test]
    fn test_module_publishing_txn_conflict() {
        let conflict_penalty_window = 4;
        let block_gas_limit = BlockGasLimitType::ComplexLimitV1 {
            effective_block_gas_limit: 1000,
            execution_gas_effective_multiplier: 1,
            io_gas_effective_multiplier: 1,
            conflict_penalty_window,
            use_module_publishing_block_conflict: true,
            block_output_limit: None,
            include_user_txn_size_in_block_output: true,
            add_block_limit_outcome_onchain: false,
            use_granular_resource_group_conflicts: true,
        };

        let mut processor = BlockGasLimitProcessor::<TestTxn>::new(block_gas_limit, 10, 1);
        processor.accumulate_fee_statement(
            execution_fee(10),
            Some(ReadWriteSummary::new(
                to_map(&[InputOutputKey::Group(2, 2)]),
                to_map(&[InputOutputKey::Group(2, 2)]),
            )),
            None,
        );
        processor.accumulate_fee_statement(
            execution_fee(20),
            Some(ReadWriteSummary::new(
                to_map(&[InputOutputKey::Group(1, 1)]),
                to_map(&[InputOutputKey::Group(1, 1)]),
            )),
            None,
        );
        assert_eq!(1, processor.compute_conflict_multiplier(8));
        assert_eq!(processor.accumulated_effective_block_gas, 30);

        processor.process_module_rw_conflict();
        assert_eq!(
            processor.accumulated_effective_block_gas,
            30 * conflict_penalty_window as u64
        );

        processor.accumulate_fee_statement(
            execution_fee(25),
            Some(ReadWriteSummary::new(
                to_map(&[InputOutputKey::Group(1, 1)]),
                to_map(&[InputOutputKey::Group(1, 1)]),
            )),
            None,
        );
        assert_eq!(
            processor.accumulated_effective_block_gas,
            55 * conflict_penalty_window as u64
        );
    }
}
