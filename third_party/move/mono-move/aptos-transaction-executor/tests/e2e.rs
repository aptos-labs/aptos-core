// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end differential test: a p2p transfer executed by the legacy
//! AptosVM (via `FakeExecutor`) and by the MonoMove-backed transaction
//! executor against the same starting state.
//!
//! Gas amounts intentionally differ between the two VMs, so writes that embed
//! the fee (the fee payer's fungible store, the APT supply) and fee events are
//! allowed to differ in *content*; everything else must match byte-for-byte.
//
// TODO(testing): revisit once the executor is more mature/wired up. See if we
// want to switch to other payloads that are less gas-dependent, or get this
// covered by other tests, such as the e2e move tests. Note that the sender's
// store is masked, so a wrong debit there is not caught.

use aptos_gas_schedule::{InitialGasSchedule, TransactionGasParameters};
use aptos_language_e2e_tests::{
    account::{Account, AccountData},
    executor::FakeExecutor,
};
use aptos_types::{
    secret_sharing::EvalProof,
    state_store::StateView,
    transaction::{
        encrypted_payload::{
            DecryptedPlaintext, DecryptionFailureReason, EncryptedInner, EncryptedPayload,
        },
        AuxiliaryInfo, ExecutionStatus, PersistedAuxiliaryInfo, SignedTransaction, Transaction,
        TransactionAuxiliaryData, TransactionOutput, TransactionStatus,
    },
};
use aptos_vm_environment::environment::AptosEnvironment;
use mono_move_aptos_state_view_providers::{StateViewModuleProvider, StateViewResourceProvider};
use mono_move_aptos_transaction_executor::{
    production_natives, AptosTransactionExecutor, TxnOutcome,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use move_core_types::{transaction_argument::TransactionArgument, vm_status::StatusCode};
use std::collections::BTreeMap;

/// Event types whose payload embeds gas amounts.
const GAS_DEPENDENT_EVENTS: &[&str] = &[
    "0x1::transaction_fee::FeeStatement",
    "0x1::fungible_asset::Withdraw",
    "0x1::coin::CoinWithdraw",
];

/// Runs one user transaction through the MonoMove executor against `state`,
/// materialized into the legacy output formats.
fn execute_v2<S: StateView>(state: &S, txn: &SignedTransaction) -> TransactionOutput {
    execute_v2_with(state, |executor| {
        executor.execute_transaction(
            &Transaction::UserTransaction(txn.clone()),
            &AuxiliaryInfo::new(PersistedAuxiliaryInfo::None, None),
        )
    })
}

/// Runs one block-metadata transaction through the MonoMove executor against
/// `state`, materialized into the legacy output formats.
fn execute_v2_block_metadata<S: StateView>(
    state: &S,
    block_metadata: &aptos_types::block_metadata::BlockMetadata,
) -> TransactionOutput {
    execute_v2_with(state, |executor| {
        executor.execute_transaction(
            &Transaction::BlockMetadata(block_metadata.clone()),
            &first_txn_aux_info(),
        )
    })
}

/// What V1's block path supplies for a block's first transaction.
fn first_txn_aux_info() -> AuxiliaryInfo {
    AuxiliaryInfo::new(
        PersistedAuxiliaryInfo::V1 {
            transaction_index: 0,
        },
        None,
    )
}

/// Builds the executor against `state` in a fresh global context, runs one
/// transaction through it, and materializes the outcome.
fn execute_v2_with<S: StateView>(
    state: &S,
    run: impl for<'guard> FnOnce(&AptosTransactionExecutor<'guard>) -> TxnOutcome,
) -> TransactionOutput {
    let global_ctx = GlobalContext::with_num_execution_workers(1);
    let guard = global_ctx
        .try_execution_context(0)
        .expect("execution context is available");
    execute_v2_in(&guard, state, run)
}

/// Like `execute_v2_with`, in an existing execution context.
fn execute_v2_in<S: StateView>(
    guard: &ExecutionGuard<'_>,
    state: &S,
    run: impl for<'guard> FnOnce(&AptosTransactionExecutor<'guard>) -> TxnOutcome,
) -> TransactionOutput {
    let natives = production_natives();
    let module_provider = StateViewModuleProvider::new(state);
    let data_provider = StateViewResourceProvider::new(guard, state);
    let env = AptosEnvironment::new(state);
    let usage = state.get_usage().expect("usage is readable");
    let executor = AptosTransactionExecutor::new(
        guard,
        natives,
        &module_provider,
        &data_provider,
        &env,
        usage,
    );
    let (output, _groups) = run(&executor)
        .materialize(
            guard,
            &data_provider,
            env.features(),
            TransactionAuxiliaryData::default(),
        )
        .expect("the transaction output materializes");
    output
}

/// Runs `txns` in order through one global context, applying each output to
/// the state before the next.
fn execute_v2_sequence<S: StateView + Sync>(
    base: &S,
    txns: &[SignedTransaction],
) -> Vec<TransactionOutput> {
    use aptos_transaction_simulation::{DeltaStateStore, SimulationStateStore};

    let state = DeltaStateStore::new_with_base(base);
    let global_ctx = GlobalContext::with_num_execution_workers(1);
    let guard = global_ctx
        .try_execution_context(0)
        .expect("execution context is available");
    let mut outputs = Vec::with_capacity(txns.len());
    for txn in txns {
        let output = execute_v2_in(&guard, &state, |executor| {
            executor.execute_transaction(
                &Transaction::UserTransaction(txn.clone()),
                &AuxiliaryInfo::new(PersistedAuxiliaryInfo::None, None),
            )
        });
        state
            .apply_write_set(output.write_set())
            .expect("write set applies");
        outputs.push(output);
    }
    outputs
}

/// Fresh genesis with a funded sender (sequence number 10) and recipient.
fn setup() -> (FakeExecutor, AccountData, AccountData) {
    let mut fx = FakeExecutor::from_head_genesis();
    let alice = fx.create_raw_account_data(1_000_000_000, 10);
    fx.add_account_data(&alice);
    let bob = fx.create_raw_account_data(100_000_000, 0);
    fx.add_account_data(&bob);
    (fx, alice, bob)
}

