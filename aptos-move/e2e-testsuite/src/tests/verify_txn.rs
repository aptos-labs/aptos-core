// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use aptos_gas_algebra::Gas;
use aptos_gas_schedule::{InitialGasSchedule, TransactionGasParameters};
use aptos_language_e2e_tests::{
    assert_prologue_disparity, assert_prologue_parity, common_transactions::EMPTY_SCRIPT,
    current_function_name, executor::FakeExecutor, feature_flags_for_orderless,
    transaction_status_eq,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config,
    chain_id::ChainId,
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
use rstest::rstest;
pub const MAX_TRANSACTION_SIZE_IN_BYTES: u64 = 6 * 1024 * 1024;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn verify_signature(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let sender =
        executor.create_raw_account_data(900_000, if stateless_account { None } else { Some(10) });
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
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    assert_prologue_parity!(
        executor.validate_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        StatusCode::INVALID_SIGNATURE
    );
}

#[rstest(
    sender_stateless_account,
    secondary_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn verify_multi_agent_invalid_sender_signature(
    sender_stateless_account: bool,
    secondary_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());

    let sender = executor.create_raw_account_data(
        1_000_010,
        if sender_stateless_account {
            None
        } else {
            Some(0)
        },
    );
    let secondary_signer = executor.create_raw_account_data(
        100_100,
        if secondary_stateless_account {
            None
        } else {
            Some(100)
        },
    );

    executor.add_account_data(&sender);
    executor.add_account_data(&secondary_signer);

    let private_key = Ed25519PrivateKey::generate_for_testing();

    // Sign using the wrong key for the sender, and correct key for the secondary signer.
    let signed_txn = transaction_test_helpers::get_test_unchecked_multi_agent_txn(
        *sender.address(),
        vec![*secondary_signer.address()],
        0,
        &private_key,
        sender.account().pubkey.as_ed25519().unwrap(),
        vec![&secondary_signer.account().privkey],
        vec![secondary_signer.account().pubkey.as_ed25519().unwrap()],
        None,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_prologue_parity!(
        executor.validate_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        StatusCode::INVALID_SIGNATURE
    );
}

#[rstest(
    sender_stateless_account,
    secondary_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn verify_multi_agent_invalid_secondary_signature(
    sender_stateless_account: bool,
    secondary_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());
    let sender = executor.create_raw_account_data(
        1_000_010,
        if sender_stateless_account {
            None
        } else {
            Some(10)
        },
    );
    let secondary_signer = executor.create_raw_account_data(
        100_100,
        if secondary_stateless_account {
            None
        } else {
            Some(100)
        },
    );

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
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_prologue_parity!(
        executor.validate_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        StatusCode::INVALID_SIGNATURE
    );
}

#[rstest(
    sender_stateless_account,
    secondary_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn verify_multi_agent_duplicate_secondary_signer(
    sender_stateless_account: bool,
    secondary_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());
    let sender = executor.create_raw_account_data(
        1_000_010,
        if sender_stateless_account {
            None
        } else {
            Some(10)
        },
    );
    let secondary_signer = executor.create_raw_account_data(
        100_100,
        if secondary_stateless_account {
            None
        } else {
            Some(100)
        },
    );
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
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_prologue_parity!(
        executor.validate_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        StatusCode::SIGNERS_CONTAIN_DUPLICATES
    );
}

