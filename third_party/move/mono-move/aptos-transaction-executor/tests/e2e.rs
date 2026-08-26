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
use aptos_language_e2e_tests::{account::AccountData, executor::FakeExecutor};
use aptos_types::{
    state_store::StateView,
    transaction::{
        AuxiliaryInfo, ExecutionStatus, PersistedAuxiliaryInfo, SignedTransaction, Transaction,
        TransactionAuxiliaryData, TransactionOutput, TransactionStatus,
    },
};
use aptos_vm_environment::environment::AptosEnvironment;
use mono_move_aptos_state_view_providers::{StateViewModuleProvider, StateViewResourceProvider};
use mono_move_aptos_transaction_executor::{
    production_natives, AptosTransactionExecutor, TxnOutcome,
};
use mono_move_global_context::GlobalContext;
use move_core_types::vm_status::StatusCode;
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

/// Builds the executor against `state`, runs one transaction through it, and
/// materializes the outcome.
fn execute_v2_with<S: StateView>(
    state: &S,
    run: impl for<'guard> FnOnce(&AptosTransactionExecutor<'guard>) -> TxnOutcome<'guard>,
) -> TransactionOutput {
    let global_ctx = GlobalContext::with_num_execution_workers(1);
    let guard = global_ctx
        .try_execution_context(0)
        .expect("execution context is available");
    let natives = production_natives(&guard);
    let module_provider = StateViewModuleProvider::new(state);
    let data_provider = StateViewResourceProvider::new(&guard, state);
    let env = AptosEnvironment::new(state);
    let usage = state.get_usage().expect("usage is readable");
    let executor = AptosTransactionExecutor::new(
        &guard,
        &natives,
        &module_provider,
        &data_provider,
        &env,
        usage,
    );
    run(&executor)
        .materialize(
            &data_provider,
            env.features(),
            TransactionAuxiliaryData::default(),
        )
        .expect("the transaction output materializes")
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
    assert!(
        matches!(
            v2_output.status(),
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(_))
        ),
        "v2 did not reject the signer-count mismatch: {:?}",
        v2_output.status()
    );

    // Write sets are not compared: the gas divergence leaves v1 with fee
    // writes v2 lacks.
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
    use aptos_types::{
        block_metadata_ext::BlockMetadataExt,
        on_chain_config::{OnChainConfig, ValidatorSet},
        randomness::{RandMetadata, Randomness},
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
    let block_metadata_ext = BlockMetadataExt::new_v3(
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
    );

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