#[test]
fn p2p_transfer_matches_v1() {
    let (fx, alice, bob) = setup();

    let txn = alice
        .account()
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            *bob.address(),
            1_000,
        ))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    // V1 (reference), on the starting state.
    let v1_output = fx.execute_transaction(txn.clone());
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "v1 rejected the transfer: {:?}",
        v1_output.status()
    );

    // V2, on the same starting state.
    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(
        v2_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "v2 failed the transfer"
    );

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// Compares the two outputs' write sets and events, masking only what gas
/// divergence explains.
fn compare_outputs(
    v1: &TransactionOutput,
    v2: &TransactionOutput,
    fee_payer: move_core_types::account_address::AccountAddress,
) {
    let v1_writes: BTreeMap<_, _> = v1.write_set().write_op_iter().collect();
    let v2_writes: BTreeMap<_, _> = v2.write_set().write_op_iter().collect();

    let v1_keys: Vec<_> = v1_writes.keys().collect();
    let v2_keys: Vec<_> = v2_writes.keys().collect();
    assert_eq!(
        v1_keys, v2_keys,
        "the two VMs wrote different sets of state keys"
    );

    // The only slots allowed to differ are the two that embed the fee: the
    // fee payer's primary store (its object group) and the APT metadata
    // object at 0xa (supply).
    let gas_dependent = gas_dependent_keys(fee_payer);
    let mut unexplained = vec![];
    let mut num_diffs = 0;
    for (key, v1_op) in &v1_writes {
        let v2_op = &v2_writes[*key];
        if v1_op.bytes() == v2_op.bytes() {
            continue;
        }
        num_diffs += 1;
        if !gas_dependent.contains(key) {
            unexplained.push(format!(
                "{key:?}:\n  v1: {:?}\n  v2: {:?}",
                v1_op.bytes(),
                v2_op.bytes()
            ));
        }
    }
    assert!(
        unexplained.is_empty(),
        "writes differ beyond gas-dependent slots:\n{}",
        unexplained.join("\n")
    );
    // Both VMs must actually have charged a fee (otherwise the mask above is
    // vacuous and something upstream is broken).
    assert!(
        num_diffs >= 1,
        "no write differed; was a fee charged at all?"
    );
    assert!(v1.gas_used() > 0, "v1 charged no gas");
    assert!(v2.gas_used() > 0, "v2 charged no gas");

    // Events: same sequence of types; payloads equal except gas-dependent ones.
    let v1_events = v1.events();
    let v2_events = v2.events();
    let v1_types: Vec<_> = v1_events
        .iter()
        .map(|e| format!("{:?}", e.type_tag()))
        .collect();
    let v2_types: Vec<_> = v2_events
        .iter()
        .map(|e| format!("{:?}", e.type_tag()))
        .collect();
    assert_eq!(v1_types, v2_types, "event sequences differ");
    for (e1, e2) in v1_events.iter().zip(v2_events) {
        let ty = e1.type_tag().to_canonical_string();
        if GAS_DEPENDENT_EVENTS.contains(&ty.as_str()) {
            continue;
        }
        assert_eq!(
            e1.event_data(),
            e2.event_data(),
            "event payload differs for {ty}"
        );
    }
}

/// A transfer exceeding the sender's balance: the payload aborts inside
/// `0x1::fungible_asset`, the payload's effects roll back, and the failure
/// epilogue still charges the fee and bumps the sequence number.
#[test]
fn p2p_transfer_insufficient_balance_aborts_like_v1() {
    let (fx, alice, bob) = setup();

    let txn = alice
        .account()
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            *bob.address(),
            u64::MAX,
        ))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    let v1_output = fx.execute_transaction(txn.clone());
    let TransactionStatus::Keep(ExecutionStatus::MoveAbort {
        code: v1_code,
        location: v1_location,
        ..
    }) = v1_output.status()
    else {
        panic!("v1 did not abort: {:?}", v1_output.status());
    };

    let v2_output = execute_v2(fx.get_state_view(), &txn);
    // TODO(correctness): compare the abort info too, once the executor resolves
    // it from the aborting module's metadata the way the legacy VM does.
    let TransactionStatus::Keep(ExecutionStatus::MoveAbort {
        code: v2_code,
        location: v2_location,
        ..
    }) = v2_output.status()
    else {
        panic!("v2 did not abort: {:?}", v2_output.status());
    };
    assert_eq!(v1_code, v2_code, "abort codes differ");
    assert_eq!(v1_location, v2_location, "abort locations differ");

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// A transfer that drains the fee payer's entire balance: the payload
/// succeeds, but the success epilogue cannot collect the fee, so the payload
/// rolls back and the fee is charged from the restored balance. The
/// transaction is kept as the can't-pay-fee abort, matching the legacy VM's
/// failure cleanup — including the abort location.
#[test]
fn p2p_transfer_draining_fee_payer_aborts_like_v1() {
    let (fx, alice, bob) = setup();

    // Send the entire balance: nothing is left for the fee at epilogue time.
    let txn = alice
        .account()
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            *bob.address(),
            1_000_000_000,
        ))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    let v1_output = fx.execute_transaction(txn.clone());
    let TransactionStatus::Keep(ExecutionStatus::MoveAbort {
        code: v1_code,
        location: v1_location,
        ..
    }) = v1_output.status()
    else {
        panic!("v1 did not abort: {:?}", v1_output.status());
    };

    let v2_output = execute_v2(fx.get_state_view(), &txn);
    let TransactionStatus::Keep(ExecutionStatus::MoveAbort {
        code: v2_code,
        location: v2_location,
        ..
    }) = v2_output.status()
    else {
        panic!("v2 did not abort: {:?}", v2_output.status());
    };
    assert_eq!(v1_code, v2_code, "abort codes differ");
    assert_eq!(v1_location, v2_location, "abort locations differ");

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// A nonexistent entry function: kept (and charged) on both VMs, because the
/// function-values feature makes a missing function runtime-reachable. This
/// exercises the kept non-abort payload failure path: rollback plus failure
/// epilogue.
#[test]
fn nonexistent_entry_function_kept_like_v1() {
    use aptos_types::transaction::{EntryFunction, TransactionPayload};
    use move_core_types::{ident_str, language_storage::ModuleId};

    let (fx, alice, _bob) = setup();

    let txn = alice
        .account()
        .transaction()
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                move_core_types::account_address::AccountAddress::ONE,
                ident_str!("coin").to_owned(),
            ),
            ident_str!("no_such_function").to_owned(),
            vec![],
            vec![],
        )))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    let v1_output = fx.execute_transaction(txn.clone());
    assert!(
        matches!(
            v1_output.status(),
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(_))
        ),
        "v1 did not keep as a miscellaneous error: {:?}",
        v1_output.status()
    );

    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(v1_output.status(), v2_output.status());

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// A multi-agent transaction supplying two senders to a one-signer entry
/// function is rejected, like on the legacy VM.
//
// TODO(correctness): compare exact statuses once argument rejection gets a
// real status instead of an invariant violation.
#[test]
fn extra_signers_rejected_like_v1() {
    let (fx, alice, bob) = setup();

    // `aptos_account::transfer` takes one `&signer`; supply two senders.
    let txn = alice
        .account()
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            *bob.address(),
            1_000,
        ))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .secondary_signers(vec![bob.account().clone()])
        .sign_multi_agent();

    let v1_output = fx.execute_transaction(txn.clone());
    assert!(
        matches!(
            v1_output.status(),
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                StatusCode::NUMBER_OF_SIGNER_ARGUMENTS_MISMATCH
            )))
        ),
        "v1 did not reject the signer-count mismatch: {:?}",
        v1_output.status()
    );

    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(
        v2_output.status(),
        v1_output.status(),
        "v2 rejected the signer-count mismatch differently"
    );

    // Write sets are not compared: the gas divergence leaves v1 with fee
    // writes v2 lacks.
}

