// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::{
        clone_exec_coverage_map, collect_coverage_keys, count_coverage_entries, coverage_delta,
        merge_coverage,
        sequence::{ExecResourceProfile, SeedInput},
        tracing::{ResourceWrite, TracingExecutor},
    },
    mutate::mutator::{Mutator, TypePool},
    prep::canvas::{BasicInput, ScriptSignature},
    state::{PersistedObjectState, PersistedOneshotSeedRecord, PersistedSeedInput},
};
use anyhow::Result;
use aptos_types::transaction::{
    ExecutionStatus, Script, TransactionArgument, TransactionPayload, TransactionStatus,
};
use move_core_types::{
    language_storage::TypeTag as VmTypeTag,
    value::MoveValue,
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use move_coverage::coverage_map::{CoverageMap, ExecCoverageMap};
use move_vm_runtime::tracing::{clear_tracing_buffer, flush_tracing_buffer};
use rand::Rng;
use std::{collections::BTreeSet, fmt::Display, path::PathBuf, time::Instant};

const MAX_ONESHOT_CORPUS: usize = 256;

#[derive(Clone)]
struct SeedRecord {
    input: SeedInput,
    score: u32,
    last_used_at: u64,
}

/// Status of one execution
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ExecStatus {
    Success,
    AbortIntrinsic {
        status_code: StatusCode,
        sub_status: Option<u64>,
        location: AbortLocation,
        function: u16,
        instruction: u16,
    },
    AbortDeclared {
        abort_code: u64,
        location: AbortLocation,
    },
    ErrorKept {
        status_code: StatusCode,
        sub_status: Option<u64>,
    },
    ErrorDiscard {
        status_code: StatusCode,
        sub_status: Option<u64>,
    },
    OutOfGas,
}

impl Into<ExecStatus> for (VMStatus, TransactionStatus) {
    fn into(self) -> ExecStatus {
        match self {
            (VMStatus::Executed, TransactionStatus::Keep(ExecutionStatus::Success)) => {
                ExecStatus::Success
            },
            (
                VMStatus::Error {
                    status_code,
                    sub_status,
                    message: _,
                },
                TransactionStatus::Discard(code),
            ) => {
                assert_eq!(status_code, code);
                ExecStatus::ErrorDiscard {
                    status_code,
                    sub_status,
                }
            },
            (
                VMStatus::Error {
                    status_code,
                    sub_status,
                    message: _,
                },
                TransactionStatus::Keep(ExecutionStatus::OutOfGas),
            ) => {
                assert_eq!(status_code, StatusCode::OUT_OF_GAS);
                assert!(sub_status.is_none());
                ExecStatus::OutOfGas
            },
            (
                VMStatus::Error {
                    status_code,
                    sub_status,
                    message: _,
                },
                TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(code))),
            ) => {
                assert_eq!(status_code, code);
                ExecStatus::ErrorKept {
                    status_code,
                    sub_status,
                }
            },
            (
                VMStatus::MoveAbort {
                    location,
                    code: abort_code,
                    message: _,
                },
                TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                    location: loc,
                    code,
                    info: _,
                }),
            ) => {
                assert_eq!(location, loc);
                assert_eq!(abort_code, code);
                ExecStatus::AbortDeclared {
                    abort_code,
                    location,
                }
            },
            (
                VMStatus::ExecutionFailure {
                    status_code,
                    sub_status,
                    location,
                    function,
                    code_offset: instruction,
                    message: _,
                },
                TransactionStatus::Keep(ExecutionStatus::ExecutionFailure {
                    location: loc,
                    function: func,
                    code_offset,
                }),
            ) => {
                assert_eq!(location, loc);
                assert_eq!(function, func);
                assert_eq!(instruction, code_offset);
                ExecStatus::AbortIntrinsic {
                    status_code,
                    sub_status,
                    location,
                    function,
                    instruction,
                }
            },
            (vm_status, txn_status) => {
                panic!("invalid status combination: {vm_status:?} and {txn_status:?}");
            },
        }
    }
}

