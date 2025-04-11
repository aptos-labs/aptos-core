// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::{
    account::{Account, TransactionBuilder},
    feature_flags_for_orderless,
};
use aptos_types::{
    move_utils::MemberId,
    on_chain_config::FeatureFlag,
    transaction::{EntryFunction, MultisigTransactionPayload, TransactionPayload},
};
use bcs::to_bytes;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::{ModuleId, StructTag, TypeTag, CORE_CODE_ADDRESS},
    parser::parse_struct_tag,
};
use rstest::rstest;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
struct TransactionContextStore {
    sender: AccountAddress,
    secondary_signers: Vec<AccountAddress>,
    gas_payer: AccountAddress,
    max_gas_amount: u64,
    gas_unit_price: u64,
    chain_id: u8,
    account_address: AccountAddress,
    module_name: String,
    function_name: String,
    type_arg_names: Vec<String>,
    args: Vec<Vec<u8>>,
    multisig_address: AccountAddress,
}

fn setup(harness: &mut MoveHarness, stateless_account: bool) -> Account {
    let path = common::test_dir_path("transaction_context.data/pack");

    let account = harness.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("admin".to_string(), *account.address());
    assert_success!(harness.publish_package_with_options(&account, &path, build_options));

    account
}

fn call_get_sender_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> AccountAddress {
    let status = harness.run_entry_function(
        account,
        str::parse(&format!(
            "{}::transaction_context_test::store_sender_from_native_txn_context",
            account.address()
        ))
        .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                account.address()
            ))
            .unwrap(),
        )
        .unwrap();

    txn_ctx_store.sender
}

fn call_get_secondary_signers_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> Vec<AccountAddress> {
    let status = harness.run_entry_function(
        account,
        str::parse(&format!(
            "{}::transaction_context_test::store_secondary_signers_from_native_txn_context",
            account.address()
        ))
        .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                account.address()
            ))
            .unwrap(),
        )
        .unwrap();

    txn_ctx_store.secondary_signers
}

fn call_get_gas_payer_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> AccountAddress {
    let status = harness.run_entry_function(
        account,
        str::parse(&format!(
            "{}::transaction_context_test::store_gas_payer_from_native_txn_context",
            account.address()
        ))
        .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                account.address()
            ))
            .unwrap(),
        )
        .unwrap();

    txn_ctx_store.gas_payer
}

fn call_get_max_gas_amount_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> u64 {
    let status = harness.run_entry_function(
        account,
        str::parse(&format!(
            "{}::transaction_context_test::store_max_gas_amount_from_native_txn_context",
            account.address()
        ))
        .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                account.address()
            ))
            .unwrap(),
        )
        .unwrap();

    txn_ctx_store.max_gas_amount
}

fn call_get_gas_unit_price_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> u64 {
    let status = harness.run_entry_function(
        account,
        str::parse(&format!(
            "{}::transaction_context_test::store_gas_unit_price_from_native_txn_context",
            account.address()
        ))
        .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                account.address()
            ))
            .unwrap(),
        )
        .unwrap();

    txn_ctx_store.gas_unit_price
}

fn call_get_chain_id_from_native_txn_context(harness: &mut MoveHarness, account: &Account) -> u8 {
    let status = harness.run_entry_function(
        account,
        str::parse(&format!(
            "{}::transaction_context_test::store_chain_id_from_native_txn_context",
            account.address()
        ))
        .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                account.address()
            ))
            .unwrap(),
        )
        .unwrap();

    txn_ctx_store.chain_id
}