/// A script transaction from `sender`, with the script given as assembly.
fn script_txn(
    sender: &AccountData,
    code: &str,
    args: Vec<TransactionArgument>,
    sequence_number: u64,
) -> SignedTransaction {
    let code = aptos_language_e2e_tests::compile::compile_script(code, vec![])
        .code()
        .to_vec();
    script_bytes_txn(sender, code, args, sequence_number)
}

/// A script transaction from `sender`, with the script given as bytecode.
fn script_bytes_txn(
    sender: &AccountData,
    code: Vec<u8>,
    args: Vec<TransactionArgument>,
    sequence_number: u64,
) -> SignedTransaction {
    use aptos_types::transaction::{Script, TransactionPayload};

    sender
        .account()
        .transaction()
        .payload(TransactionPayload::Script(Script::new(code, vec![], args)))
        .sequence_number(sequence_number)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign()
}

/// A script transferring APT through the framework.
const TRANSFER_SCRIPT: &str = r#"
script
use 0x1::aptos_account as account

public fun main(sender: &signer, to: address, amount: u64)
    move_loc sender
    move_loc to
    move_loc amount
    call account::transfer
    ret
"#;

/// Runs the transfer script with `amount` as its amount argument on both VMs
/// and asserts they agree.
fn assert_transfer_script_matches_v1(amount: TransactionArgument) {
    let (fx, alice, bob) = setup();
    let txn = script_txn(
        &alice,
        TRANSFER_SCRIPT,
        vec![TransactionArgument::Address(*bob.address()), amount],
        10,
    );

    let v1_output = fx.execute_transaction(txn.clone());
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "v1 rejected the script: {:?}",
        v1_output.status()
    );
    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(v2_output.status(), v1_output.status());

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// A script payload runs like on v1, with the same effects.
#[test]
fn script_transfer_matches_v1() {
    assert_transfer_script_matches_v1(TransactionArgument::U64(1_000));
}

/// A pre-serialized script argument is accepted like on v1.
#[test]
fn script_serialized_argument_matches_v1() {
    assert_transfer_script_matches_v1(TransactionArgument::Serialized(
        bcs::to_bytes(&1_000u64).unwrap(),
    ));
}

/// A script's abort is located at the script, like on v1.
#[test]
fn script_abort_matches_v1() {
    use move_core_types::vm_status::AbortLocation;

    let (fx, alice, _bob) = setup();
    let txn = script_txn(
        &alice,
        r#"
script

public fun main()
    ld_u64 42
    abort
"#,
        vec![],
        10,
    );

    let v1_output = fx.execute_transaction(txn.clone());
    assert!(
        matches!(
            v1_output.status(),
            TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                location: AbortLocation::Script,
                code: 42,
                ..
            })
        ),
        "v1 did not abort at the script: {:?}",
        v1_output.status()
    );
    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(v2_output.status(), v1_output.status());

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// A script's runtime error is located at the script, like on v1.
#[test]
fn script_runtime_error_matches_v1() {
    use move_core_types::vm_status::AbortLocation;

    let (fx, alice, _bob) = setup();
    let txn = script_txn(
        &alice,
        r#"
script

public fun main(divisor: u64)
    ld_u64 1
    move_loc divisor
    div
    pop
    ret
"#,
        vec![TransactionArgument::U64(0)],
        10,
    );

    let v1_output = fx.execute_transaction(txn.clone());
    assert!(
        matches!(
            v1_output.status(),
            TransactionStatus::Keep(ExecutionStatus::ExecutionFailure {
                location: AbortLocation::Script,
                ..
            })
        ),
        "v1 did not fail at the script: {:?}",
        v1_output.status()
    );
    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(v2_output.status(), v1_output.status());

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// A script that emits an event is refused with the same status as on v1.
#[test]
fn event_emitting_script_refused_like_v1() {
    let (fx, alice, _bob) = setup();
    let txn = script_txn(
        &alice,
        r#"
script
use 0x1::event

public fun main()
    ld_u64 1
    call event::emit<u64>
    ret
"#,
        vec![],
        10,
    );

    let v1_output = fx.execute_transaction(txn.clone());
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::INVALID_OPERATION_IN_SCRIPT
        ))),
        "v1 did not refuse the script: {:?}",
        v1_output.status()
    );
    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(v2_output.status(), v1_output.status());

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// A script that does not deserialize is kept with the same status as on v1.
#[test]
fn undeserializable_script_kept_like_v1() {
    let (fx, alice, _bob) = setup();
    let txn = script_bytes_txn(&alice, vec![0xDE, 0xAD, 0xBE, 0xEF], vec![], 10);

    let v1_output = fx.execute_transaction(txn.clone());
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::CODE_DESERIALIZATION_ERROR
        ))),
        "v1 did not reject the script: {:?}",
        v1_output.status()
    );
    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(v2_output.status(), v1_output.status());

    // Write sets are not compared: the payload fails before anything is
    // loaded, and v2 charges no intrinsic gas, so it writes no fee.
}