// This test is testing with sender = VM reserved address.
// Making it a stateless account and sending txn with sequence number 0 would trigger creating an Account
// resource for a reserved address which fails. So, not testing for these cases.
#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn verify_reserved_sender(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let sender =
        executor.create_raw_account_data(900_000, if stateless_account { None } else { Some(10) });
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
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    if use_orderless_transactions {
        // Orderless transactions don't check if the Account resource exists.
        // The authentication fails for these transactions.
        assert_prologue_parity!(
            executor.validate_transaction(signed_txn.clone()).status(),
            executor.execute_transaction(signed_txn).status(),
            StatusCode::INVALID_AUTH_KEY
        );
    } else {
        assert_prologue_parity!(
            executor.validate_transaction(signed_txn.clone()).status(),
            executor.execute_transaction(signed_txn).status(),
            StatusCode::INVALID_AUTH_KEY
        );
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn verify_simple_payment_1(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    // create and publish a sender with 1_000_000 coins and a receiver with 100_000 coins
    let sender =
        executor.create_raw_account_data(900_000, if stateless_account { None } else { Some(0) });
    let receiver = executor.create_raw_account_data(100_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    // Create a new transaction that has the exact right sequence number.
    let txn = sender
        .account()
        .transaction()
        .payload(aptos_stdlib::aptos_coin_transfer(
            *receiver.address(),
            1_000,
        ))
        .sequence_number(0)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    assert_eq!(executor.validate_transaction(txn).status(), None);
}

#[rstest(
    sender_stateless_account,
    receiver_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn verify_simple_payment_2(
    sender_stateless_account: bool,
    receiver_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    // create and publish a sender with 1_000_000 coins and a receiver with 100_000 coins
    let sender = executor.create_raw_account_data(
        900_000,
        if sender_stateless_account {
            None
        } else {
            Some(0)
        },
    );
    let receiver = executor.create_raw_account_data(
        100_000,
        if receiver_stateless_account {
            None
        } else {
            Some(10)
        },
    );
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    // Create a new transaction that has the bad auth key.
    println!("txn2");
    let empty_script = &*EMPTY_SCRIPT;
    let txn = receiver
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(0)
        .max_gas_amount(105_000)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
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
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn verify_simple_payment_3(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    // create and publish a sender with 1_000_000 coins and a receiver with 100_000 coins
    let sender =
        executor.create_raw_account_data(900_000, if stateless_account { None } else { Some(10) });
    let receiver = executor.create_raw_account_data(100_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    // Create a new transaction that has a too new sequence number.
    let empty_script = &*EMPTY_SCRIPT;
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(1)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    if stateless_account && !use_orderless_transactions {
        assert_prologue_disparity!(
            executor.validate_transaction(txn.clone()).status() => None,
            executor.execute_transaction(txn).status() =>
            TransactionStatus::Discard(StatusCode::SEQUENCE_NUMBER_TOO_NEW)
        );
    } else if !use_orderless_transactions {
        assert_prologue_parity!(
            executor.validate_transaction(txn.clone()).status(),
            executor.execute_transaction(txn).status(),
            StatusCode::SEQUENCE_NUMBER_TOO_OLD
        );
    } else {
        assert_eq!(executor.validate_transaction(txn).status(), None);
    }
}

#[rstest(
    sender_stateless_account,
    receiver_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn verify_simple_payment_4(
    sender_stateless_account: bool,
    receiver_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    // create and publish a sender with 1_000_000 coins and a receiver with 100_000 coins
    let sender = executor.create_raw_account_data(
        900_000,
        if sender_stateless_account {
            None
        } else {
            Some(0)
        },
    );
    let receiver = executor.create_raw_account_data(
        100_000,
        if receiver_stateless_account {
            None
        } else {
            Some(10)
        },
    );
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    // Create a new transaction that has a too new sequence number.
    let empty_script = &*EMPTY_SCRIPT;
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(11)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    if !use_orderless_transactions {
        assert_prologue_disparity!(
            executor.validate_transaction(txn.clone()).status() => None,
            executor.execute_transaction(txn).status() =>
            TransactionStatus::Discard(StatusCode::SEQUENCE_NUMBER_TOO_NEW)
        );
    } else {
        assert_eq!(executor.validate_transaction(txn).status(), None);
    }

    // Create a new transaction that doesn't have enough balance to pay for gas.
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(0)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE
    );

    // Create a new transaction from a bogus stateful account that doesn't exist
    let bogus_stateful_account = executor.create_raw_account_data(100_000, Some(0));
    let txn = bogus_stateful_account
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(0)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    // The sender doesn't have account balance to pay for Account resource creation.
    assert_eq!(
        executor.validate_transaction(txn).status(),
        Some(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE)
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
        .sequence_number(0)
        .gas_unit_price((txn_gas_params.max_price_per_gas_unit + GasQuantity::one()).into())
        .max_gas_amount(1_000_000)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
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
        .sequence_number(0)
        .max_gas_amount(gas_limit.into())
        .gas_unit_price(txn_gas_params.max_price_per_gas_unit.into())
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
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
        .sequence_number(0)
        .max_gas_amount((txn_gas_params.maximum_number_of_gas_units + GasQuantity::one()).into())
        .gas_unit_price((txn_gas_params.max_price_per_gas_unit).into())
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
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
        .sequence_number(0)
        .max_gas_amount((txn_gas_params.maximum_number_of_gas_units + GasQuantity::one()).into())
        .gas_unit_price((txn_gas_params.max_price_per_gas_unit).into())
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
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
        .sequence_number(0)
        .max_gas_amount(105_000)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH
        )))
    );
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(false, false, false),
    case(false, true, false)
)]
fn verify_simple_payment_5(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    // create and publish a sender with 1_000_000 coins and a receiver with 100_000 coins
    let sender =
        executor.create_raw_account_data(900_000, if stateless_account { None } else { Some(0) });
    let receiver = executor.create_raw_account_data(100_000, Some(0));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let empty_script = &*EMPTY_SCRIPT;

    // Create a new transaction that has a old sequence number.
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(empty_script.clone(), vec![], vec![]))
        .sequence_number(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    assert_prologue_disparity!(
        executor.validate_transaction(txn.clone()).status() => None,
        executor.execute_transaction(txn).status() => TransactionStatus::Discard(StatusCode::SEQUENCE_NUMBER_TOO_NEW)
    );
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
pub fn test_arbitrary_script_execution(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    // create a FakeExecutor with a genesis from file
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());

    // create an empty transaction
    let sender =
        executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);

    // If CustomScripts is on, result should be Keep(DeserializationError). If it's off, the
    // result should be Keep(UnknownScript)
    let random_script = vec![];
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(random_script, vec![], vec![]))
        .sequence_number(0)
        .max_gas_amount(105_000)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
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

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn verify_expiration_time(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let sender =
        executor.create_raw_account_data(900_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);
    let private_key = &sender.account().privkey;
    let txn = transaction_test_helpers::get_test_signed_transaction(
        *sender.address(),
        0, /* sequence_number */
        private_key,
        private_key.public_key(),
        None, /* script */
        0,    /* expiration_time */
        1,    /* gas_unit_price */
        None, /* max_gas_amount */
        use_txn_payload_v2_format,
        use_orderless_transactions,
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
        1,    /* gas_unit_price */
        None, /* max_gas_amount */
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_prologue_parity!(
        executor.validate_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        StatusCode::TRANSACTION_EXPIRED
    );
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn verify_chain_id(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let sender =
        executor.create_raw_account_data(900_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let txn = transaction_test_helpers::get_test_txn_with_chain_id(
        *sender.address(),
        0,
        &private_key,
        private_key.public_key(),
        // all tests use ChainId::test() for chain_id,so pick something different
        ChainId::new(ChainId::test().id() + 1),
        use_txn_payload_v2_format,
        use_orderless_transactions,
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
    let sender = executor.create_raw_account_data(900_000, Some(std::u64::MAX));
    executor.add_account_data(&sender);
    let private_key = &sender.account().privkey;
    let txn = transaction_test_helpers::get_test_signed_transaction(
        *sender.address(),
        std::u64::MAX, /* sequence_number */
        private_key,
        private_key.public_key(),
        None,     /* script */
        u64::MAX, /* expiration_time */
        1,        /* gas_unit_price */
        None,     /* max_gas_amount */
        false,
        false,
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
    bad_dep: CompiledModule,
) -> (CompiledModule, Vec<u8>) {
    let good_module_code = format!(
        "
    module 0x{}.Test2 {{
        import 0x1.Test;
        struct S {{ b: bool }}

        foo(): Test.S1 {{
        label b0:
            return Test.new_S1();
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
            .chain(std::iter::once(&bad_dep))
            .collect(),
    };
    let module = compiler
        .into_compiled_module(good_module_code.as_str())
        .expect("Failed to compile");
    let mut bytes = vec![];
    module.serialize(&mut bytes).unwrap();
    (module, bytes)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_script_dependency_fails_verification(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());

    // Get a module that fails verification into the store.
    let (module, bytes) = bad_module();
    executor.add_module(&module.self_id(), bytes);

    // Create a module that tries to use that module.
    let sender =
        executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);

    let code = "
    import 0x1.Test;

    main() {
        let x: Test.S1;
    label b0:
        x = Test.new_S1();
        return;
    }
    ";

    let compiler = Compiler {
        deps: vec![&module],
    };
    let script = compiler.into_script_blob(code).expect("Failed to compile");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(script, vec![], vec![]))
        .sequence_number(0)
        .max_gas_amount(105_000)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    // As of now, we verify module/script dependencies. This will result in an
    // invariant violation as we try to load `Test`
    assert_eq!(executor.validate_transaction(txn.clone()).status(), None);
    match executor.execute_transaction(txn).status() {
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(status)) => {
            assert_eq!(status, &Some(StatusCode::UNEXPECTED_VERIFIER_ERROR));
        },
        _ => panic!("Kept transaction with an invariant violation!"),
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_type_tag_dependency_fails_verification(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());

    // Get a module that fails verification into the store.
    let (module, bytes) = bad_module();
    executor.add_module(&module.self_id(), bytes);

    // Create a transaction that tries to use that module.
    let sender =
        executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);

    let code = "
    main<T>() {
    label b0:
        return;
    }
    ";

    let compiler = Compiler {
        deps: vec![&module],
    };
    let script = compiler.into_script_blob(code).expect("Failed to compile");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(
            script,
            vec![TypeTag::Struct(Box::new(StructTag {
                address: account_config::CORE_CODE_ADDRESS,
                module: Identifier::new("Test").unwrap(),
                name: Identifier::new("S1").unwrap(),
                type_args: vec![],
            }))],
            vec![],
        ))
        .sequence_number(0)
        .max_gas_amount(105_000)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    // As of now, we verify module/script dependencies. This will result in an
    // invariant violation as we try to load `Test`
    assert_eq!(executor.validate_transaction(txn.clone()).status(), None);
    match executor.execute_transaction(txn).status() {
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(status)) => {
            assert_eq!(status, &Some(StatusCode::UNEXPECTED_VERIFIER_ERROR));
        },
        _ => panic!("Kept transaction with an invariant violation!"),
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_script_transitive_dependency_fails_verification(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());

    // Get a module that fails verification into the store.
    let (bad_module, bad_module_bytes) = bad_module();
    executor.add_module(&bad_module.self_id(), bad_module_bytes);

    // Create a module that tries to use that module.
    let (good_module, good_module_bytes) =
        good_module_uses_bad(account_config::CORE_CODE_ADDRESS, bad_module);
    executor.add_module(&good_module.self_id(), good_module_bytes);

    // Create a transaction that tries to use that module.
    let sender =
        executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);

    let code = "
    import 0x1.Test2;

    main() {
    label b0:
        Test2.bar();
        return;
    }
    ";

    let compiler = Compiler {
        deps: vec![&good_module],
    };
    let script = compiler.into_script_blob(code).expect("Failed to compile");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(script, vec![], vec![]))
        .sequence_number(0)
        .max_gas_amount(105_000)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    // As of now, we verify module/script dependencies. This will result in an
    // invariant violation as we try to load `Test`
    assert_eq!(executor.validate_transaction(txn.clone()).status(), None);
    match executor.execute_transaction(txn).status() {
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(status)) => {
            assert_eq!(status, &Some(StatusCode::UNEXPECTED_VERIFIER_ERROR));
        },
        _ => panic!("Kept transaction with an invariant violation!"),
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_type_tag_transitive_dependency_fails_verification(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());

    // Get a module that fails verification into the store.
    let (bad_module, bad_module_bytes) = bad_module();
    executor.add_module(&bad_module.self_id(), bad_module_bytes);

    // Create a module that tries to use that module.
    let (good_module, good_module_bytes) =
        good_module_uses_bad(account_config::CORE_CODE_ADDRESS, bad_module);
    executor.add_module(&good_module.self_id(), good_module_bytes);

    // Create a transaction that tries to use that module.
    let sender =
        executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);

    let code = "
    main<T>() {
    label b0:
        return;
    }
    ";

    let compiler = Compiler {
        deps: vec![&good_module],
    };
    let script = compiler.into_script_blob(code).expect("Failed to compile");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(
            script,
            vec![TypeTag::Struct(Box::new(StructTag {
                address: account_config::CORE_CODE_ADDRESS,
                module: Identifier::new("Test2").unwrap(),
                name: Identifier::new("S").unwrap(),
                type_args: vec![],
            }))],
            vec![],
        ))
        .sequence_number(0)
        .max_gas_amount(105_000)
        .gas_unit_price(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    // As of now, we verify module/script dependencies. This will result in an
    // invariant violation as we try to load `Test`
    assert_eq!(executor.validate_transaction(txn.clone()).status(), None);
    match executor.execute_transaction(txn).status() {
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(status)) => {
            assert_eq!(status, &Some(StatusCode::UNEXPECTED_VERIFIER_ERROR));
        },
        _ => panic!("Kept transaction with an invariant violation!"),
    }
}
