// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use aptos_gas_algebra::Gas;
use aptos_gas_schedule::{InitialGasSchedule, TransactionGasParameters};
use aptos_language_e2e_tests::{
    assert_prologue_disparity, assert_prologue_parity, common_transactions::EMPTY_SCRIPT,
    current_function_name, executor::FakeExecutor, transaction_status_eq,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config,
    chain_id::ChainId,
    on_chain_config::FeatureFlag,
    test_helpers::transaction_test_helpers,
    transaction::{ExecutionStatus, Script, TransactionArgument, TransactionStatus},
    vm_status::StatusCode,
};
use move_binary_format::file_format::CompiledModule;
use move_core_types::{
    gas_algebra::GasQuantity,
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use move_ir_compiler::Compiler;
use test_case::test_case;

pub const MAX_TRANSACTION_SIZE_IN_BYTES: u64 = 6 * 1024 * 1024;

fn executor_with_lazy_loading(enable_lazy_loading: bool) -> FakeExecutor {
    let mut executor = FakeExecutor::from_head_genesis();
    let addr = AccountAddress::ONE;
    if enable_lazy_loading {
        executor.enable_features(&addr, vec![FeatureFlag::ENABLE_LAZY_LOADING], vec![]);
    } else {
        executor.enable_features(&addr, vec![], vec![FeatureFlag::ENABLE_LAZY_LOADING]);
    }
    executor
}

fn success_if_lazy_loading_enabled_or_invariant_violation(
    enable_lazy_loading: bool,
    status: TransactionStatus,
) {
    if enable_lazy_loading {
        assert!(matches!(
            status,
            TransactionStatus::Keep(ExecutionStatus::Success)
        ));
    } else {
        assert!(matches!(
            status,
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                StatusCode::UNEXPECTED_VERIFIER_ERROR
            )))
        ));
    }
}

#[test]
fn verify_signature() {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = executor.create_raw_account_data(900_000, Some(10));
    executor.add_account_data(&sender);
    // Generate a new key pair to try and sign things with.
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let program = aptos_stdlib::aptos_coin_transfer(*sender.address(), 100);
    let signed_txn = transaction_test_helpers::get_test_unchecked_txn(
        *sender.address(),
        0,
        &private_key,
        sender.account().pubkey.as_ed25519().unwrap(),
        program,
    );

    assert_prologue_parity!(
        executor.validate_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        StatusCode::INVALID_SIGNATURE
    );
}

#[test]
fn verify_multi_agent_invalid_sender_signature() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    let sender = executor.create_raw_account_data(1_000_010, Some(10));
    let secondary_signer = executor.create_raw_account_data(100_100, Some(100));

    executor.add_account_data(&sender);
    executor.add_account_data(&secondary_signer);

    let private_key = Ed25519PrivateKey::generate_for_testing();

    // Sign using the wrong key for the sender, and correct key for the secondary signer.
    let signed_txn = transaction_test_helpers::get_test_unchecked_multi_agent_txn(
        *sender.address(),
        vec![*secondary_signer.address()],
        10,
        &private_key,
        sender.account().pubkey.as_ed25519().unwrap(),
        vec![&secondary_signer.account().privkey],
        vec![secondary_signer.account().pubkey.as_ed25519().unwrap()],
        None,
    );
    assert_prologue_parity!(
        executor.validate_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        StatusCode::INVALID_SIGNATURE
    );
}

#[test]
fn verify_multi_agent_invalid_secondary_signature() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());
    let sender = executor.create_raw_account_data(1_000_010, Some(10));
    let secondary_signer = executor.create_raw_account_data(100_100, Some(100));

    executor.add_account_data(&sender);
    executor.add_account_data(&secondary_signer);

    let private_key = Ed25519PrivateKey::generate_for_testing();

    // Sign using the correct keys for the sender, but wrong keys for the secondary signer.
    let signed_txn = transaction_test_helpers::get_test_unchecked_multi_agent_txn(
        *sender.address(),
        vec![*secondary_signer.address()],
        10,
        &sender.account().privkey,
        sender.account().pubkey.as_ed25519().unwrap(),
        vec![&private_key],
        vec![secondary_signer.account().pubkey.as_ed25519().unwrap()],
        None,
    );
    assert_prologue_parity!(
        executor.validate_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        StatusCode::INVALID_SIGNATURE
    );
}