/// The same script run twice through one global context, hitting the script
/// cache the second time, behaves like on v1 both times.
#[test]
fn script_cache_hit_matches_v1() {
    let (mut fx, alice, bob) = setup();
    let args = || {
        vec![
            TransactionArgument::Address(*bob.address()),
            TransactionArgument::U64(1_000),
        ]
    };
    let txns = [
        script_txn(&alice, TRANSFER_SCRIPT, args(), 10),
        script_txn(&alice, TRANSFER_SCRIPT, args(), 11),
    ];

    let v2_outputs = execute_v2_sequence(fx.get_state_view(), &txns);
    for (txn, v2_output) in txns.iter().zip(&v2_outputs) {
        let v1_output = fx.execute_and_apply(txn.clone());
        assert_eq!(
            v1_output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success),
            "v1 rejected the script: {:?}",
            v1_output.status()
        );
        assert_eq!(v2_output.status(), v1_output.status());
        compare_outputs(&v1_output, v2_output, *alice.address());
    }
}

/// A transaction from `sender` calling `<address>::<module>::<function>` with
/// no type arguments and the given BCS-encoded arguments.
fn call_txn(
    sender: &AccountData,
    address: move_core_types::account_address::AccountAddress,
    module: &str,
    function: &str,
    args: Vec<Vec<u8>>,
) -> SignedTransaction {
    use aptos_types::transaction::{EntryFunction, TransactionPayload};
    use move_core_types::{identifier::Identifier, language_storage::ModuleId};

    sender
        .account()
        .transaction()
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(address, Identifier::new(module).unwrap()),
            Identifier::new(function).unwrap(),
            vec![],
            args,
        )))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign()
}

/// Asserts that both VMs keep `txn` with the miscellaneous error `code` and
/// agree, modulo gas, on the output.
fn assert_kept_with_code_like_v1(
    fx: &FakeExecutor,
    sender: &AccountData,
    txn: SignedTransaction,
    code: StatusCode,
) {
    let output = assert_output_matches_v1(fx, &txn, *sender.address());
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(code))),
        "v1 did not keep the transaction with {code:?}"
    );
}

/// Entry functions with signatures a transaction may not call. Assembled from
/// MASM, since the framework has none and the Move compiler would refuse some.
const UNCALLABLE_ENTRY_FUNCTIONS: &str = r#"
module 0xcafe::uncallable
use 0x1::option

struct Private has drop
  x: u64

entry public fun returns_value(): u64
    ld_u64 0
    ret

entry public fun takes_ref(x: &u64)
    ret

entry public fun signer_after_arg(x: u64, s: &signer)
    ret

entry public fun takes_option_of_private(o: option::Option<Private>)
    ret
"#;

/// Publishes `UNCALLABLE_ENTRY_FUNCTIONS` straight into `fx`'s state,
/// returning the module's address.
fn publish_uncallable_module(
    fx: &mut FakeExecutor,
) -> move_core_types::account_address::AccountAddress {
    let (module, blob) =
        aptos_language_e2e_tests::compile::compile_module(UNCALLABLE_ENTRY_FUNCTIONS);
    fx.add_module(&module.self_id(), blob.into_inner());
    *module.self_id().address()
}

/// A public function that is not `entry` is refused like on v1.
#[test]
fn non_entry_function_rejected_like_v1() {
    use move_core_types::account_address::AccountAddress;

    let (fx, alice, bob) = setup();
    let txn = call_txn(
        &alice,
        AccountAddress::ONE,
        "aptos_governance",
        "assert_proposal_expiration",
        vec![
            bcs::to_bytes(bob.address()).unwrap(),
            bcs::to_bytes(&0u64).unwrap(),
        ],
    );
    assert_kept_with_code_like_v1(
        &fx,
        &alice,
        txn,
        StatusCode::EXECUTE_ENTRY_FUNCTION_CALLED_ON_NON_ENTRY_FUNCTION,
    );
}

/// An entry function that returns values is refused like on v1.
#[test]
fn returning_function_rejected_like_v1() {
    let (mut fx, alice, _bob) = setup();
    let address = publish_uncallable_module(&mut fx);
    let txn = call_txn(&alice, address, "uncallable", "returns_value", vec![]);
    assert_kept_with_code_like_v1(
        &fx,
        &alice,
        txn,
        StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
    );
}

/// An entry function with a reference parameter is refused like on v1.
#[test]
fn reference_parameter_rejected_like_v1() {
    let (mut fx, alice, _bob) = setup();
    let address = publish_uncallable_module(&mut fx);
    let txn = call_txn(&alice, address, "uncallable", "takes_ref", vec![
        bcs::to_bytes(&0u64).unwrap(),
    ]);
    assert_kept_with_code_like_v1(
        &fx,
        &alice,
        txn,
        StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
    );
}

/// An entry function whose signer parameter follows another parameter is
/// refused like on v1.
#[test]
fn signer_after_argument_rejected_like_v1() {
    let (mut fx, alice, _bob) = setup();
    let address = publish_uncallable_module(&mut fx);
    let txn = call_txn(&alice, address, "uncallable", "signer_after_arg", vec![
        bcs::to_bytes(&0u64).unwrap(),
    ]);
    assert_kept_with_code_like_v1(
        &fx,
        &alice,
        txn,
        StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
    );
}

/// An entry function taking an `Option` of a private struct is refused like on
/// v1.
#[test]
fn option_of_private_struct_rejected_like_v1() {
    let (mut fx, alice, _bob) = setup();
    let address = publish_uncallable_module(&mut fx);
    // `Some(Private { x: 0 })`: v1 admits the type but fails to construct the
    // value, so it would accept a `None`.
    let txn = call_txn(
        &alice,
        address,
        "uncallable",
        "takes_option_of_private",
        vec![bcs::to_bytes(&vec![0u64]).unwrap()],
    );
    assert_kept_with_code_like_v1(
        &fx,
        &alice,
        txn,
        StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
    );
}

/// A native function is refused like on v1.
#[test]
fn native_function_rejected_like_v1() {
    use move_core_types::account_address::AccountAddress;

    let (fx, alice, _bob) = setup();
    let txn = call_txn(&alice, AccountAddress::ONE, "hash", "sha2_256", vec![
        bcs::to_bytes(&vec![1u8, 2, 3]).unwrap(),
    ]);
    assert_kept_with_code_like_v1(
        &fx,
        &alice,
        txn,
        StatusCode::USER_DEFINED_NATIVE_NOT_ALLOWED,
    );
}

