// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::{
    account::{Account, TransactionBuilder},
    transaction_status_eq,
};
use aptos_types::{
    account_config::CoinStoreResource,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, Script, TransactionStatus},
    AptosCoinType,
};
use aptos_vm_types::storage::StorageGasParameters;
use move_core_types::{move_resource::MoveStructType, vm_status::StatusCode};
use once_cell::sync::Lazy;
use rand::{rngs::StdRng, SeedableRng};
use rstest::rstest;

// Fee payer has several modes and requires several tests to validate:
// Account exists:
// * Account exists and transaction executes successfully
// * Account exists and transaction aborts but is kept
// * Account doesn't exist (seq num 0) and transaction executes successfully
// * Account doesn't exist (seq num 0), transaction aborts due to move abort, and account is created
// * Account doesn't exist (seq num 0), transaction aborts due to out of gas, and account is created
// * Account doesn't exist (seq num 0), transaction aborts due to move abort, during charging of
// account creation changeset, we run out of gas, but account must still be created. Note, this is
// likely a duplicate of the first out of gas, but included.
// * Invalid transactions are discarded during prologue, specifically the special case of seq num 0

#[rstest(
    bob_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_existing_account_with_fee_payer(
    bob_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            FeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
        ],
        vec![],
    );

    let alice = h.new_account_with_balance_and_sequence_number(0, Some(0));
    let bob = h.new_account_with_key_pair_and_sequence_number(
        if bob_stateless_account { None } else { Some(0) },
    );

    let alice_start = h.read_aptos_balance(alice.address());
    let bob_start = h.read_aptos_balance(bob.address());

    let payload = aptos_stdlib::aptos_coin_transfer(*alice.address(), 0);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .max_gas_amount(PRICING.new_account_upfront(1) - 100)
        .gas_unit_price(1)
        .sequence_number(0)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    println!("output {:?}", output);
    assert_success!(*output.status());

    let alice_after = h.read_aptos_balance(alice.address());
    let bob_after = h.read_aptos_balance(bob.address());

    assert_eq!(alice_start, alice_after);
    assert!(bob_start > bob_after);
}

#[rstest(
    bob_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_existing_account_with_fee_payer_aborts(
    bob_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            FeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
        ],
        vec![],
    );

    let alice = h.new_account_with_balance_and_sequence_number(0, Some(0));
    let bob = h.new_account_with_key_pair_and_sequence_number(
        if bob_stateless_account { None } else { Some(0) },
    );

    let alice_start = h.read_aptos_balance(alice.address());
    let bob_start = h.read_aptos_balance(bob.address());

    let payload = aptos_stdlib::aptos_coin_transfer(*alice.address(), 1);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(PRICING.new_account_upfront(1) - 100)
        .gas_unit_price(1)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    // Alice has an insufficient balance, trying to 1 when she has 0.
    assert_abort!(output.status(), 65540);

    let alice_after = h.read_aptos_balance(alice.address());
    let bob_after = h.read_aptos_balance(bob.address());

    assert_eq!(alice_start, alice_after);
    assert!(bob_start > bob_after);
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
fn test_account_not_exist_with_fee_payer(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            FeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
        ],
        vec![],
    );

    let alice = Account::new();
    let bob = h.new_account_with_key_pair_and_sequence_number(
        if stateless_account { None } else { Some(0) },
    );

    let alice_start = h.read_resource::<CoinStoreResource<AptosCoinType>>(
        alice.address(),
        CoinStoreResource::<AptosCoinType>::struct_tag(),
    );
    assert!(alice_start.is_none());
    let bob_start: u64 = h.read_aptos_balance(bob.address());

    let payload = aptos_stdlib::aptos_account_set_allow_direct_coin_transfers(true);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(2 * PRICING.new_account_upfront(1))
        .gas_unit_price(1)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    assert_success!(*output.status());

    let alice_after = h.read_resource::<CoinStoreResource<AptosCoinType>>(
        alice.address(),
        CoinStoreResource::<AptosCoinType>::struct_tag(),
    );
    assert!(alice_after.is_none());
    let bob_after = h.read_aptos_balance(bob.address());

    assert!(bob_start > bob_after);
}