#[test]
fn verify_multi_agent_duplicate_secondary_signer() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());
    let sender = executor.create_raw_account_data(1_000_010, Some(10));
    let secondary_signer = executor.create_raw_account_data(100_100, Some(100));
    let third_signer = executor.create_raw_account_data(100_100, Some(100));

    executor.add_account_data(&sender);
    executor.add_account_data(&secondary_signer);
    executor.add_account_data(&third_signer);

    // Duplicates in secondary signers.
    let signed_txn = transaction_test_helpers::get_test_unchecked_multi_agent_txn(
        *sender.address(),
        vec![
            *secondary_signer.address(),
            *third_signer.address(),
            *secondary_signer.address(),
        ],
        10,
        &sender.account().privkey,
        sender.account().pubkey.as_ed25519().unwrap(),
        vec![
            &secondary_signer.account().privkey,
            &third_signer.account().privkey,
            &secondary_signer.account().privkey,
        ],
        vec![
            secondary_signer.account().pubkey.as_ed25519().unwrap(),
            third_signer.account().pubkey.as_ed25519().unwrap(),
            secondary_signer.account().pubkey.as_ed25519().unwrap(),
        ],
        None,
    );
    assert_prologue_parity!(
        executor.validate_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        StatusCode::SIGNERS_CONTAIN_DUPLICATES
    );
}

#[test]
fn verify_reserved_sender() {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = executor.create_raw_account_data(900_000, Some(10));
    executor.add_account_data(&sender);
    // Generate a new key pair to try and sign things with.
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let program = aptos_stdlib::aptos_coin_transfer(*sender.address(), 100);
    let signed_txn = transaction_test_helpers::get_test_signed_txn(
        account_config::reserved_vm_address(),
        0,
        &private_key,
        private_key.public_key(),
        Some(program),
    );

    assert_prologue_parity!(
        executor.validate_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        StatusCode::INVALID_AUTH_KEY
    );
}