/// Transactions violating the pre-execution gas bounds expressible by the
/// fixture's gas schedule are discarded with the same status code as V1,
/// before touching any state.
#[test]
fn gas_checks_discard_like_v1() {
    let (fx, alice, bob) = setup();
    let gas_params = TransactionGasParameters::initial();
    let max_gas = u64::from(gas_params.maximum_number_of_gas_units);
    let below_min_price = u64::from(gas_params.min_price_per_gas_unit).checked_sub(1);

    let transfer = |price: u64, max_gas: u64| {
        alice
            .account()
            .transaction()
            .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
                *bob.address(),
                1_000,
            ))
            .sequence_number(10)
            .gas_unit_price(price)
            .max_gas_amount(max_gas)
            .sign()
    };
    let oversized = alice
        .account()
        .transaction()
        .payload(
            aptos_cached_packages::aptos_stdlib::aptos_account_batch_transfer(
                vec![*bob.address(); 3_000],
                vec![1; 3_000],
            ),
        )
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    let mut cases = vec![
        (oversized, StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE),
        (
            transfer(100, max_gas + 1),
            StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,
        ),
        (
            transfer(100, 10),
            StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,
        ),
        (
            transfer(u64::MAX, 1_000_000),
            StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND,
        ),
        (
            encrypted_txn(
                &alice,
                encrypted_min_gas_unit_price() - 1,
                failed_decryption,
            ),
            StatusCode::ENCRYPTED_TXN_GAS_UNIT_PRICE_BELOW_MIN_BOUND,
        ),
    ];
    // The fixture's current minimum price is zero, for which no valid `u64`
    // gas price can be below the bound. Keep the check active when the
    // schedule raises that minimum.
    if let Some(price) = below_min_price {
        cases.push((
            transfer(price, 1_000_000),
            StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND,
        ));
    }
    for (txn, expected) in cases {
        let v1_status = fx.execute_transaction(txn.clone()).status().clone();
        assert_eq!(
            v1_status,
            TransactionStatus::Discard(expected),
            "v1 did not discard with {expected:?}"
        );
        let v2_output = execute_v2(fx.get_state_view(), &txn);
        assert_eq!(
            v2_output.status(),
            &v1_status,
            "v2 discard differs from v1 for {expected:?}"
        );
    }
}

/// An encrypted transaction. It is signed while still encrypted, the way a
/// client submits it, then moved to the state `after_decryption` returns.
fn encrypted_txn(
    sender: &AccountData,
    gas_unit_price: u64,
    after_decryption: impl FnOnce(EncryptedInner) -> EncryptedPayload,
) -> SignedTransaction {
    use aptos_crypto::HashValue;
    use aptos_types::{
        secret_sharing::Ciphertext,
        transaction::{TransactionExtraConfig, TransactionPayload},
    };

    let original = EncryptedInner {
        ciphertext: Ciphertext::random(),
        extra_config: TransactionExtraConfig::V1 {
            multisig_address: None,
            replay_protection_nonce: None,
        },
        payload_hash: HashValue::random(),
        encryption_epoch: 1,
        claimed_entry_fun: None,
    };
    let mut txn = sender
        .account()
        .transaction()
        .payload(TransactionPayload::EncryptedPayload(
            EncryptedPayload::Encrypted(original.clone()),
        ))
        .sequence_number(10)
        .gas_unit_price(gas_unit_price)
        .max_gas_amount(1_000_000)
        .sign();
    *txn.payload_mut() = TransactionPayload::EncryptedPayload(after_decryption(original));
    txn
}

/// A payload whose decryption failed.
fn failed_decryption(original: EncryptedInner) -> EncryptedPayload {
    EncryptedPayload::FailedDecryption {
        original,
        eval_proof: Some(EvalProof::random()),
        reason: DecryptionFailureReason::CryptoFailure,
    }
}

/// The minimum gas unit price an encrypted transaction must pay.
fn encrypted_min_gas_unit_price() -> u64 {
    TransactionGasParameters::initial()
        .encrypted_txn_min_price_per_gas_unit
        .into()
}

/// A transaction that could not be decrypted still commits, and its sequence
/// number is bumped so it cannot be replayed.
//
// Only the status is compared: nothing runs, so no fee is charged yet.
#[test]
fn undecrypted_payload_kept_like_v1() {
    use aptos_types::{account_config::AccountResource, state_store::state_key::StateKey};

    let (fx, alice, _bob) = setup();
    let txn = encrypted_txn(&alice, encrypted_min_gas_unit_price(), failed_decryption);

    let v1_output = fx.execute_transaction(txn.clone());
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT
        ))),
        "v1 did not keep the undecrypted transaction"
    );

    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(v2_output.status(), v1_output.status());

    // The sequence number lives in the sender's account resource.
    let account_key =
        StateKey::resource_typed::<AccountResource>(alice.address()).expect("the key builds");
    for (vm, output) in [("v1", &v1_output), ("v2", &v2_output)] {
        assert!(
            output
                .write_set()
                .write_op_iter()
                .any(|(key, _)| key == &account_key),
            "{vm} did not bump the sequence number"
        );
    }
}

/// A decrypted payload runs as the plain transaction it carries.
#[test]
fn decrypted_payload_matches_v1() {
    use aptos_types::transaction::{TransactionExecutable, TransactionPayload};

    let (fx, alice, bob) = setup();
    let TransactionPayload::EntryFunction(transfer) =
        aptos_cached_packages::aptos_stdlib::aptos_account_transfer(*bob.address(), 1_000)
    else {
        unreachable!("the SDK builds a transfer as an entry function")
    };
    let txn = encrypted_txn(&alice, encrypted_min_gas_unit_price(), |original| {
        EncryptedPayload::Decrypted {
            original,
            eval_proof: EvalProof::random(),
            decrypted: DecryptedPlaintext::new(
                TransactionExecutable::EntryFunction(transfer),
                [0u8; 16],
            ),
        }
    });

    let v1_output = fx.execute_transaction(txn.clone());
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "v1 rejected the decrypted transfer: {:?}",
        v1_output.status()
    );
    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(v2_output.status(), v1_output.status());
    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// Only a multisig transaction may leave out the executable.