impl ExecStatus {
    /// Return a short category label for aggregated reporting
    pub fn category(&self) -> &'static str {
        match self {
            ExecStatus::Success => "success",
            ExecStatus::AbortIntrinsic { .. } => "abort",
            ExecStatus::AbortDeclared { .. } => "abort",
            ExecStatus::ErrorKept { .. } => "error",
            ExecStatus::ErrorDiscard { .. } => "discard",
            ExecStatus::OutOfGas => "out-of-gas",
        }
    }

    pub fn is_missing_data(&self) -> bool {
        match self {
            ExecStatus::AbortIntrinsic { status_code, .. }
            | ExecStatus::ErrorKept { status_code, .. }
            | ExecStatus::ErrorDiscard { status_code, .. } => {
                *status_code == StatusCode::MISSING_DATA
            },
            ExecStatus::Success | ExecStatus::AbortDeclared { .. } | ExecStatus::OutOfGas => false,
        }
    }
}

impl Display for ExecStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecStatus::Success => write!(f, "success"),
            ExecStatus::AbortIntrinsic {
                status_code,
                sub_status,
                location,
                function,
                instruction,
            } => match sub_status {
                None => write!(
                    f,
                    "abort({status_code:?} @ {location}::{function}::{instruction})"
                ),
                Some(v) => write!(
                    f,
                    "abort({status_code:?}::{v} @ {location}::{function}::{instruction})"
                ),
            },
            ExecStatus::AbortDeclared {
                location,
                abort_code,
            } => write!(f, "abort({abort_code} @ {location})"),
            ExecStatus::ErrorKept {
                status_code,
                sub_status,
            } => match sub_status {
                None => write!(f, "error({status_code:?})"),
                Some(v) => write!(f, "error({status_code:?}::{v})"),
            },
            ExecStatus::ErrorDiscard {
                status_code,
                sub_status,
            } => match sub_status {
                None => write!(f, "discard({status_code:?})"),
                Some(v) => write!(f, "discard({status_code:?}::{v})"),
            },
            ExecStatus::OutOfGas => write!(f, "out-of-gas"),
        }
    }
}

/// A one-shot fuzzer
pub struct OneshotFuzzer {
    script_index: usize,
    script_sig: ScriptSignature,
    script_code: Vec<u8>,
    executor: TracingExecutor,
    mutator: Mutator,
    trace_path: PathBuf,
    coverage: ExecCoverageMap,
    seedpool: Vec<SeedRecord>,
    replay_log: Vec<SeedInput>,

    // statistics counting
    exec_count: u64,
    last_new_coverage_time: Option<Instant>,
    coverage_at_last_report: usize,
}

impl OneshotFuzzer {
    /// Create a new one-shot fuzzer
    pub fn new(
        executor: TracingExecutor,
        seed: u64,
        script_index: usize,
        script_sig: ScriptSignature,
        script_code: Vec<u8>,
        type_pool: TypePool,
        trace_path: PathBuf,
        dict_string: Vec<String>,
    ) -> Self {
        // prepare the fuzzer
        Self {
            mutator: Mutator::new(
                seed,
                executor.all_addresses_by_kind(),
                type_pool,
                dict_string,
            ),
            executor,
            script_index,
            script_sig,
            script_code,
            trace_path,
            coverage: ExecCoverageMap::new(String::new()),
            seedpool: vec![],
            replay_log: vec![],
            exec_count: 0,
            last_new_coverage_time: None,
            coverage_at_last_report: 0,
        }
    }

    /// Get the core of the script
    pub fn script_desc(&self) -> String {
        self.script_sig.ident.to_string()
    }

    /// Get the current corpus (seed pool) size
    pub fn corpus_size(&self) -> usize {
        self.seedpool.len()
    }

    fn pick_seed_index(&mut self) -> usize {
        debug_assert!(!self.seedpool.is_empty());
        let total_weight = self
            .seedpool
            .iter()
            .map(|record| u64::from(record.score.max(1)))
            .sum::<u64>();
        let mut ticket = self.mutator.rng_mut().gen_range(0, total_weight);
        for (idx, record) in self.seedpool.iter_mut().enumerate() {
            let weight = u64::from(record.score.max(1));
            if ticket < weight {
                record.last_used_at = self.exec_count;
                return idx;
            }
            ticket -= weight;
        }
        let last_idx = self.seedpool.len() - 1;
        self.seedpool[last_idx].last_used_at = self.exec_count;
        last_idx
    }