#[test]
fn verify_simple_payment() {
    let mut executor = FakeExecutor::from_head_genesis();
    // create and publish a sender with 1_000_000 coins and a receiver with 100_000 coins
    let sender = executor.create_raw_account_data(900_000, Some(10));
    let receiver = executor.create_raw_account_data(100_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    // define the arguments to the peer to peer transaction
    let transfer_amount = 1_000;

    let empty_script = &*EMPTY_SCRIPT;

    // Create a new transaction that has the exact right sequence number.
    let txn = sender
        .account()
        .transaction()
        .payload(aptos_stdlib::aptos_coin_transfer(
            *receiver.address(),
            transfer_amount,
        ))
        .sequence_number(10)
        .sign();
    assert_eq!(executor.validate_transaction(txn).status(), None);

    // Create a new transaction that has the bad auth key.
    let txn = receiver
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(10)
        .max_gas_amount(100_000)
        .gas_unit_price(1)
        .raw()
        .sign(
            &sender.account().privkey,
            sender.account().pubkey.as_ed25519().unwrap(),
        )
        .unwrap()
        .into_inner();

    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::INVALID_AUTH_KEY
    );

    // Create a new transaction that has a old sequence number.
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(1)
        .sign();
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::SEQUENCE_NUMBER_TOO_OLD
    );

    // Create a new transaction that has a too new sequence number.
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(11)
        .sign();
    assert_prologue_disparity!(
        executor.validate_transaction(txn.clone()).status() => None,
        executor.execute_transaction(txn).status() =>
        TransactionStatus::Discard(StatusCode::SEQUENCE_NUMBER_TOO_NEW)
    );

    // Create a new transaction that doesn't have enough balance to pay for gas.
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(10)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE
    );

    // Create a new transaction from a bogus account that doesn't exist
    let bogus_account = executor.create_raw_account_data(100_000, Some(10));
    let txn = bogus_account
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(0)
        .sign();
    assert_prologue_disparity!(
        executor.validate_transaction(txn.clone()).status() => None,
        executor.execute_transaction(txn).status() => TransactionStatus::Keep(ExecutionStatus::Success)
    );

    // The next couple tests test transaction size, and bounds on gas price and the number of
    // gas units that can be submitted with a transaction.
    //
    // We test these in the reverse order that they appear in verify_transaction, and build up
    // the errors one-by-one to make sure that we are both catching all of them, and
    // that we are doing so in the specified order.
    let txn_gas_params = TransactionGasParameters::initial();

    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(10)
        .gas_unit_price((txn_gas_params.max_price_per_gas_unit + GasQuantity::one()).into())
        .max_gas_amount(1_000_000)
        .sign();
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND
    );

    // Test for a max_gas_amount that is insufficient to pay the minimum fee.
    // Find the minimum transaction gas units and subtract 1.
    let mut gas_limit: Gas =
        (txn_gas_params.min_transaction_gas_units).to_unit_round_up_with_params(&txn_gas_params);

    if gas_limit > 0.into() {
        gas_limit = gas_limit.checked_sub(1.into()).unwrap();
    }
    // Calculate how many extra bytes of transaction arguments to add to ensure
    // that the minimum transaction gas gets rounded up when scaling to the
    // external gas units. (Ignore the size of the script itself for simplicity.)
    let extra_txn_bytes = if u64::from(txn_gas_params.gas_unit_scaling_factor)
        > u64::from(txn_gas_params.min_transaction_gas_units)
    {
        txn_gas_params.large_transaction_cutoff
            + GasQuantity::from(
                u64::from(txn_gas_params.gas_unit_scaling_factor)
                    / u64::from(txn_gas_params.intrinsic_gas_per_byte),
            )
    } else {
        0.into()
    };
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(
            empty_script.clone(),
            vec![],
            vec![TransactionArgument::U8(42); u64::from(extra_txn_bytes) as usize],
        ))
        .sequence_number(10)
        .max_gas_amount(gas_limit.into())
        .gas_unit_price(txn_gas_params.max_price_per_gas_unit.into())
        .sign();
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS
    );

    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(10)
        .max_gas_amount((txn_gas_params.maximum_number_of_gas_units + GasQuantity::one()).into())
        .gas_unit_price((txn_gas_params.max_price_per_gas_unit).into())
        .sign();
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND
    );

    let txn = sender
        .account()
        .transaction()
        .script(Script::new(
            empty_script.clone(),
            vec![],
            vec![TransactionArgument::U8(42); MAX_TRANSACTION_SIZE_IN_BYTES as usize],
        ))
        .sequence_number(10)
        .max_gas_amount((txn_gas_params.maximum_number_of_gas_units + GasQuantity::one()).into())
        .gas_unit_price((txn_gas_params.max_price_per_gas_unit).into())
        .sign();
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE
    );

    // Create a new transaction with wrong argument.

    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![
            TransactionArgument::U8(42),
        ]))
        .sequence_number(10)
        .max_gas_amount(100_000)
        .gas_unit_price(1)
        .sign();
    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH
        )))
    );
}

#[test]
pub fn test_arbitrary_script_execution() {
    // create a FakeExecutor with a genesis from file
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create an empty transaction
    let sender = executor.create_raw_account_data(1_000_000, Some(10));
    executor.add_account_data(&sender);

    // If CustomScripts is on, result should be Keep(DeserializationError). If it's off, the
    // result should be Keep(UnknownScript)
    let random_script = vec![];
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(random_script, vec![], vec![]))
        .sequence_number(10)
        .max_gas_amount(100_000)
        .gas_unit_price(1)
        .sign();
    assert_eq!(executor.validate_transaction(txn.clone()).status(), None);
    let status = executor.execute_transaction(txn).status().clone();
    assert!(!status.is_discarded());
    assert_eq!(
        status.status(),
        // StatusCode::CODE_DESERIALIZATION_ERROR
        Ok(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::CODE_DESERIALIZATION_ERROR
        )))
    );
}