#[test]
fn empty_payload_discarded_like_v1() {
    use aptos_types::transaction::{
        TransactionExecutable, TransactionExtraConfig, TransactionPayload, TransactionPayloadInner,
    };

    let (fx, alice, _bob) = setup();
    let txn = alice
        .account()
        .transaction()
        .payload(TransactionPayload::Payload(TransactionPayloadInner::V1 {
            executable: TransactionExecutable::Empty,
            extra_config: TransactionExtraConfig::V1 {
                multisig_address: None,
                replay_protection_nonce: None,
            },
        }))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    let v1_status = fx.execute_transaction(txn.clone()).status().clone();
    assert_eq!(
        v1_status,
        TransactionStatus::Discard(StatusCode::EMPTY_PAYLOAD_PROVIDED)
    );
    assert_eq!(execute_v2(fx.get_state_view(), &txn).status(), &v1_status);
}

/// An address with a balance but no account yet.
fn fund_fresh_account(fx: &mut FakeExecutor, funder: &AccountData, amount: u64) -> Account {
    let fresh = fx.create_raw_account();
    let fund = funder
        .account()
        .transaction()
        .payload(
            aptos_cached_packages::aptos_stdlib::aptos_account_fungible_transfer_only(
                *fresh.address(),
                amount,
            ),
        )
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();
    fx.execute_and_apply(fund);
    assert!(
        !has_account_resource(fx.get_state_view(), fresh.address()),
        "funding must not create the account"
    );
    fresh
}

/// Whether an account resource is stored at `address`.
fn has_account_resource<S: StateView>(
    state: &S,
    address: &move_core_types::account_address::AccountAddress,
) -> bool {
    use aptos_types::{account_config::AccountResource, state_store::state_key::StateKey};

    let key = StateKey::resource_typed::<AccountResource>(address).expect("the account key builds");
    state
        .get_state_value_bytes(&key)
        .expect("the state is readable")
        .is_some()
}

/// A transfer that is `sender`'s first transaction.
fn first_transfer(
    sender: &Account,
    to: &AccountData,
    amount: u64,
    max_gas: u64,
) -> SignedTransaction {
    sender
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            *to.address(),
            amount,
        ))
        .sequence_number(0)
        .gas_unit_price(100)
        .max_gas_amount(max_gas)
        .sign()
}

/// `status` with any abort info stripped.
//
// TODO(correctness): compare the abort info too, once the executor resolves it.
fn without_abort_info(status: &TransactionStatus) -> TransactionStatus {
    match status {
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { location, code, .. }) => {
            TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                location: location.clone(),
                code: *code,
                info: None,
            })
        },
        TransactionStatus::Keep(
            ExecutionStatus::Success
            | ExecutionStatus::OutOfGas
            | ExecutionStatus::ExecutionFailure { .. }
            | ExecutionStatus::MiscellaneousError(_),
        )
        | TransactionStatus::Discard(_)
        | TransactionStatus::Retry => status.clone(),
    }
}

/// Asserts both VMs agree on `txn`, modulo gas, and returns V1's output.
fn assert_output_matches_v1(
    fx: &FakeExecutor,
    txn: &SignedTransaction,
    fee_payer: move_core_types::account_address::AccountAddress,
) -> TransactionOutput {
    let v1_output = fx.execute_transaction(txn.clone());
    let v2_output = execute_v2(fx.get_state_view(), txn);
    assert_eq!(
        without_abort_info(v2_output.status()),
        without_abort_info(v1_output.status()),
        "v2 status differs from v1"
    );
    compare_outputs(&v1_output, &v2_output, fee_payer);
    v1_output
}

/// Entry functions that call the randomness API, of every visibility. All but
/// `unannotated` get the `#[randomness]` annotation when published.
const RANDOMNESS_ENTRY_FUNCTIONS: &str = r#"
module 0xcafe::randomness_users
use 0x1::randomness

entry fun annotated()
    call randomness::u64_integer
    pop
    ret

entry friend fun friend_annotated()
    call randomness::u64_integer
    pop
    ret

entry fun unannotated()
    call randomness::u64_integer
    pop
    ret

entry public fun public_annotated()
    call randomness::u64_integer
    pop
    ret
"#;

/// Publishes `RANDOMNESS_ENTRY_FUNCTIONS` with the annotation attached to the
/// functions whose name says so, and returns the module's address.
fn publish_randomness_module(
    fx: &mut FakeExecutor,
) -> move_core_types::account_address::AccountAddress {
    use aptos_types::vm::module_metadata::{
        KnownAttribute, RuntimeModuleMetadataV1, APTOS_METADATA_KEY_V1,
    };
    use move_core_types::metadata::Metadata;

    let (mut module, _) =
        aptos_language_e2e_tests::compile::compile_module(RANDOMNESS_ENTRY_FUNCTIONS);
    let metadata = RuntimeModuleMetadataV1 {
        fun_attributes: ["annotated", "friend_annotated", "public_annotated"]
            .into_iter()
            .map(|name| (name.to_string(), vec![KnownAttribute::randomness(None)]))
            .collect(),
        ..Default::default()
    };
    module.metadata.push(Metadata {
        key: APTOS_METADATA_KEY_V1.to_vec(),
        value: bcs::to_bytes(&metadata).expect("the metadata serializes"),
    });

    let mut blob = vec![];
    module
        .serialize(&mut blob)
        .expect("the annotated module serializes");
    fx.add_module(&module.self_id(), blob);
    *module.self_id().address()
}

/// Runs a block prologue carrying a randomness seed, which genesis leaves
/// unset, so the randomness API has something to derive from.
fn seed_block_randomness(fx: &mut FakeExecutor) {
    let block_metadata_ext = block_metadata_with_randomness_seed(fx);
    let outputs = fx
        .execute_transaction_block(vec![Transaction::BlockMetadataExt(block_metadata_ext)])
        .expect("v1 executes the block");
    fx.apply_write_set(outputs[0].write_set());
}