    fn prune_seedpool(&mut self) {
        while self.seedpool.len() > MAX_ONESHOT_CORPUS {
            let remove_idx = self
                .seedpool
                .iter()
                .enumerate()
                .min_by_key(|(_, record)| (record.score, record.last_used_at))
                .map(|(idx, _)| idx)
                .expect("non-empty seedpool");
            self.seedpool.swap_remove(remove_idx);
        }
    }

    /// Sample one seed from the local corpus.
    pub fn sample_seed<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<SeedInput> {
        if self.seedpool.is_empty() {
            return None;
        }
        let index = rng.gen_range(0, self.seedpool.len());
        Some(self.seedpool[index].input.clone())
    }

    /// Remember a concrete seed in the local corpus if not already present.
    pub fn remember_seed(&mut self, seed: SeedInput) {
        self.remember_seed_with_score(seed, 1);
    }

    pub fn remember_seed_with_score(&mut self, seed: SeedInput, score: u32) {
        if let Some(existing) = self.seedpool.iter_mut().find(|record| record.input == seed) {
            existing.score = existing.score.saturating_add(score.max(1));
            existing.last_used_at = self.exec_count;
            return;
        }
        self.seedpool.push(SeedRecord {
            input: seed,
            score: score.max(1),
            last_used_at: self.exec_count,
        });
        self.prune_seedpool();
    }

    /// Access the local corpus for persistence.
    pub fn seed_pool_snapshot(&self) -> Vec<SeedInput> {
        self.seedpool
            .iter()
            .map(|record| record.input.clone())
            .collect()
    }

    pub fn seed_record_snapshot(&self) -> Result<Vec<PersistedOneshotSeedRecord>> {
        self.seedpool
            .iter()
            .map(|record| {
                Ok(PersistedOneshotSeedRecord {
                    input: PersistedSeedInput::try_from_seed(&record.input)?,
                    score: record.score,
                    last_used_at: record.last_used_at,
                })
            })
            .collect()
    }

    pub fn replay_log_snapshot(&self) -> Result<Vec<PersistedSeedInput>> {
        self.replay_log
            .iter()
            .map(PersistedSeedInput::try_from_seed)
            .collect()
    }

    /// Import concrete seeds into the local corpus.
    pub fn import_seed_pool<S: Into<SeedInput>>(&mut self, seeds: Vec<S>) {
        for seed in seeds {
            self.remember_seed(seed.into());
        }
    }

    /// Clone the current coverage snapshot for persistence.
    pub fn coverage_snapshot(&self) -> ExecCoverageMap {
        clone_exec_coverage_map(&self.coverage)
    }

    /// Restore a checkpointed corpus and coverage snapshot.
    pub fn restore_checkpoint<S: Into<SeedInput>>(
        &mut self,
        seeds: Vec<S>,
        coverage: ExecCoverageMap,
    ) {
        self.import_seed_pool(seeds);
        self.exec_count = self.seedpool.len() as u64;
        self.coverage = coverage;
        self.coverage_at_last_report = count_coverage_entries(&self.coverage);
        if !self.seedpool.is_empty() || self.coverage_at_last_report > 0 {
            self.last_new_coverage_time = Some(Instant::now());
        }
    }

    pub fn restore_checkpoint_records(
        &mut self,
        seeds: Vec<PersistedOneshotSeedRecord>,
        coverage: ExecCoverageMap,
    ) -> Result<()> {
        self.seedpool.clear();
        for record in seeds {
            self.seedpool.push(SeedRecord {
                input: record.input.into_seed()?,
                score: record.score.max(1),
                last_used_at: record.last_used_at,
            });
        }
        self.prune_seedpool();
        self.exec_count = self
            .seedpool
            .iter()
            .map(|record| record.last_used_at)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.coverage = coverage;
        self.coverage_at_last_report = count_coverage_entries(&self.coverage);
        if !self.seedpool.is_empty() || self.coverage_at_last_report > 0 {
            self.last_new_coverage_time = Some(Instant::now());
        }
        Ok(())
    }