#[test]
fn verify_expiration_time() {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = executor.create_raw_account_data(900_000, Some(0));
    executor.add_account_data(&sender);
    let private_key = &sender.account().privkey;
    let txn = transaction_test_helpers::get_test_signed_transaction(
        *sender.address(),
        0, /* sequence_number */
        private_key,
        private_key.public_key(),
        None, /* script */
        0,    /* expiration_time */
        0,    /* gas_unit_price */
        None, /* max_gas_amount */
    );
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::TRANSACTION_EXPIRED
    );

    // 10 is picked to make sure that SEQUENCE_NUMBER_TOO_NEW will not override the
    // TRANSACTION_EXPIRED error.
    let txn = transaction_test_helpers::get_test_signed_transaction(
        *sender.address(),
        10, /* sequence_number */
        private_key,
        private_key.public_key(),
        None, /* script */
        0,    /* expiration_time */
        0,    /* gas_unit_price */
        None, /* max_gas_amount */
    );
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::TRANSACTION_EXPIRED
    );
}

#[test]
fn verify_chain_id() {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = executor.create_raw_account_data(900_000, Some(0));
    executor.add_account_data(&sender);
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let txn = transaction_test_helpers::get_test_txn_with_chain_id(
        *sender.address(),
        0,
        &private_key,
        private_key.public_key(),
        // all tests use ChainId::test() for chain_id,so pick something different
        ChainId::new(ChainId::test().id() + 1),
    );
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::BAD_CHAIN_ID
    );
}

#[test]
fn verify_max_sequence_number() {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = executor.create_raw_account_data(900_000, Some(u64::MAX));
    executor.add_account_data(&sender);
    let private_key = &sender.account().privkey;
    let txn = transaction_test_helpers::get_test_signed_transaction(
        *sender.address(),
        u64::MAX, /* sequence_number */
        private_key,
        private_key.public_key(),
        None,     /* script */
        u64::MAX, /* expiration_time */
        0,        /* gas_unit_price */
        None,     /* max_gas_amount */
    );
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::SEQUENCE_NUMBER_TOO_BIG
    );
}

fn bad_module() -> (CompiledModule, Vec<u8>) {
    let bad_module_code = "
    module 0x1.Test {
        struct R1 { b: bool }
        struct S1 has copy, drop { r1: Self.R1 }

        public new_S1(): Self.S1 {
            let s: Self.S1;
            let r: Self.R1;
        label b0:
            r = R1 { b: true };
            s = S1 { r1: move(r) };
            return move(s);
        }
    }
    ";
    let compiler = Compiler { deps: vec![] };
    let module = compiler
        .into_compiled_module(bad_module_code)
        .expect("Failed to compile");
    let mut bytes = vec![];
    module.serialize(&mut bytes).unwrap();
    (module, bytes)
}

fn good_module_uses_bad(
    address: AccountAddress,
    bad_dep: &CompiledModule,
) -> (CompiledModule, Vec<u8>) {
    let good_module_code = format!(
        "
    module 0x{}.Test2 {{
        import 0x1.Test;
        struct S {{ b: bool }}

        public foo() {{
            let s: Test.S1;
        label b0:
            s = Test.new_S1();
            return;
        }}
        public bar() {{
        label b0:
            return;
        }}
    }}
    ",
        address.to_hex(),
    );

    let framework_modules = aptos_cached_packages::head_release_bundle().compiled_modules();
    let compiler = Compiler {
        deps: framework_modules
            .iter()
            .chain(std::iter::once(bad_dep))
            .collect(),
    };
    let module = compiler
        .into_compiled_module(good_module_code.as_str())
        .expect("Failed to compile");
    let mut bytes = vec![];
    module.serialize(&mut bytes).unwrap();
    (module, bytes)
}