/// Only a private or friend entry function carrying the `#[randomness]`
/// annotation may call the randomness API. Each case is compared against V1
/// rather than against fixed abort codes.
#[test]
fn randomness_api_matches_v1() {
    const RANDOMNESS_EVENT: &str = "0x1::randomness::RandomnessGeneratedEvent";

    let (mut fx, alice, _bob) = setup();
    seed_block_randomness(&mut fx);
    let module = publish_randomness_module(&mut fx);
    let call = |function: &str| call_txn(&alice, module, "randomness_users", function, vec![]);

    for function in ["annotated", "friend_annotated"] {
        let output = assert_output_matches_v1(&fx, &call(function), *alice.address());
        assert_eq!(
            output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success),
            "{function} could not use randomness"
        );
        assert!(
            output
                .events()
                .iter()
                .any(|event| event.type_tag().to_canonical_string() == RANDOMNESS_EVENT),
            "{function} used randomness without the framework recording it"
        );
    }

    let unannotated = assert_output_matches_v1(&fx, &call("unannotated"), *alice.address());
    assert!(
        matches!(
            unannotated.status(),
            TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
        ),
        "an unannotated function used randomness: {:?}",
        unannotated.status()
    );
    let public_annotated =
        assert_output_matches_v1(&fx, &call("public_annotated"), *alice.address());
    assert_eq!(
        public_annotated.status(),
        unannotated.status(),
        "a public entry function must not be allowed to use randomness"
    );
}

/// A first transaction from an address with no account creates one.
#[test]
fn first_transaction_creates_account_like_v1() {
    let (mut fx, alice, bob) = setup();
    let carol = fund_fresh_account(&mut fx, &alice, 100_000_000);
    let txn = first_transfer(&carol, &bob, 1_000, 1_000_000);
    let output = assert_output_matches_v1(&fx, &txn, *carol.address());
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
}

/// The new account survives a payload that aborts.
#[test]
fn account_created_when_payload_aborts_like_v1() {
    let (mut fx, alice, bob) = setup();
    let carol = fund_fresh_account(&mut fx, &alice, 100_000_000);
    let txn = first_transfer(&carol, &bob, 1_000_000_000, 1_000_000);
    let output = assert_output_matches_v1(&fx, &txn, *carol.address());
    assert!(
        matches!(
            output.status(),
            TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
        ),
        "the payload did not abort: {:?}",
        output.status()
    );
}

/// A budget too small to pay for the new account is rejected.
#[test]
fn unaffordable_account_creation_discarded_like_v1() {
    let (mut fx, alice, bob) = setup();
    let carol = fund_fresh_account(&mut fx, &alice, 100_000_000);
    let txn = first_transfer(&carol, &bob, 1_000, 100);
    let v1_status = fx.execute_transaction(txn.clone()).status().clone();
    assert_eq!(
        v1_status,
        TransactionStatus::Discard(StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS)
    );
    assert_eq!(execute_v2(fx.get_state_view(), &txn).status(), &v1_status);
}

/// An account that already exists is not created again.
#[test]
fn existing_account_first_transaction_matches_v1() {
    let (fx, alice, bob) = setup();
    let txn = first_transfer(bob.account(), &alice, 1_000, 1_000_000);
    let output = assert_output_matches_v1(&fx, &txn, *bob.address());
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
}

/// A block-metadata transaction produces byte-identical outputs on both VMs:
/// the block prologue runs unmetered on both, so nothing is gas-masked.
#[test]
fn block_metadata_matches_v1() {
    use aptos_types::{
        block_metadata::BlockMetadata,
        on_chain_config::{OnChainConfig, ValidatorSet},
    };

    let (mut fx, _alice, _bob) = setup();
    let validator_set = ValidatorSet::fetch_config(fx.get_state_view())
        .expect("the validator set is readable")
        .expect("genesis has a validator set");
    let proposer = *validator_set
        .payload()
        .next()
        .expect("genesis has a validator")
        .account_address();
    let block_metadata = BlockMetadata::new(
        aptos_crypto::HashValue::sha3_256_of(b"mono-move block"),
        1,
        1,
        proposer,
        vec![],
        vec![],
        fx.get_block_time() + 100,
    );

    let v1_outputs = fx
        .execute_transaction_block(vec![Transaction::BlockMetadata(block_metadata.clone())])
        .expect("v1 executes the block");
    let v1_output = &v1_outputs[0];
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "v1 rejected the block metadata: {:?}",
        v1_output.status()
    );

    let v2_output = execute_v2_block_metadata(fx.get_state_view(), &block_metadata);
    compare_system_outputs(v1_output, &v2_output);
}

/// A V3 extended block-metadata transaction produces byte-identical outputs
/// on both VMs. The randomness seed exercises the option-argument encoding;
/// the decryption payload stays absent, as on a chain without pending
/// encrypted transactions.
#[test]
fn block_metadata_ext_v3_matches_v1() {
    let (mut fx, _alice, _bob) = setup();
    let block_metadata_ext = block_metadata_with_randomness_seed(&mut fx);

    let v1_outputs = fx
        .execute_transaction_block(vec![Transaction::BlockMetadataExt(
            block_metadata_ext.clone(),
        )])
        .expect("v1 executes the block");
    let v1_output = &v1_outputs[0];
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "v1 rejected the ext block metadata: {:?}",
        v1_output.status()
    );

    let v2_output = execute_v2_with(fx.get_state_view(), |executor| {
        executor.execute_transaction(
            &Transaction::BlockMetadataExt(block_metadata_ext.clone()),
            &first_txn_aux_info(),
        )
    });
    compare_system_outputs(v1_output, &v2_output);
}

/// Extended block metadata carrying a randomness seed and no decryption
/// payload, proposed by a genesis validator.
fn block_metadata_with_randomness_seed(
    fx: &mut FakeExecutor,
) -> aptos_types::block_metadata_ext::BlockMetadataExt {
    use aptos_types::{
        block_metadata_ext::BlockMetadataExt,
        on_chain_config::{OnChainConfig, ValidatorSet},
        randomness::{RandMetadata, Randomness},
    };

    let validator_set = ValidatorSet::fetch_config(fx.get_state_view())
        .expect("the validator set is readable")
        .expect("genesis has a validator set");
    let proposer = *validator_set
        .payload()
        .next()
        .expect("genesis has a validator")
        .account_address();
    BlockMetadataExt::new_v3(
        aptos_crypto::HashValue::sha3_256_of(b"mono-move ext block"),
        1,
        1,
        proposer,
        vec![],
        vec![],
        fx.get_block_time() + 100,
        Some(Randomness::new(
            RandMetadata { epoch: 1, round: 1 },
            vec![7u8; 32],
        )),
        None,
    )
}

