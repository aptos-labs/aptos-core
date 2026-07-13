// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Cross-session delayed-field squash regression matrix. Each scenario runs two VM sessions over a
//! test-only `0x1::session` package (session_1 transforms a delayed-field container, session_2
//! touches the aggregator), then squashes their change sets, mirroring the prologue/user/epilogue
//! interaction. Under the strict squash, benign delta+delta and disjoint flows succeed with correct
//! materialized values, while cross-session structural cases fail closed (see `session.move`).

use crate::{assert_success, tests::common, MoveHarness};
use aptos_aggregator::{
    delayed_change::DelayedChange, resolver::TDelayedFieldView, types::DelayedFieldValue,
};
use aptos_block_executor::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    executor::BlockExecutor,
    task::{BeforeMaterializationOutput, ExecutionStatus, ExecutorTask, TransactionOutput},
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
    types::InputOutputKey,
};
use aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
        value::ValueWithLayout,
    },
    contract_event::ContractEvent,
    error::PanicError,
    fee_statement::FeeStatement,
    state_store::{state_key::StateKey, state_value::StateValueMetadata, TStateView},
    transaction::{AuxiliaryInfo, BlockExecutableTransaction},
    write_set::WriteOp,
};
use aptos_vm::{
    move_vm_ext::{
        session::view_with_change_set::new_for_executor_view_with_change_set_for_test,
        AptosMoveResolver, SessionId,
    },
    AptosVM,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    change_set::VMChangeSet,
    module_and_script_storage::code_storage::AptosCodeStorage,
    module_write_set::ModuleWrite,
    resolver::{
        BlockSynchronizationKillSwitch, ExecutorView, ResourceGroupSize, ResourceGroupView,
    },
    storage::change_set_configs::ChangeSetConfigs,
};
use claims::assert_ok;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag},
    value::{MoveTypeLayout, MoveValue},
    vm_status::VMStatus,
};
use move_vm_runtime::{
    execution_tracing::Trace,
    module_traversal::{TraversalContext, TraversalStorage},
    ModuleStorage,
};
use move_vm_types::{delayed_values::delayed_field_id::DelayedFieldID, gas::UnmeteredGasMeter};
use once_cell::sync::OnceCell;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    str::FromStr,
    sync::{Mutex, OnceLock},
};

/// What we expect a scenario to do once squashed.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Expected {
    /// Squash must fail-closed AND the error must contain every one of these substrings. This pins
    /// the *exact* failure (status code + the discriminating message), not merely that it failed.
    Fail(&'static [&'static str]),
    /// Squash succeeds and the aggregators materialize to exactly these (sorted) values.
    Values(Vec<u128>),
}

/// Observed outcome recorded by `execute_transaction` (runs on Block-STM worker threads),
/// read back and asserted by `testcase`. `Failed` carries the full `{:?}` of the squash error
/// (which includes the major status code and message) so the exact error can be asserted.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Ok(Vec<u128>),
    Failed(String),
}

fn outcome_cell() -> &'static Mutex<Option<Outcome>> {
    static CELL: OnceLock<Mutex<Option<Outcome>>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(None))
}
use triomphe::Arc as TriompheArc;

struct TestTask {
    vm: AptosVM,
}

impl TestTask {
    fn run(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl ModuleStorage,
        workload: &SessionWorkload,
    ) -> VMChangeSet {
        let mut session = self.vm.new_session(resolver, SessionId::Void, None);
        let traversal_storage = TraversalStorage::new();
        let mut traversal_ctx = TraversalContext::new(&traversal_storage);
        let args = vec![MoveValue::Signer(AccountAddress::ONE)
            .simple_serialize()
            .unwrap()];
        let result = session.execute_function_bypass_visibility(
            &workload.module,
            workload.function.as_ident_str(),
            vec![],
            args,
            &mut UnmeteredGasMeter,
            &mut traversal_ctx,
            module_storage,
        );
        assert_ok!(result);
        let result = session.finish(
            &ChangeSetConfigs::unlimited_at_gas_feature_version(LATEST_GAS_FEATURE_VERSION),
            module_storage,
        );
        assert_ok!(result)
    }
}

#[derive(Clone)]
struct SessionWorkload {
    module: ModuleId,
    function: Identifier,
}

#[derive(Clone)]
struct TestTransaction {
    session_1: SessionWorkload,
    session_2: SessionWorkload,
}

impl BlockExecutableTransaction for TestTransaction {
    type Event = ContractEvent;
    type Key = StateKey;
    type Tag = StructTag;
    type Value = WriteOp;

    fn user_txn_bytes_len(&self) -> usize {
        0
    }
}

