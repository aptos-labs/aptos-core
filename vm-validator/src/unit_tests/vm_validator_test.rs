// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::vm_validator::{get_account_sequence_number, PooledVMValidator, TransactionValidation};
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use aptos_db::AptosDB;
use aptos_gas_schedule::{InitialGasSchedule, TransactionGasParameters};
use aptos_storage_interface::{
    state_store::state_view::db_state_view::LatestDbStateCheckpointView, DbReaderWriter,
};
use aptos_types::{
    account_address, account_config,
    chain_id::ChainId,
    test_helpers::transaction_test_helpers,
    transaction::{Script, TransactionPayload},
    vm_status::StatusCode,
};
use aptos_vm::aptos_vm::AptosVMBlockExecutor;
use move_core_types::{account_address::AccountAddress, gas_algebra::GasQuantity};
use rand::SeedableRng;
use rstest::rstest;

const MAX_TRANSACTION_SIZE_IN_BYTES: u64 = 6 * 1024 * 1024;

struct TestValidator {
    vm_validator: PooledVMValidator,
    _db_path: aptos_temppath::TempPath,
}

impl TestValidator {
    fn new() -> Self {
        let _db_path = aptos_temppath::TempPath::new();
        _db_path.create_as_dir().unwrap();
        let (db, db_rw) = DbReaderWriter::wrap(AptosDB::new_for_test(_db_path.path()));
        aptos_executor_test_helpers::bootstrap_genesis::<AptosVMBlockExecutor>(
            &db_rw,
            &aptos_vm_genesis::test_genesis_transaction(),
        )
        .expect("Db-bootstrapper should not fail.");

        // Create another client for the vm_validator since the one used for the executor will be
        // run on another runtime which will be dropped before this function returns.
        let vm_validator = PooledVMValidator::new(db, 1);
        TestValidator {
            vm_validator,
            _db_path,
        }
    }
}

impl std::ops::Deref for TestValidator {
    type Target = PooledVMValidator;

    fn deref(&self) -> &Self::Target {
        &self.vm_validator
    }
}