/// Both system-transaction outputs must be fee-free and match byte-for-byte,
/// except state-value metadata (slot deposits, creation time), which V2 does
/// not stamp yet.
fn compare_system_outputs(v1_output: &TransactionOutput, v2_output: &TransactionOutput) {
    assert_eq!(v2_output.status(), v1_output.status());
    assert_eq!(v1_output.gas_used(), 0);
    assert_eq!(v2_output.gas_used(), 0);

    let v1_writes: BTreeMap<_, _> = v1_output.write_set().write_op_iter().collect();
    let v2_writes: BTreeMap<_, _> = v2_output.write_set().write_op_iter().collect();
    assert_eq!(
        v1_writes.keys().collect::<Vec<_>>(),
        v2_writes.keys().collect::<Vec<_>>(),
        "the two VMs wrote different sets of state keys"
    );
    for (key, v1_op) in &v1_writes {
        use aptos_types::write_set::TransactionWrite;
        let v2_op = &v2_writes[*key];
        assert_eq!(
            v1_op.write_op_kind(),
            v2_op.write_op_kind(),
            "write kinds differ for {key:?}"
        );
        assert_eq!(
            v1_op.bytes(),
            v2_op.bytes(),
            "write bytes differ for {key:?}"
        );
    }
    assert_eq!(v1_output.events(), v2_output.events(), "events differ");
}

/// A V0 block epilogue runs nothing on-chain: both VMs commit an empty
/// success output.
#[test]
fn block_epilogue_v0_is_empty_like_v1() {
    use aptos_types::transaction::BlockEndInfo;

    let (fx, _alice, _bob) = setup();
    let block_epilogue = Transaction::block_epilogue_v0(
        aptos_crypto::HashValue::sha3_256_of(b"mono-move epilogue block"),
        BlockEndInfo::new_empty(),
    );

    let v1_outputs = fx
        .execute_transaction_block(vec![block_epilogue.clone()])
        .expect("v1 executes the block");
    let v1_output = &v1_outputs[0];
    assert!(
        v1_output.write_set().write_op_iter().next().is_none(),
        "a V0 block epilogue must write nothing"
    );

    let v2_output = execute_v2_with(fx.get_state_view(), |executor| {
        executor.execute_transaction(&block_epilogue, &first_txn_aux_info())
    });
    compare_system_outputs(v1_output, &v2_output);
}

/// A state-checkpoint transaction runs nothing on-chain: both VMs commit an
/// empty success.
#[test]
fn state_checkpoint_is_empty_like_v1() {
    let (fx, _alice, _bob) = setup();
    let state_checkpoint = Transaction::StateCheckpoint(aptos_crypto::HashValue::sha3_256_of(
        b"mono-move checkpoint",
    ));

    let v1_outputs = fx
        .execute_transaction_block(vec![state_checkpoint.clone()])
        .expect("v1 executes the block");
    let v1_output = &v1_outputs[0];
    assert!(
        v1_output.write_set().write_op_iter().next().is_none(),
        "a state checkpoint must write nothing"
    );

    let v2_output = execute_v2_with(fx.get_state_view(), |executor| {
        executor.execute_transaction(&state_checkpoint, &first_txn_aux_info())
    });
    compare_system_outputs(v1_output, &v2_output);
}

/// A V1 block epilogue distributes the block's transaction fees to its
/// validators via `0x1::block::block_epilogue`, producing byte-identical
/// outputs on both VMs.
#[test]
fn block_epilogue_v1_matches_v1() {
    use aptos_types::transaction::{BlockEndInfoExt, FeeDistribution};

    let (fx, _alice, _bob) = setup();
    let block_epilogue = Transaction::block_epilogue_v1(
        aptos_crypto::HashValue::sha3_256_of(b"mono-move epilogue block"),
        BlockEndInfoExt::new_empty(),
        // Genesis has a single validator, at index 0.
        FeeDistribution::new(BTreeMap::from([(0, 100_000)])),
    );

    let v1_outputs = fx
        .execute_transaction_block(vec![block_epilogue.clone()])
        .expect("v1 executes the block");
    let v1_output = &v1_outputs[0];
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "v1 rejected the block epilogue: {:?}",
        v1_output.status()
    );
    // Guard against v1 hitting its swallow-the-failure fallback, which would
    // make the comparison below vacuous.
    assert!(
        v1_output.write_set().write_op_iter().next().is_some(),
        "v1 recorded no fee"
    );

    let v2_output = execute_v2_with(fx.get_state_view(), |executor| {
        executor.execute_transaction(&block_epilogue, &first_txn_aux_info())
    });
    compare_system_outputs(v1_output, &v2_output);
}

/// Two dependent transfers executed sequentially, each transaction's outputs
/// applied to the state before the next: the second transaction's prologue
/// only passes if it observes the first one's sequence-number bump.
#[test]
fn sequential_execution_applies_outputs() {
    use aptos_transaction_simulation::{DeltaStateStore, SimulationStateStore};

    let (fx, alice, bob) = setup();

    let transfer = |seq| {
        alice
            .account()
            .transaction()
            .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
                *bob.address(),
                1_000,
            ))
            .sequence_number(seq)
            .gas_unit_price(100)
            .max_gas_amount(1_000_000)
            .sign()
    };

    let state = DeltaStateStore::new_with_base(fx.get_state_view());
    for (i, txn) in [transfer(10), transfer(11)].iter().enumerate() {
        let output = execute_v2(&state, txn);
        assert_eq!(
            output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success),
            "transaction {i} failed"
        );
        state
            .apply_write_set(output.write_set())
            .expect("write set applies");
    }
}

/// The state keys whose content embeds the transaction fee: the fee payer's
/// primary fungible store group and the APT metadata object group (supply).
fn gas_dependent_keys(
    fee_payer: move_core_types::account_address::AccountAddress,
) -> Vec<aptos_types::state_store::state_key::StateKey> {
    use aptos_types::state_store::state_key::StateKey;
    use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
    use std::str::FromStr;

    let object_group = StructTag::from_str("0x1::object::ObjectGroup").unwrap();
    let store = aptos_types::account_config::fungible_store::primary_apt_store(fee_payer);
    vec![
        StateKey::resource_group(&store, &object_group),
        StateKey::resource_group(&AccountAddress::TEN, &object_group),
    ]
}