    pub fn replay_checkpoint_log(&mut self, replay_log: Vec<PersistedSeedInput>) -> Result<()> {
        self.replay_log.clear();
        for record in replay_log {
            let seed = record.into_seed()?;
            self.replay_seed(&seed)?;
            self.replay_log.push(seed);
        }
        Ok(())
    }

    fn build_payload(&self, ty_args: Vec<VmTypeTag>, args: Vec<MoveValue>) -> TransactionPayload {
        TransactionPayload::Script(Script::new(
            self.script_code.clone(),
            ty_args,
            args.iter()
                .map(|arg| {
                    TransactionArgument::Serialized(
                        MoveValue::simple_serialize(arg).expect("arguments must serialize"),
                    )
                })
                .collect(),
        ))
    }

    fn replay_seed(&mut self, seed: &SeedInput) -> Result<()> {
        let payload = self.build_payload(seed.ty_args.clone(), seed.args.clone());
        let _ = self
            .executor
            .run_payload_with_sender(seed.sender, payload)?;
        Ok(())
    }

    /// Export the current shared object-discovery state.
    pub fn object_state_snapshot(&self) -> PersistedObjectState {
        self.mutator.snapshot_object_state()
    }

    /// Restore the shared object-discovery state.
    pub fn restore_object_state(&mut self, state: &PersistedObjectState) -> Result<()> {
        self.mutator.restore_object_state(state)
    }

    /// Get the total number of covered bytecode positions across all modules
    pub fn coverage_count(&self) -> usize {
        count_coverage_entries(&self.coverage)
    }

    /// Globalized coverage keys for cross-fuzzer deduplication.
    pub fn coverage_keys(&self) -> BTreeSet<String> {
        collect_coverage_keys(&self.coverage)
    }

    /// Get the total number of executions
    pub fn exec_count(&self) -> u64 {
        self.exec_count
    }

    /// Get when coverage was last found
    pub fn last_new_coverage_time(&self) -> Option<Instant> {
        self.last_new_coverage_time
    }

    /// Get the coverage delta since last report and reset the snapshot
    pub fn coverage_delta_since_report(&mut self) -> usize {
        coverage_delta(&self.coverage, &mut self.coverage_at_last_report)
    }

    pub fn average_seed_score(&self) -> f64 {
        if self.seedpool.is_empty() {
            return 0.0;
        }
        let total: u64 = self
            .seedpool
            .iter()
            .map(|record| u64::from(record.score))
            .sum();
        total as f64 / self.seedpool.len() as f64
    }

    pub fn best_seed_score(&self) -> u32 {
        self.seedpool
            .iter()
            .map(|record| record.score)
            .max()
            .unwrap_or(0)
    }

    /// Short description: `module::function`
    pub fn script_short_desc(&self) -> String {
        format!(
            "{}::{}",
            self.script_sig.ident.module_name(),
            self.script_sig.ident.function_name()
        )
    }