#[derive(Debug)]
struct TestOutput;

impl TransactionOutput for TestOutput {
    type BeforeMaterializationGuard<'a> = &'a Self;
    type CommittedOutput = aptos_types::transaction::TransactionOutput;
    type Txn = TestTransaction;

    fn skip_output() -> Self {
        Self
    }

    fn before_materialization(&self) -> Result<Self::BeforeMaterializationGuard<'_>, PanicError> {
        Ok(self)
    }

    fn incorporate_materialized_txn_output(
        &mut self,
        _patched_resource_write_set: Vec<(StateKey, WriteOp)>,
        _patched_events: Vec<ContractEvent>,
    ) -> Result<(Self::CommittedOutput, Trace), PanicError> {
        Ok((
            aptos_types::transaction::TransactionOutput::new_empty_success(),
            Trace::empty(),
        ))
    }
}

impl BeforeMaterializationOutput<TestTransaction> for &TestOutput {
    fn resource_write_set(&self) -> HashMap<StateKey, ValueWithLayout<WriteOp>> {
        HashMap::new()
    }

    fn module_write_set(&self) -> &BTreeMap<StateKey, ModuleWrite<WriteOp>> {
        static EMPTY: OnceCell<BTreeMap<StateKey, ModuleWrite<WriteOp>>> = OnceCell::new();
        EMPTY.get_or_init(BTreeMap::new)
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        BTreeMap::new()
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(StateKey, StateValueMetadata, TriompheArc<MoveTypeLayout>)> {
        vec![]
    }

    fn group_reads_needing_delayed_field_exchange(&self) -> Vec<(StateKey, StateValueMetadata)> {
        vec![]
    }

    fn get_events(&self) -> Vec<(ContractEvent, Option<MoveTypeLayout>)> {
        vec![]
    }

    fn resource_group_write_set(
        &self,
    ) -> HashMap<
        StateKey,
        (
            ValueWithLayout<WriteOp>,
            ResourceGroupSize,
            BTreeMap<StructTag, ValueWithLayout<WriteOp>>,
        ),
    > {
        HashMap::new()
    }

    fn for_each_resource_key(
        &self,
        _callback: &mut dyn FnMut(&StateKey) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        Ok(())
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        _callback: &mut dyn FnMut(&StateKey, HashSet<&StructTag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        Ok(())
    }

    fn fee_statement(&self) -> FeeStatement {
        FeeStatement::zero()
    }

    fn has_new_epoch_event(&self) -> bool {
        false
    }

    fn output_approx_size(&self) -> u64 {
        0
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<StateKey, StructTag>> {
        HashSet::new()
    }

    fn storage_keys_read(&self) -> impl Iterator<Item = &StateKey> {
        std::iter::empty()
    }

    fn storage_keys_written(&self) -> impl Iterator<Item = &StateKey> {
        std::iter::empty()
    }
}

impl ExecutorTask for TestTask {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Error = VMStatus;
    type Output = TestOutput;
    type Txn = TestTransaction;

    fn init(
        environment: &AptosEnvironment,
        _state_view: &impl TStateView<Key = StateKey>,
        async_runtime_checks_enabled: bool,
    ) -> Self {
        assert!(environment.vm_config().delayed_field_optimization_enabled);
        let vm = AptosVM::new_for_block_executor(environment, async_runtime_checks_enabled);
        Self { vm }
    }

    fn execute_transaction(
        &self,
        view: &(impl ExecutorView
              + ResourceGroupView
              + AptosCodeStorage
              + BlockSynchronizationKillSwitch),
        txn: &Self::Txn,
        _auxiliary_info: &Self::AuxiliaryInfo,
        _txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        let resolver = self.vm.as_move_resolver_with_group_view(view);
        let mut change_set_1 = self.run(&resolver, view, &txn.session_1);
        println!("  [session_1] change set: {:?}", change_set_1);

        let new_view =
            new_for_executor_view_with_change_set_for_test(view, view, change_set_1.clone());
        let new_resolver = self.vm.as_move_resolver_with_group_view(&new_view);
        let change_set_2 = self.run(&new_resolver, view, &txn.session_2);
        println!("  [session_2] change set: {:?}", change_set_2);

        // Exercise the strict resource-group squash (the fail-closed window
        // [RELEASE_V1_46, RELEASE_V1_48)) by passing `true` explicitly, independent of the harness's
        // gas feature version.
        let outcome = match change_set_1.squash_additional_change_set(change_set_2, true) {
            Ok(()) => {
                // Resolve every aggregator in the SQUASHED change set to its final, materialized
                // value (base value + recorded change), via a layered view over the base. This
                // proves the squash result is semantically correct, not merely non-aborting.
                let final_view = new_for_executor_view_with_change_set_for_test(
                    view,
                    view,
                    change_set_1.clone(),
                );
                let mut vals: Vec<u128> = vec![];
                for id in change_set_1.delayed_field_change_set().keys() {
                    match final_view.get_delayed_field_value(id) {
                        Ok(DelayedFieldValue::Aggregator(v)) => vals.push(v),
                        other => panic!("unexpected delayed field value for {id:?}: {other:?}"),
                    }
                }
                vals.sort();
                println!("  SQUASH OK; resolved aggregator values = {vals:?}");
                Outcome::Ok(vals)
            },
            Err(e) => {
                // NOTE: this is the RAW squash error (a `code_invariant_error`, status
                // DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR) because the harness calls
                // `squash_additional_change_set` directly. In the real VM, the epilogue's
                // `RespawnedSession::finish_with_squashed_change_set` catches any squash error and
                // remaps it to `UNKNOWN_INVARIANT_VIOLATION_ERROR`, so the on-chain transaction is a
                // terminal `Keep(MiscellaneousError)` (gas charged) -- not a Block-STM fatal.
                let detail = format!("{:?}", e);
                println!("  SQUASH FAILED: {detail}");
                Outcome::Failed(detail)
            },
        };
        *outcome_cell().lock().unwrap() = Some(outcome);

        ExecutionStatus::Success(TestOutput)
    }
}

fn testcase(label: &str, txn: TestTransaction, expected: Expected) {
    println!("######## SCENARIO: {label} ########");
    *outcome_cell().lock().unwrap() = None;
    let mut h = MoveHarness::new();
    let path = common::test_dir_path("session.data/pack");
    let account = h.new_account_at(AccountAddress::ONE);
    assert_success!(h.publish_package(&account, &path));

    let state_view = h.executor.get_state_view();
    let txn_provider =
        DefaultTxnProvider::<TestTransaction, AuxiliaryInfo>::new_without_info(vec![txn]);
    let mut guard = AptosModuleCacheManagerGuard::none_with_delayed_fields_for_testing(state_view);
    let executor = BlockExecutor::<
        TestTransaction,
        TestTask,
        _,
        NoOpTransactionCommitHook<TestOutput>,
        DefaultTxnProvider<TestTransaction, AuxiliaryInfo>,
        AuxiliaryInfo,
    >::new(BlockExecutorConfig::new_maybe_block_limit(2, None), None);
    let _ = executor.execute_transactions_parallel_for_testing(
        &txn_provider,
        state_view,
        &TransactionSliceMetadata::unknown(),
        &mut guard,
    );

    let observed = outcome_cell()
        .lock()
        .unwrap()
        .clone()
        .expect("execute_transaction must have recorded an outcome");
    match (&expected, &observed) {
        (Expected::Fail(needles), Outcome::Failed(detail)) => {
            for needle in *needles {
                assert!(
                    detail.contains(needle),
                    "[{label}] error did not contain expected substring.\n  expected substring: {needle}\n  actual error: {detail}"
                );
            }
            println!("  => MATCH: fail-closed with the exact expected error");
        },
        (Expected::Values(exp), Outcome::Ok(got)) => {
            assert_eq!(
                got, exp,
                "[{label}] aggregator values wrong: got {got:?}, expected {exp:?}"
            );
            println!("  => MATCH: squash OK and values correct {got:?}");
        },
        (exp, got) => panic!("[{label}] outcome mismatch: expected {exp:?}, observed {got:?}"),
    }
}

fn test_txn(session_1_function: &str, session_2_function: &str) -> TestTransaction {
    let module = ModuleId::from_str("0x1::session").unwrap();
    TestTransaction {
        session_1: SessionWorkload {
            module: module.clone(),
            function: Identifier::new(session_1_function).unwrap(),
        },
        session_2: SessionWorkload {
            module,
            function: Identifier::new(session_2_function).unwrap(),
        },
    }
}

#[test]
fn test_session_with_delayed_fields() {
    // Block-STM + Move VM recursion needs a large stack; run off the default test thread.
    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(run_all_scenarios)
        .unwrap()
        .join()
        .unwrap();
}

fn run_all_scenarios() {
    // (label, session_1, session_2, expected), asserted under the STRICT resource-group squash
    // (fail-closed window [RELEASE_V1_46, RELEASE_V1_48)). Cross-session structural cases fail
    // closed with the exact error; benign delta+delta flows and disjoint controls succeed with
    // correct values.
    let scenarios = [
        // NORMAL concurrent-FA flow: same aggregator delta'd in BOTH sessions, NO structural
        // change (user-session transfer delta + epilogue gas delta). Must stay allowed + correct.
        (
            "N1 normal GROUP delta+delta (FA flow)",
            "test_5_increment_aggregator",
            "test_5_increment_aggregator",
            Expected::Values(vec![2]),
        ),
        (
            "N2 normal standalone delta+delta",
            "test_1_increment_aggregator",
            "test_1_increment_aggregator",
            Expected::Values(vec![2]),
        ),
        (
            "1 standalone resource resize + agg",
            "test_1_change_resource_size",
            "test_1_increment_aggregator",
            Expected::Fail(&[
                "DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR",
                "Refusing to squash a resource write with a later in-place delayed-field change on the same key (fail-closed for safety)",
                "Resource(0x1::session::Test1)",
                "into InPlaceDelayedFieldChange",
            ]),
        ),
        (
            "2 move agg between resources",
            "test_2_move_aggregator",
            "test_2_increment_aggregator",
            Expected::Fail(&[
                "DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR",
                "Trying to squash incompatible writes",
                "Resource(0x1::session::Test2)",
                "into InPlaceDelayedFieldChange",
            ]),
        ),
        (
            "3 two aggregators",
            "test_3_increment_fst_aggregator",
            "test_3_increment_snd_aggregator",
            Expected::Values(vec![100, 200]),
        ),
        (
            "4 write/overwrite resources",
            "test_4_write_fst_resource",
            "test_4_write_snd_resource",
            Expected::Values(vec![0]),
        ),
        (
            "5 GROUP grow (sibling) + agg",
            "test_5_change_resource_group_size",
            "test_5_increment_aggregator",
            Expected::Fail(&[
                "DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR",
                "Refusing to squash a resource-group write with a later in-place delayed-field change on the same group (fail-closed for safety)",
                "ResourceGroup(0x1::session::Group1)",
                "materialized_size: 84",
            ]),
        ),
        (
            "6 move agg between groups",
            "test_6_move_aggregator_between_groups",
            "test_6_increment_aggregator",
            Expected::Fail(&[
                "DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR",
                "Refusing to squash a resource-group write with a later in-place delayed-field change on the same group (fail-closed for safety)",
                "ResourceGroup(0x1::session::Group1)",
                "materialized_size: 84",
            ]),
        ),
        (
            "7 move agg group->resource",
            "test_7_move_aggregator_from_group_to_resource",
            "test_7_increment_aggregator",
            Expected::Fail(&[
                "DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR",
                "Refusing to squash a resource-group write with a later in-place delayed-field change on the same group (fail-closed for safety)",
                "ResourceGroup(0x1::session::Group1)",
                "materialized_size: 84",
            ]),
        ),
        // ---- Additional delayed-field arm coverage (understand each squash arm's behavior) ----
        // GROUP, reverse of #5: session_1 deltas the group aggregator (RG-InPlace), session_2 does
        // a full group write (WriteResourceGroup). This is the OVERWRITE arm
        // `RG-InPlace ⊕ WriteResourceGroup` — must succeed AND the later session must read the
        // earlier delta (resolved value proves it).
        (
            "8 group RG-InPlace then WriteResourceGroup (overwrite)",
            "test_5_increment_aggregator",
            "test_5_change_resource_group_size",
            Expected::Values(vec![1]),
        ),
        // GROUP: two full group writes `WriteResourceGroup ⊕ WriteResourceGroup` (merge), neither
        // touches the aggregator.
        (
            "9 group WriteResourceGroup then WriteResourceGroup (merge)",
            "test_5_change_resource_group_size",
            "test_8_write_group_member_data",
            Expected::Values(vec![]),
        ),
        // STANDALONE, reverse of #1: session_1 deltas the resource aggregator (InPlace), session_2
        // rewrites the resource (WriteWithDelayedFields). OVERWRITE arm
        // `InPlaceDelayedFieldChange ⊕ WriteWithDelayedFields` — must succeed and carry the delta.
        (
            "10 standalone InPlace then WriteWithDelayedFields (overwrite)",
            "test_1_increment_aggregator",
            "test_1_change_resource_size",
            Expected::Values(vec![1]),
        ),
        // STANDALONE: two full resource writes `WriteWithDelayedFields ⊕ WriteWithDelayedFields`
        // (merge), neither touches the aggregator.
        (
            "11 standalone WriteWithDelayedFields x2 (merge)",
            "test_1_change_resource_size",
            "test_4_write_fst_resource",
            Expected::Values(vec![]),
        ),
    ];
    for (label, s1, s2, expected) in scenarios {
        testcase(label, test_txn(s1, s2), expected);
    }
}