fn call_get_entry_function_payload_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> (AccountAddress, String, String, Vec<String>, Vec<Vec<u8>>) {
    let status = harness.run_entry_function(
        account,
        str::parse(&format!(
            "{}::transaction_context_test::store_entry_function_payload_from_native_txn_context",
            account.address()
        ))
        .unwrap(),
        vec![
            TypeTag::U64,
            TypeTag::Vector(Box::new(TypeTag::Address)),
            TypeTag::Struct(Box::new(StructTag {
                address: AccountAddress::from_hex_literal("0x1").unwrap(),
                module: ident_str!("transaction_fee").to_owned(),
                name: ident_str!("FeeStatement").to_owned(),
                type_args: vec![],
            })),
        ],
        vec![
            bcs::to_bytes(&7777777u64).unwrap(),
            bcs::to_bytes(&true).unwrap(),
        ],
    );
    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                account.address()
            ))
            .unwrap(),
        )
        .unwrap();

    (
        txn_ctx_store.account_address,
        txn_ctx_store.module_name,
        txn_ctx_store.function_name,
        txn_ctx_store.type_arg_names,
        txn_ctx_store.args,
    )
}

fn new_move_harness() -> MoveHarness {
    MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            FeatureFlag::TRANSACTION_CONTEXT_EXTENSION,
        ],
        vec![],
    )
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
fn test_transaction_context_sender(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let account = setup(&mut harness, stateless_account);

    let addr = call_get_sender_from_native_txn_context(&mut harness, &account);
    assert_eq!(addr, *account.address());
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
fn test_transaction_context_max_gas_amount(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let account = setup(&mut harness, stateless_account);

    let max_gas_amount = call_get_max_gas_amount_from_native_txn_context(&mut harness, &account);
    assert_eq!(max_gas_amount, 2000000);
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
fn test_transaction_context_gas_unit_price(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let account = setup(&mut harness, stateless_account);

    let max_gas_amount = call_get_gas_unit_price_from_native_txn_context(&mut harness, &account);
    assert_eq!(max_gas_amount, 100);
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
fn test_transaction_context_chain_id(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let account = setup(&mut harness, stateless_account);

    let chain_id = call_get_chain_id_from_native_txn_context(&mut harness, &account);
    assert_eq!(chain_id, 4);
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
fn test_transaction_context_gas_payer_as_sender(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let account = setup(&mut harness, stateless_account);

    let gas_payer = call_get_gas_payer_from_native_txn_context(&mut harness, &account);
    assert_eq!(gas_payer, *account.address());
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
fn test_transaction_context_secondary_signers_empty(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let account = setup(&mut harness, stateless_account);

    let secondary_signers =
        call_get_secondary_signers_from_native_txn_context(&mut harness, &account);
    assert_eq!(secondary_signers, vec![]);
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
fn test_transaction_context_gas_payer_as_separate_account(
    alice_stateless_account: bool,
    bob_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );

    let alice = setup(&mut harness, alice_stateless_account);
    let bob = harness.new_account_with_balance_and_sequence_number(
        1000000,
        if bob_stateless_account { None } else { Some(0) },
    );

    let fun: MemberId = str::parse(&format!(
        "{}::transaction_context_test::store_gas_payer_from_native_txn_context",
        alice.address()
    ))
    .unwrap();
    let MemberId {
        module_id,
        member_id: function_id,
    } = fun;
    let ty_args = vec![];
    let args = vec![];
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        function_id,
        ty_args,
        args,
    ));
    let mut transaction_builder = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .current_time(harness.executor.get_block_time_seconds())
        .upgrade_payload(
            harness.use_txn_payload_v2_format,
            harness.use_orderless_transactions,
        );
    if !harness.use_orderless_transactions {
        transaction_builder = transaction_builder
            .sequence_number(harness.sequence_number_opt(alice.address()).unwrap());
    }
    let transaction = transaction_builder.sign_fee_payer();

    let output = harness.run_raw(transaction);
    assert_success!(*output.status());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            alice.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                alice.address()
            ))
            .unwrap(),
        )
        .unwrap();

    let gas_payer = txn_ctx_store.gas_payer;
    assert_eq!(gas_payer, *bob.address());
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
fn test_transaction_context_secondary_signers(
    alice_stateless_account: bool,
    bob_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );

    let alice = setup(&mut harness, alice_stateless_account);
    let bob = harness.new_account_with_balance_and_sequence_number(
        1000000,
        if bob_stateless_account { None } else { Some(0) },
    );

    let fun: MemberId = str::parse(&format!(
        "{}::transaction_context_test::store_secondary_signers_from_native_txn_context_multi",
        alice.address()
    ))
    .unwrap();
    let MemberId {
        module_id,
        member_id: function_id,
    } = fun;
    let ty_args = vec![];
    let args = vec![];
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        function_id,
        ty_args,
        args,
    ));
    let mut transaction_builder = TransactionBuilder::new(alice.clone())
        .secondary_signers(vec![bob.clone()])
        .payload(payload)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .current_time(harness.executor.get_block_time_seconds())
        .upgrade_payload(
            harness.use_txn_payload_v2_format,
            harness.use_orderless_transactions,
        );
    if !harness.use_orderless_transactions {
        transaction_builder = transaction_builder
            .sequence_number(harness.sequence_number_opt(alice.address()).unwrap());
    }
    let transaction = transaction_builder.sign_multi_agent();

    let output = harness.run_raw(transaction);
    assert_success!(*output.status());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            alice.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                alice.address()
            ))
            .unwrap(),
        )
        .unwrap();

    let secondary_signers = txn_ctx_store.secondary_signers;
    assert_eq!(secondary_signers, vec![*bob.address()]);
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
fn test_transaction_context_entry_function_payload(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let account = setup(&mut harness, stateless_account);

    let (account_address, module_name, function_name, type_arg_names, args) =
        call_get_entry_function_payload_from_native_txn_context(&mut harness, &account);

    assert_eq!(account_address, *account.address());
    assert_eq!(module_name, "transaction_context_test");
    assert_eq!(
        function_name,
        "store_entry_function_payload_from_native_txn_context"
    );
    assert_eq!(type_arg_names, vec![
        "u64",
        "vector<address>",
        "0x1::transaction_fee::FeeStatement"
    ]);
    assert_eq!(args, vec![
        bcs::to_bytes(&7777777u64).unwrap(),
        bcs::to_bytes(&true).unwrap()
    ]);
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
fn test_transaction_context_multisig_payload(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness = new_move_harness();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let account = setup(&mut harness, stateless_account);

    let multisig_transaction_payload =
        MultisigTransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                CORE_CODE_ADDRESS,
                ident_str!("transaction_context_test").to_owned(),
            ),
            ident_str!("store_multisig_payload_from_native_txn_context").to_owned(),
            vec![],
            vec![],
        ));

    let serialized_multisig_transaction_payload =
        bcs::to_bytes(&multisig_transaction_payload).unwrap();

    let status = harness.run_entry_function(
        &account,
        str::parse(&format!(
            "{}::transaction_context_test::prepare_multisig_payload_test",
            account.address()
        ))
        .unwrap(),
        vec![],
        vec![to_bytes(&serialized_multisig_transaction_payload).unwrap()],
    );
    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                account.address()
            ))
            .unwrap(),
        )
        .unwrap();

    let multisig_address = txn_ctx_store.multisig_address;

    let status = harness.run_multisig(
        &account,
        txn_ctx_store.multisig_address,
        Some(multisig_transaction_payload),
    );
    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag(&format!(
                "{}::transaction_context_test::TransactionContextStore",
                account.address()
            ))
            .unwrap(),
        )
        .unwrap();

    assert_eq!(multisig_address, txn_ctx_store.multisig_address);
    assert_eq!(txn_ctx_store.account_address, *account.address());
    assert_eq!(txn_ctx_store.module_name, "transaction_context_test");
    assert_eq!(
        txn_ctx_store.function_name,
        "store_multisig_payload_from_native_txn_context"
    );
    assert!(txn_ctx_store.type_arg_names.is_empty());
    assert!(txn_ctx_store.args.is_empty());
}