    /// Execute one entry-point.
    /// Returns (status, corpus_size, found_new_coverage, resource_writes, profile, seed).
    /// Resource writes are only returned for successful transactions.
    /// The profile and executed seed are always returned.
    pub fn run_one(
        &mut self,
    ) -> Result<(
        ExecStatus,
        usize,
        bool,
        Vec<ResourceWrite>,
        ExecResourceProfile,
        SeedInput,
    )> {
        let seed_choice = self
            .mutator
            .should_mutate(self.seedpool.len())
            .map(|_| self.pick_seed_index());
        let sender = match seed_choice {
            None => self.mutator.random_signer(),
            Some(_) => {
                let index = self.pick_seed_index();
                if self.mutator.random_percent() < 70 {
                    self.seedpool[index].input.sender
                } else {
                    self.mutator.random_signer()
                }
            },
        };

        // the VM automatically injects the signer from the transaction sender,
        // so we only generate/mutate non-signer parameters as script arguments
        let non_signer_params: Vec<_> = self
            .script_sig
            .parameters
            .iter()
            .filter(|ty| !matches!(ty, BasicInput::Signer))
            .collect();

        // generate or mutate type arguments and value arguments
        let (ty_args, args): (Vec<VmTypeTag>, Vec<MoveValue>) = match seed_choice {
            None => {
                // generate new type arguments and value arguments
                let ty_args = self.mutator.random_type_args(&self.script_sig.generics);
                let args = non_signer_params
                    .iter()
                    .map(|ty| self.mutator.random_value(ty))
                    .collect();
                (ty_args, args)
            },
            Some(index) => {
                // mutate existing seed
                let seed = &self.seedpool[index].input;
                let seed_ty_args = &seed.ty_args;
                let seed_args = &seed.args;
                assert_eq!(non_signer_params.len(), seed_args.len());

                let ty_args = if !self.script_sig.generics.is_empty()
                    && self.mutator.should_mutate_type_args()
                {
                    self.mutator
                        .mutate_type_args(&self.script_sig.generics, seed_ty_args)
                } else {
                    seed_ty_args.clone()
                };

                let args = seed_args
                    .iter()
                    .zip(non_signer_params.iter())
                    .map(|(val, ty)| self.mutator.mutate_value(ty, val))
                    .collect();
                (ty_args, args)
            },
        };

        let executed_seed = SeedInput {
            sender,
            ty_args: ty_args.clone(),
            args: args.clone(),
        };
        let payload = self.build_payload(ty_args.clone(), args.clone());

        // prologue: reset the VM's trace buffer (truncates and reopens the file)
        clear_tracing_buffer();

        // execute with tracking to capture resource reads
        let (vm_status, txn_status, resource_writes, resource_reads) = self
            .executor
            .run_payload_with_sender_tracking(executed_seed.sender, payload)?;
        self.replay_log.push(executed_seed.clone());

        // epilogue: flush and read coverage
        flush_tracing_buffer();

        // update coverage and seed pool
        self.exec_count += 1;
        let exec_status: ExecStatus = (vm_status, txn_status).into();
        let coverage_map = CoverageMap::from_trace_file(&self.trace_path)?;
        let found_new = self.update_coverage(coverage_map);
        if found_new {
            self.last_new_coverage_time = Some(Instant::now());
        }
        let profile = ExecResourceProfile::from_execution(
            self.script_index,
            &resource_writes,
            &resource_reads,
            matches!(exec_status, ExecStatus::Success),
        );

        // only share resource writes from successful transactions
        let shared_writes = if matches!(exec_status, ExecStatus::Success) {
            resource_writes
        } else {
            vec![]
        };
        self.mutator.update_object_dict(&shared_writes);

        // return status
        Ok((
            exec_status,
            self.seedpool.len(),
            found_new,
            shared_writes,
            profile,
            executed_seed,
        ))
    }

    /// Absorb shared object discoveries from other fuzzers
    pub fn absorb_shared_object_writes(&mut self, writes: &[ResourceWrite]) {
        self.mutator.update_object_dict(writes);
    }

    /// Update coverage map, return true if new coverage is found
    fn update_coverage(&mut self, new_map: CoverageMap) -> bool {
        merge_coverage(&mut self.coverage, new_map)
    }
}

#[cfg(test)]
mod tests {
    use super::{ExecStatus, OneshotFuzzer, SeedInput, MAX_ONESHOT_CORPUS};
    use crate::{
        executor::tracing::TracingExecutor,
        mutate::mutator::TypePool,
        prep::{
            canvas::{BasicInput, ScriptSignature},
            ident::FunctionIdent,
        },
    };
    use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
    use move_core_types::{
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::ModuleId,
        value::MoveValue,
        vm_status::{AbortLocation, StatusCode, VMStatus},
    };
    use std::path::PathBuf;

    fn abort_location() -> AbortLocation {
        AbortLocation::Module(ModuleId::new(
            AccountAddress::ONE,
            Identifier::new("test").unwrap(),
        ))
    }