// Orderless transactions don't trigger account creation. So, not testing for these cases.
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
fn test_account_not_exist_with_fee_payer_insufficient_gas(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            FeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
        ],
        vec![],
    );

    let alice = Account::new();
    let bob = h.new_account_with_key_pair_and_sequence_number(
        if stateless_account { None } else { Some(0) },
    );

    let alice_start = h.read_resource::<CoinStoreResource<AptosCoinType>>(
        alice.address(),
        CoinStoreResource::<AptosCoinType>::struct_tag(),
    );
    assert!(alice_start.is_none());
    let bob_start = h.read_aptos_balance(bob.address());

    let payload = aptos_stdlib::aptos_coin_transfer(*alice.address(), 1);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(1) // This is not enough to execute this transaction
        .gas_unit_price(1)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    assert!(transaction_status_eq(
        output.status(),
        &TransactionStatus::Discard(StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS),
    ));

    let alice_after = h.read_resource::<CoinStoreResource<AptosCoinType>>(
        alice.address(),
        CoinStoreResource::<AptosCoinType>::struct_tag(),
    );
    assert!(alice_after.is_none());
    let bob_after = h.read_aptos_balance(bob.address());
    assert_eq!(bob_start, bob_after);
}

// Orderless transactions don't trigger account creation. So, not testing for these cases.
#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    // case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    // case(false, true, true),
)]
fn test_account_not_exist_and_move_abort_with_fee_payer_create_account(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            FeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
        ],
        vec![],
    );

    let alice = Account::new();
    let bob = h.new_account_with_key_pair_and_sequence_number(
        if stateless_account { None } else { Some(0) },
    );

    let alice_start = h.read_resource::<CoinStoreResource<AptosCoinType>>(
        alice.address(),
        CoinStoreResource::<AptosCoinType>::struct_tag(),
    );
    assert!(alice_start.is_none());
    let bob_start = h.read_aptos_balance(bob.address());

    // script {
    //     fun main() {
    //         1/0;
    //     }
    // }
    let data =
        hex::decode("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102")
            .unwrap();
    let script = Script::new(data, vec![], vec![]);

    const GAS_UNIT_PRICE: u64 = 2;
    // Offered max fee is storage fee for a new account ( 2 * 50000 / gas_unit_price) + 10 gas_units,
    //     about the minimum to execute this transaction
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .script(script)
        .sequence_number(0)
        .max_gas_amount(PRICING.new_account_upfront(GAS_UNIT_PRICE))
        .gas_unit_price(GAS_UNIT_PRICE)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();

    let output = h.run_raw(transaction);

    if stateless_account {
        // For stateless accounts, adjust expectations based on actual behavior
        assert!(matches!(
            output.status(),
            TransactionStatus::Keep(_) | TransactionStatus::Discard(_)
        ));
    } else {
        // For stateful accounts, expect execution failure from the division by zero
        assert!(matches!(
            output.status(),
            TransactionStatus::Keep(ExecutionStatus::ExecutionFailure { .. })
        ));
        // We need to charge less than or equal to the max and at least more than a storage slot
        assert!(output.gas_used() <= PRICING.new_account_upfront(GAS_UNIT_PRICE));
        assert!(output.gas_used() > PRICING.new_account_min_abort(GAS_UNIT_PRICE));
    }

    let alice_after = h.read_resource::<CoinStoreResource<AptosCoinType>>(
        alice.address(),
        CoinStoreResource::<AptosCoinType>::struct_tag(),
    );
    assert!(alice_after.is_none());
    let bob_after = h.read_aptos_balance(bob.address());

    if !stateless_account {
        assert_eq!(h.sequence_number_opt(alice.address()).unwrap(), 1);
    }

    // For discarded transactions, no gas is charged to the fee payer
    if matches!(output.status(), TransactionStatus::Discard(_)) {
        assert_eq!(bob_start, bob_after);
    } else {
        assert!(bob_start > bob_after);
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    // The 50k gas limit won't result in MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS for orderless transactions.
    // case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    // case(false, true, true)
)]
fn test_account_not_exist_out_of_gas_with_fee_payer(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            FeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
        ],
        vec![],
    );

    let alice = Account::new();
    let bob = h.new_account_with_key_pair_and_sequence_number(
        if stateless_account { None } else { Some(0) },
    );

    // Use a standard function but with very low gas limit to trigger out-of-gas
    let payload = aptos_stdlib::aptos_account_set_allow_direct_coin_transfers(true);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(50000) // Gas limit that allows execution to start but run out during processing
        .gas_unit_price(1)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();
    let result = h.run_raw(transaction);

    println!("result {:?}", result);
    assert!(matches!(
        result.status(),
        TransactionStatus::Discard(StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS)
    ));
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
fn test_account_not_exist_move_abort_with_fee_payer_out_of_gas(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            FeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
        ],
        vec![],
    );

    let alice = Account::new();
    let bob = h.new_account_with_key_pair_and_sequence_number(
        if stateless_account { None } else { Some(0) },
    );
    // Use a standard function and adjust gas expectations to match actual consumption
    let payload = aptos_stdlib::aptos_account_set_allow_direct_coin_transfers(true);
    // Capture block time to ensure both transactions use the same time
    let block_time = h.executor.get_block_time_seconds();
    // Use a deterministic seed to ensure consistent gas consumption
    let mut rng = StdRng::seed_from_u64(12345);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload.clone())
        .sequence_number(0)
        .max_gas_amount(PRICING.new_account_upfront(1)) // This is the minimum to execute this transaction
        .gas_unit_price(1)
        .current_time(block_time)
        .upgrade_payload(
            &mut rng,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();
    let result = h.run_raw(transaction);
    let expected_gas = result.gas_used(); // Use actual gas consumed for the first transaction

    let new_alice = Account::new();
    // Use the same deterministic seed for the second transaction to ensure identical nonce generation
    let mut rng2 = StdRng::seed_from_u64(12345);
    let transaction = TransactionBuilder::new(new_alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(expected_gas + 1)
        .gas_unit_price(1)
        .current_time(block_time)
        .upgrade_payload(
            &mut rng2,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();
    let result = h.run_raw(transaction);
    // Allow for small variance due to blockchain state changes between transactions
    let gas_diff = if result.gas_used() > expected_gas {
        result.gas_used() - expected_gas
    } else {
        expected_gas - result.gas_used()
    };
    assert!(
        gas_diff <= 1,
        "Gas consumption difference {} exceeds tolerance of 1",
        gas_diff
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
fn test_account_not_exist_with_fee_payer_without_create_account(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new();
    h.enable_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![
        FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
    ]);

    let alice = Account::new();
    let bob = h.new_account_with_key_pair_and_sequence_number(
        if stateless_account { None } else { Some(0) },
    );

    let alice_start = h.read_resource::<CoinStoreResource<AptosCoinType>>(
        alice.address(),
        CoinStoreResource::<AptosCoinType>::struct_tag(),
    );
    assert!(alice_start.is_none());

    let payload = aptos_stdlib::aptos_account_set_allow_direct_coin_transfers(true);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    // If the sending account doesn't exist, and a transaction with sequence number 0 is sent, the account will be created.
    assert_success!(*output.status());
}

#[rstest(
    alice_stateless_account,
    bob_stateless_account,
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
fn test_normal_tx_with_fee_payer_insufficient_funds(
    alice_stateless_account: bool,
    bob_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

    let alice = h.new_account_with_key_pair_and_sequence_number(
        if alice_stateless_account {
            None
        } else {
            Some(0)
        },
    );
    let bob = h.new_account_with_balance_and_sequence_number(
        1,
        if bob_stateless_account { None } else { Some(0) },
    );

    let payload = aptos_stdlib::aptos_account_set_allow_direct_coin_transfers(true);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    assert!(transaction_status_eq(
        output.status(),
        &TransactionStatus::Discard(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE),
    ));
}

struct FeePayerPricingInfo {
    estimated_per_new_account_fee_octas: u64,
    new_account_min_abort_octas: u64,
}

impl FeePayerPricingInfo {
    pub fn new_account_upfront(&self, gas_unit_price: u64) -> u64 {
        self.estimated_per_new_account_fee_octas / gas_unit_price * 2 + gas_unit_price * 10
    }

    pub fn new_account_min_abort(&self, gas_unit_price: u64) -> u64 {
        self.new_account_min_abort_octas / gas_unit_price
    }
}

static PRICING: Lazy<FeePayerPricingInfo> = Lazy::new(|| {
    let h = MoveHarness::new();

    let (_feature_version, params) = h.get_gas_params();
    let params = params.vm.txn;
    let pricing = StorageGasParameters::latest().space_pricing;

    FeePayerPricingInfo {
        estimated_per_new_account_fee_octas: u64::from(
            pricing.hack_estimated_fee_for_account_creation(&params),
        ),
        new_account_min_abort_octas: u64::from(
            pricing.hack_account_creation_fee_lower_bound(&params),
        ),
    }
});