// These tests are meant to test all high-level code paths that lead to a validation error in the
// verification of a transaction in the VM. However, there are a couple notable exceptions that we
// do _not_ test here -- this is due to limitations around execution and semantics. The following
// errors are not exercised:
// * SEQUENCE_NUMBER_TOO_OLD -- We can't test sequence number too old here without running execution
//   first in order to bump the account's sequence number. This needs to (and is) tested in the
//   language e2e tests in: aptos-core/language/e2e-testsuite/src/tests/verify_txn.rs ->
//   verify_simple_payment.
// * SEQUENCE_NUMBER_TOO_NEW -- This error is filtered out when running validation; it is only
//   testable when running the executor.
// * INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE -- This is tested in verify_txn.rs.
// * SENDING_ACCOUNT_FROZEN: Tested in functional-tests/tests/aptos_account/freezing.move.
// * Errors arising from deserializing the code -- these are tested in
//   - move-language/move/language/move-binary-format/src/unit_tests/deserializer_tests.rs
//   - move-language/move/language/move-binary-format/tests/serializer_tests.rs
// * Errors arising from calls to `static_verify_program` -- this is tested separately in tests for
//   the bytecode verifier.
// * Testing for invalid genesis write sets -- this is tested in
//   move-language/move/language/e2e-testsuite/src/tests/genesis.rs

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_transaction(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let program = aptos_stdlib::aptos_coin_mint(address, 100);
    let transaction = transaction_test_helpers::get_test_signed_txn(
        address,
        1,
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        Some(program),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(ret.status(), None);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_invalid_signature(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let mut rng = ::rand::rngs::StdRng::from_seed([1u8; 32]);
    let other_private_key = Ed25519PrivateKey::generate(&mut rng);
    // Submit with an account using an different private/public keypair

    let address = account_config::aptos_test_root_address();
    let program = aptos_stdlib::aptos_coin_transfer(address, 100);
    let transaction = transaction_test_helpers::get_test_unchecked_txn(
        address,
        1,
        &other_private_key,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        program,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(ret.status().unwrap(), StatusCode::INVALID_SIGNATURE);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_known_script_too_large_args(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let transaction = transaction_test_helpers::get_test_signed_transaction(
        address,
        1,
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        Some(TransactionPayload::Script(Script::new(
            vec![42; MAX_TRANSACTION_SIZE_IN_BYTES as usize],
            vec![],
            vec![],
        ))),
        /* generate a
         * program with args
         * longer than the
         * max size */
        0,
        0, /* max gas price */
        None,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(
        ret.status().unwrap(),
        StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE
    );
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_max_gas_units_above_max(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let transaction = transaction_test_helpers::get_test_signed_transaction(
        address,
        1,
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        None,
        0,
        0,              /* max gas price */
        Some(u64::MAX), // Max gas units
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(
        ret.status().unwrap(),
        StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND
    );
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_max_gas_units_below_min(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    // Calculate a size for the transaction script that will ensure
    // that the minimum transaction gas is at least 1 after scaling to the
    // external gas units.
    let txn_gas_params = TransactionGasParameters::initial();
    let txn_bytes = txn_gas_params.large_transaction_cutoff
        + GasQuantity::from(
            u64::from(txn_gas_params.gas_unit_scaling_factor)
                / u64::from(txn_gas_params.intrinsic_gas_per_byte),
        );
    let transaction = transaction_test_helpers::get_test_signed_transaction(
        address,
        1,
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        Some(TransactionPayload::Script(Script::new(
            vec![42; u64::from(txn_bytes) as usize],
            vec![],
            vec![],
        ))),
        0,
        0,       /* max gas price */
        Some(0), // Max gas units
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(
        ret.status().unwrap(),
        StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS
    );
}

#[test]
fn test_get_account_sequence_number() {
    let vm_validator = TestValidator::new();
    let root_address = account_config::aptos_test_root_address();
    let state_view = vm_validator
        .vm_validator
        .get_next_vm()
        .lock()
        .unwrap()
        .db_reader
        .latest_state_checkpoint_view()
        .unwrap();
    assert_eq!(
        get_account_sequence_number(&state_view, root_address).unwrap(),
        0
    );
    assert_eq!(
        get_account_sequence_number(
            &state_view,
            AccountAddress::new([5u8; AccountAddress::LENGTH]),
        )
        .unwrap(),
        0
    );
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_max_gas_price_above_bounds(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let transaction = transaction_test_helpers::get_test_signed_transaction(
        address,
        1,
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        None,
        0,
        u64::MAX, /* max gas price */
        None,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(
        ret.status().unwrap(),
        StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND
    );
}

// NB: This test is designed to fail if/when we bump the minimum gas price to be non-zero. You will
// then need to update this price here in order to make the test pass -- uncomment the commented
// out assertion and remove the current failing assertion in this case.
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_max_gas_price_below_bounds(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let program = aptos_stdlib::aptos_coin_transfer(address, 100);
    let transaction = transaction_test_helpers::get_test_signed_transaction(
        address,
        1,
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        Some(program),
        // Initial Time was set to 0 with a TTL 86400 secs.
        40000,
        0, /* max gas price */
        None,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(ret.status(), None);
    //assert_eq!(
    //    ret.status().unwrap().major_status,
    //    StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND
    //);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_invalid_auth_key(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let mut rng = ::rand::rngs::StdRng::from_seed([1u8; 32]);
    let other_private_key = Ed25519PrivateKey::generate(&mut rng);
    // Submit with an account using an different private/public keypair

    let address = account_config::aptos_test_root_address();
    let program = aptos_stdlib::aptos_coin_transfer(address, 100);
    let transaction = transaction_test_helpers::get_test_signed_txn(
        address,
        1,
        &other_private_key,
        other_private_key.public_key(),
        Some(program),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(ret.status().unwrap(), StatusCode::INVALID_AUTH_KEY);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_account_doesnt_exist(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let random_account_addr = account_address::AccountAddress::random();
    let program = aptos_stdlib::aptos_coin_transfer(address, 100);
    let transaction = transaction_test_helpers::get_test_signed_transaction(
        random_account_addr,
        1,
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        Some(program),
        u64::MAX,
        1, /* max gas price */
        None,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let ret = vm_validator.validate_transaction(transaction).unwrap();
    if use_orderless_transactions {
        // The sequence number check is ignored. And the prologue will then fail at the next check.
        assert_eq!(
            ret.status().unwrap(),
            StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE
        );
    } else {
        // To accommodate stateless account, the prologue will now output SEQUENCE_NUMBER_TOO_NEW status code
        // instead of SENDING_ACCOUNT_DOES_NOT_EXIST.
        // When a transaction validation fails with SEQUENCE_NUMBER_TOO_NEW, the validation result is still
        // set to `success` so that mempool will continue to forward the transaction.
        assert_eq!(ret.status(), None);
    }
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_sequence_number_too_new(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let program = aptos_stdlib::aptos_coin_transfer(address, 100);
    let transaction = transaction_test_helpers::get_test_signed_txn(
        address,
        1,
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        Some(program),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(ret.status(), None);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_invalid_arguments(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let program = aptos_stdlib::aptos_coin_transfer(address, 100);
    let transaction = transaction_test_helpers::get_test_signed_txn(
        address,
        1,
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        Some(program),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let _ret = vm_validator.validate_transaction(transaction).unwrap();
    // TODO: Script arguement types are now checked at execution time. Is this an idea behavior?
    // assert_eq!(ret.status().unwrap().major_status, StatusCode::TYPE_MISMATCH);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_expiration_time(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let transaction = transaction_test_helpers::get_test_signed_transaction(
        address,
        1, /* sequence_number */
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        None, /* script */
        0,    /* expiration_time */
        0,    /* gas_unit_price */
        None, /* max_gas_amount */
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(ret.status().unwrap(), StatusCode::TRANSACTION_EXPIRED);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_validate_chain_id(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let vm_validator = TestValidator::new();

    let address = account_config::aptos_test_root_address();
    let transaction = transaction_test_helpers::get_test_txn_with_chain_id(
        address,
        0, /* sequence_number */
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        // all tests use ChainId::test() for chain_id, so pick something different
        ChainId::new(ChainId::test().id() + 1),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let ret = vm_validator.validate_transaction(transaction).unwrap();
    assert_eq!(ret.status().unwrap(), StatusCode::BAD_CHAIN_ID);
}