    fn make_test_fuzzer() -> OneshotFuzzer {
        OneshotFuzzer::new(
            TracingExecutor::new(),
            7,
            0,
            ScriptSignature {
                name: "script_0".to_string(),
                ident: FunctionIdent::from_function_tuple(
                    AccountAddress::ONE,
                    Identifier::new("m").unwrap(),
                    Identifier::new("f").unwrap(),
                ),
                generics: vec![],
                parameters: vec![BasicInput::U64],
            },
            vec![],
            TypePool::new(),
            PathBuf::from("/tmp/move-fuzz-oneshot-tests.trace"),
            vec![],
        )
    }

    fn make_seed(value: u64) -> SeedInput {
        SeedInput {
            sender: AccountAddress::ONE,
            ty_args: vec![],
            args: vec![MoveValue::U64(value)],
        }
    }

    #[test]
    fn test_exec_status_from_success_and_category() {
        let status: ExecStatus = (
            VMStatus::Executed,
            TransactionStatus::Keep(ExecutionStatus::Success),
        )
            .into();
        assert!(matches!(status, ExecStatus::Success));
        assert_eq!(status.category(), "success");
        assert_eq!(status.to_string(), "success");
    }

    #[test]
    fn test_exec_status_from_move_abort_preserves_location_and_code() {
        let location = abort_location();
        let status: ExecStatus = (
            VMStatus::MoveAbort {
                location: location.clone(),
                code: 42,
                message: None,
            },
            TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                location: location.clone(),
                code: 42,
                info: None,
            }),
        )
            .into();

        assert!(matches!(
            status,
            ExecStatus::AbortDeclared {
                abort_code: 42,
                location: ref actual,
            } if *actual == location
        ));
        assert_eq!(status.category(), "abort");
        assert!(status.to_string().contains("abort(42"));
    }

    #[test]
    fn test_exec_status_from_out_of_gas() {
        let status: ExecStatus = (
            VMStatus::Error {
                status_code: StatusCode::OUT_OF_GAS,
                sub_status: None,
                message: None,
            },
            TransactionStatus::Keep(ExecutionStatus::OutOfGas),
        )
            .into();

        assert!(matches!(status, ExecStatus::OutOfGas));
        assert_eq!(status.category(), "out-of-gas");
        assert_eq!(status.to_string(), "out-of-gas");
    }

    #[test]
    fn test_exec_status_display_for_intrinsic_abort_with_substatus() {
        let status = ExecStatus::AbortIntrinsic {
            status_code: StatusCode::ARITHMETIC_ERROR,
            sub_status: Some(7),
            location: abort_location(),
            function: 2,
            instruction: 9,
        };

        let rendered = status.to_string();
        assert!(rendered.contains("ARITHMETIC_ERROR::7"));
        assert!(rendered.contains("::2::9"));
    }

    #[test]
    fn test_seedpool_scoring_updates_existing_and_prunes_lowest_score() {
        let mut fuzzer = make_test_fuzzer();
        let low_score_seed = make_seed(0);
        fuzzer.remember_seed_with_score(low_score_seed.clone(), 1);

        for value in 1..=MAX_ONESHOT_CORPUS as u64 {
            fuzzer.remember_seed_with_score(make_seed(value), 10);
        }

        assert_eq!(fuzzer.corpus_size(), MAX_ONESHOT_CORPUS);
        assert!(
            !fuzzer
                .seedpool
                .iter()
                .any(|record| record.input == low_score_seed),
            "lowest-score seed should be pruned when the corpus exceeds the cap"
        );

        let boosted_seed = make_seed(1);
        fuzzer.remember_seed_with_score(boosted_seed.clone(), 7);
        let retained = fuzzer
            .seedpool
            .iter()
            .find(|record| record.input == boosted_seed)
            .expect("existing seed should still be present");
        assert_eq!(retained.score, 17);
        assert_eq!(fuzzer.best_seed_score(), 17);
        assert!(fuzzer.average_seed_score() > 0.0);
        assert_eq!(fuzzer.seed_pool_snapshot().len(), fuzzer.corpus_size());
    }
}