fn execute_script_for_test(
    executor: &mut FakeExecutor,
    script_code: &str,
    ty_args: Vec<TypeTag>,
    link_to_good_module: bool,
) -> TransactionStatus {
    // Get a module that fails verification into the store.
    let (bad_module, bad_module_bytes) = bad_module();
    executor.add_module(&bad_module.self_id(), bad_module_bytes);

    // Create a module that tries to use that module.
    let (good_module, good_module_bytes) =
        good_module_uses_bad(account_config::CORE_CODE_ADDRESS, &bad_module);
    executor.add_module(&good_module.self_id(), good_module_bytes);

    let deps = if link_to_good_module {
        vec![&good_module]
    } else {
        vec![&bad_module]
    };
    let compiler = Compiler { deps };

    let script = compiler
        .into_script_blob(script_code)
        .expect("Failed to compile");

    // Create a transaction that tries to use that module.
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    let txn = sender
        .account()
        .transaction()
        .script(Script::new(script, ty_args, vec![]))
        .sequence_number(10)
        .max_gas_amount(100_000)
        .gas_unit_price(1)
        .sign();
    assert_eq!(executor.validate_transaction(txn.clone()).status(), None);
    executor.execute_transaction(txn).status().clone()
}

#[test_case(true)]
#[test_case(false)]
fn test_script_dependency_fails_verification(enable_lazy_loading: bool) {
    let mut executor = executor_with_lazy_loading(enable_lazy_loading);

    let code = "
    import 0x1.Test;

    main() {
        let x: Test.S1;
    label b0:
        x = Test.new_S1();
        return;
    }
    ";

    let status = execute_script_for_test(&mut executor, code, vec![], false);
    assert!(matches!(
        status,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::UNEXPECTED_VERIFIER_ERROR
        )))
    ));
}

#[test_case(true)]
#[test_case(false)]
fn test_type_tag_dependency_fails_verification(enable_lazy_loading: bool) {
    let mut executor = executor_with_lazy_loading(enable_lazy_loading);

    let code = "
    main<T>() {
    label b0:
        return;
    }
    ";

    let ty_args = vec![TypeTag::Struct(Box::new(StructTag {
        address: account_config::CORE_CODE_ADDRESS,
        module: Identifier::new("Test").unwrap(),
        name: Identifier::new("S1").unwrap(),
        type_args: vec![],
    }))];
    let status = execute_script_for_test(&mut executor, code, ty_args, false);
    assert!(matches!(
        status,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::UNEXPECTED_VERIFIER_ERROR
        )))
    ));
}

#[test_case(true)]
#[test_case(false)]
fn test_script_transitive_dependency_fails_verification_bar(enable_lazy_loading: bool) {
    let mut executor = executor_with_lazy_loading(enable_lazy_loading);

    let code = "
    import 0x1.Test2;

    main() {
    label b0:
        // bar does not use bad module, but Test2 does
        Test2.bar();
        return;
    }
    ";

    let status = execute_script_for_test(&mut executor, code, vec![], true);
    success_if_lazy_loading_enabled_or_invariant_violation(enable_lazy_loading, status);
}

#[test_case(true)]
#[test_case(false)]
fn test_script_transitive_dependency_fails_verification_foo(enable_lazy_loading: bool) {
    let mut executor = executor_with_lazy_loading(enable_lazy_loading);

    let code = "
    import 0x1.Test2;

    main() {
    label b0:
        // foo uses bad module
        Test2.foo();
        return;
    }
    ";

    let status = execute_script_for_test(&mut executor, code, vec![], true);
    assert!(matches!(
        status,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::UNEXPECTED_VERIFIER_ERROR
        )))
    ));
}

#[test_case(true)]
#[test_case(false)]
fn test_type_tag_transitive_dependency_fails_verification(enable_lazy_loading: bool) {
    let mut executor = executor_with_lazy_loading(enable_lazy_loading);

    let code = "
    main<T>() {
    label b0:
        return;
    }
    ";

    let ty_args = vec![TypeTag::Struct(Box::new(StructTag {
        address: account_config::CORE_CODE_ADDRESS,
        module: Identifier::new("Test2").unwrap(),
        name: Identifier::new("S").unwrap(),
        type_args: vec![],
    }))];

    // Type tag is using good module, so for lazy loading there should be no verification errors.
    let status = execute_script_for_test(&mut executor, code, ty_args, true);
    success_if_lazy_loading_enabled_or_invariant_violation(enable_lazy_loading, status);
}
