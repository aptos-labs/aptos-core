// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::native_coin,
    types::{
        AccountIdentifier, Amount, Currency, CurrencyMetadata, Operation, OperationType,
        Transaction, Transfer, FUNGIBLE_ASSET_MODULE, FUNGIBLE_STORE_RESOURCE,
        OBJECT_CORE_RESOURCE, OBJECT_MODULE, OBJECT_RESOURCE_GROUP,
    },
    RosettaContext,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    HashValue, PrivateKey, SigningKey, Uniform,
};
use aptos_rest_client::aptos_api_types::{ResourceGroup, TransactionOnChainData};
use aptos_types::{
    account_address::create_derived_object_address,
    account_config::{
        fungible_store::FungibleStoreResource, DepositFAEvent, ObjectCoreResource, WithdrawFAEvent,
    },
    chain_id::ChainId,
    contract_event::ContractEvent,
    event::{EventHandle, EventKey},
    fee_statement::FeeStatement,
    move_utils::move_event_v2::MoveEventV2Type,
    on_chain_config::CurrentTimeMicroseconds,
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    test_helpers::transaction_test_helpers::get_test_raw_transaction,
    transaction::{ExecutionStatus, TransactionInfo},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::StructTag};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::{collections::HashSet, str::FromStr};

const APT_ADDRESS: AccountAddress = AccountAddress::TEN;
const OTHER_CURRENCY_ADDRESS: &str = "0x12341234123412341234123412341234";
static OTHER_CURRENCY: Lazy<Currency> = Lazy::new(|| Currency {
    symbol: "FUN".to_string(),
    decimals: 2,
    metadata: Some(CurrencyMetadata {
        move_type: None,
        fa_address: Some(OTHER_CURRENCY_ADDRESS.to_string()),
    }),
});

async fn test_rosetta_context() -> RosettaContext {
    let mut currencies = HashSet::new();
    currencies.insert(OTHER_CURRENCY.clone());

    RosettaContext::new(None, ChainId::test(), None, currencies).await
}

#[tokio::test]
async fn test_extract_transfer_rejects_sign_confused_amounts() {
    let context = test_rosetta_context().await;
    let sender = AccountAddress::random();
    let receiver = AccountAddress::random();

    let mut ops = vec![
        Operation::withdraw(
            0,
            None,
            AccountIdentifier::base_account(sender),
            native_coin(),
            1,
        ),
        Operation::deposit(
            1,
            None,
            AccountIdentifier::base_account(receiver),
            native_coin(),
            1,
        ),
    ];

    // A well-formed transfer of 1 unit is accepted with the right amount.
    let transfer = Transfer::extract_transfer(&context, &ops).expect("valid transfer");
    assert_eq!(transfer.amount.0, 1);

    // Flip the signs: withdraw "1", deposit "-1". -(1) == -1 satisfies the
    // negation check and -1 <= u64::MAX, but casting -1 to u64 wraps to
    // u64::MAX. This must be rejected, not silently turned into a huge amount.
    ops[0].amount = Some(Amount {
        value: "1".to_string(),
        currency: native_coin(),
    });
    ops[1].amount = Some(Amount {
        value: "-1".to_string(),
        currency: native_coin(),
    });
    assert!(
        Transfer::extract_transfer(&context, &ops).is_err(),
        "transfer with a negative deposit amount must be rejected"
    );

    // An i128::MIN withdraw must not panic or wrap while negating.
    ops[0].amount = Some(Amount {
        value: i128::MIN.to_string(),
        currency: native_coin(),
    });
    ops[1].amount = Some(Amount {
        value: "1".to_string(),
        currency: native_coin(),
    });
    assert!(Transfer::extract_transfer(&context, &ops).is_err());
}

fn primary_store_address(owner: AccountAddress, metadata: AccountAddress) -> AccountAddress {
    create_derived_object_address(owner, metadata)
}

fn test_transaction(
    sender: AccountAddress,
    version: u64,
    changes: WriteSet,
    events: Vec<ContractEvent>,
) -> TransactionOnChainData {
    // generate random key
    let private_key = Ed25519PrivateKey::generate_for_testing();

    TransactionOnChainData {
        version,
        transaction: aptos_types::transaction::Transaction::UserTransaction(
            aptos_types::transaction::SignedTransaction::new(
                get_test_raw_transaction(
                    sender,
                    0,         // Sequence number doesn't matter for this
                    None,      // TODO: payload
                    None,      // Expiration timestamp
                    Some(101), // Gas unit price, specifically make it different than 100 to check calculations
                    None,      // Max gas amount
                ),
                // Dummy keys and signatures
                private_key.public_key(),
                Ed25519Signature::dummy_signature(),
            ),
        ),
        info: TransactionInfo::builder_v0()
            .transaction_hash(HashValue::random())
            .state_change_hash(HashValue::random())
            .event_root_hash(HashValue::random())
            .gas_used(178) // chosen arbitrarily
            .status(ExecutionStatus::Success) // TODO: Add other statuses
            .build(),
        events,
        accumulator_root_hash: Default::default(),
        changes,
    }
}

fn resource_group_modification_write_op<T: Serialize>(
    address: &AccountAddress,
    resource: &StructTag,
    input: &T,
) -> (StateKey, WriteOp) {
    let encoded = bcs::to_bytes(input).unwrap();
    let state_key = StateKey::resource_group(address, resource);
    let write_op = WriteOp::modification(
        encoded.into(),
        StateValueMetadata::new(0, 0, &CurrentTimeMicroseconds { microseconds: 0 }),
    );
    (state_key, write_op)
}

struct FaData {
    fa_metadata_address: AccountAddress,
    owner: AccountAddress,
    store_address: AccountAddress,
    previous_balance: u64,
    deposit: bool,
    amount: u64,
}

impl FaData {
    fn create_change(&self) -> (Vec<(StateKey, WriteOp)>, Vec<ContractEvent>) {
        let object_core = ObjectCoreResource {
            guid_creation_num: 0,
            owner: self.owner,
            allow_ungated_transfer: false,
            transfer_events: EventHandle::new(EventKey::new(42, self.owner), 22),
        };

        let (new_balance, contract_event) = if self.deposit {
            let event = DepositFAEvent {
                store: self.store_address,
                amount: self.amount,
            };
            (
                self.previous_balance + self.amount,
                event
                    .create_event_v2()
                    .expect("Creating DepositFAEvent should always succeed"),
            )
        } else {
            let event = WithdrawFAEvent {
                store: self.store_address,
                amount: self.amount,
            };

            (
                self.previous_balance - self.amount,
                event
                    .create_event_v2()
                    .expect("Creating WithdrawFAEvent should always succeed"),
            )
        };

        let store = FungibleStoreResource::new(self.fa_metadata_address, new_balance, false);
        let mut group = ResourceGroup::new();
        group.insert(
            StructTag {
                address: AccountAddress::ONE,
                module: ident_str!(OBJECT_MODULE).into(),
                name: ident_str!(OBJECT_CORE_RESOURCE).into(),
                type_args: vec![],
            },
            bcs::to_bytes(&object_core).unwrap(),
        );
        group.insert(
            StructTag {
                address: AccountAddress::ONE,
                module: ident_str!(FUNGIBLE_ASSET_MODULE).into(),
                name: ident_str!(FUNGIBLE_STORE_RESOURCE).into(),
                type_args: vec![],
            },
            bcs::to_bytes(&store).unwrap(),
        );

        let write_ops = vec![
            // Update sender
            resource_group_modification_write_op(
                &self.store_address,
                &StructTag {
                    address: AccountAddress::ONE,
                    module: ident_str!(OBJECT_MODULE).into(),
                    name: ident_str!(OBJECT_RESOURCE_GROUP).into(),
                    type_args: vec![],
                },
                &group,
            ),
        ];

        (write_ops, vec![contract_event])
    }
}

fn mint_fa_output(
    owner: AccountAddress,
    fa_address: AccountAddress,
    store_address: AccountAddress,
    previous_balance: u64,
    amount: u64,
) -> (WriteSet, Vec<ContractEvent>) {
    let (minter_ops, minter_events) = FaData {
        fa_metadata_address: fa_address,
        owner,
        store_address,
        previous_balance,
        deposit: true,
        amount,
    }
    .create_change();

    let write_set = WriteSetMut::new(minter_ops).freeze().unwrap();
    (write_set, minter_events)
}
fn transfer_fa_output(
    owner: AccountAddress,
    fa_address: AccountAddress,
    store_address: AccountAddress,
    previous_balance: u64,
    dest: AccountAddress,
    dest_store_address: AccountAddress,
    dest_previous_balance: u64,
    amount: u64,
) -> (WriteSet, Vec<ContractEvent>) {
    let (mut sender_ops, mut sender_events) = FaData {
        fa_metadata_address: fa_address,
        owner,
        store_address,
        previous_balance,
        deposit: false,
        amount,
    }
    .create_change();

    let (mut dest_ops, mut dest_events) = FaData {
        fa_metadata_address: fa_address,
        owner: dest,
        store_address: dest_store_address,
        previous_balance: dest_previous_balance,
        deposit: true,
        amount,
    }
    .create_change();
    sender_ops.append(&mut dest_ops);
    sender_events.append(&mut dest_events);
    let write_set = WriteSetMut::new(sender_ops).freeze().unwrap();
    (write_set, sender_events)
}

#[tokio::test]
async fn test_fa_mint() {
    let context = test_rosetta_context().await;

    let version = 0;
    let amount = 100;
    let sender = AccountAddress::random();
    let store_address = primary_store_address(sender, APT_ADDRESS);
    let (mint_changes, mint_events) = mint_fa_output(sender, APT_ADDRESS, store_address, 0, amount);
    let input = test_transaction(sender, version, mint_changes, mint_events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");
    assert_eq!(2, expected_txn.operations.len());

    // TODO: Check that reading is working correctly
    let operation_1 = expected_txn.operations.first().unwrap();
    assert_eq!(
        operation_1.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        operation_1.amount.as_ref().unwrap().value,
        format!("{}", amount)
    );
    assert_eq!(
        operation_1
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender,
    );
    let operation_2 = expected_txn.operations.get(1).unwrap();
    assert_eq!(operation_2.operation_type, OperationType::Fee.to_string());
    assert_eq!(
        operation_2
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender,
    );
    // TODO: Check fee
}

#[tokio::test]
async fn test_fa_transfer() {
    let context = test_rosetta_context().await;

    let version = 0;
    let amount = 100;
    let sender = AccountAddress::random();
    let receiver = AccountAddress::random();
    let store_address = primary_store_address(sender, APT_ADDRESS);
    let receiver_store_address = primary_store_address(receiver, APT_ADDRESS);
    let (changes, events) = transfer_fa_output(
        sender,
        APT_ADDRESS,
        store_address,
        amount * 2,
        receiver,
        receiver_store_address,
        0,
        amount,
    );
    let input = test_transaction(sender, version, changes, events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");
    assert_eq!(3, expected_txn.operations.len(), "Ops: {:#?}", expected_txn);

    // TODO: Check that reading is working correctly
    // TODO: Do we want to order these?
    let operation_1 = expected_txn.operations.first().unwrap();
    assert_eq!(
        operation_1
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender
    );
    assert_eq!(
        operation_1.operation_type,
        OperationType::Withdraw.to_string()
    );
    assert_eq!(
        operation_1.amount.as_ref().unwrap().value,
        format!("-{}", amount)
    );
    let operation_2 = expected_txn.operations.get(1).unwrap();
    assert_eq!(
        operation_2.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        operation_2.amount.as_ref().unwrap().value,
        format!("{}", amount)
    );
    assert_eq!(
        operation_2
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        receiver
    );
    let operation_3 = expected_txn.operations.get(2).unwrap();
    assert_eq!(operation_3.operation_type, OperationType::Fee.to_string());
    assert_eq!(
        operation_3
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender
    );
    // TODO: Check fee
}

#[tokio::test]
async fn test_mainnet_apt_fa_transfer_tracks_primary_stores() {
    let context = test_rosetta_context().await;

    // Mainnet transaction 4141282946:
    // https://explorer.aptoslabs.com/txn/4141282946/userTxnOverview?network=mainnet
    let version = 4_141_282_946;
    let amount = 100_000;
    let sender = AccountAddress::from_str(
        "0x5b470b5a9536d94da23746acf069d1d41e45e2b01605ab4aa1b3892f6cac5f5f",
    )
    .unwrap();
    let receiver = AccountAddress::from_str(
        "0xc67545d6f3d36ed01efc9b28cbfd0c1ae326d5d262dd077a29539bcee0edce9e",
    )
    .unwrap();
    let store_address = AccountAddress::from_str(
        "0x9016ac2c7b1016f621e463edf37bf4993a45f810df80bbec1cd1b25453cd0d5e",
    )
    .unwrap();
    let receiver_store_address = AccountAddress::from_str(
        "0xeba1a450b0a5e7cbda0e2ed079bbbe9bd17aeda5910d832a3b22888e38633d76",
    )
    .unwrap();

    assert_eq!(store_address, primary_store_address(sender, APT_ADDRESS));
    assert_eq!(
        receiver_store_address,
        primary_store_address(receiver, APT_ADDRESS)
    );

    let (changes, events) = transfer_fa_output(
        sender,
        APT_ADDRESS,
        store_address,
        60_420_417,
        receiver,
        receiver_store_address,
        10_362_875_460,
        amount,
    );
    let input = test_transaction(sender, version, changes, events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");
    assert_eq!(3, expected_txn.operations.len(), "Ops: {:#?}", expected_txn);

    let withdraw_op = expected_txn.operations.first().unwrap();
    assert_eq!(
        withdraw_op.operation_type,
        OperationType::Withdraw.to_string()
    );
    assert_eq!(
        withdraw_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender
    );
    assert_eq!(
        withdraw_op.amount.as_ref().unwrap().value,
        format!("-{}", amount)
    );
    assert_eq!(withdraw_op.amount.as_ref().unwrap().currency, native_coin());

    let deposit_op = expected_txn.operations.get(1).unwrap();
    assert_eq!(
        deposit_op.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        deposit_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        receiver
    );
    assert_eq!(
        deposit_op.amount.as_ref().unwrap().value,
        format!("{}", amount)
    );
    assert_eq!(deposit_op.amount.as_ref().unwrap().currency, native_coin());
}

#[tokio::test]
async fn test_mainnet_reclaim_non_primary_store_tracks_secondary_store() {
    let context = test_rosetta_context().await;

    // Mainnet transaction 5519638905:
    // https://explorer.aptoslabs.com/txn/5519638905/userTxnOverview?network=mainnet
    let version = 5_519_638_905;
    let amount = 50_000_000;
    let sender = AccountAddress::from_str(
        "0xf10387b231218d7ad53fb44ff6cd9eda5e50b3e3ed6aaf4d13b9f9e54d1f4cae",
    )
    .unwrap();
    let non_primary_store_address = AccountAddress::from_str(
        "0x76dbc266c7c11ba64308fcfa6b2507c77b3e2bfa48edcf6235e9f15eeeb5839d",
    )
    .unwrap();
    let expected_primary_store_address = AccountAddress::from_str(
        "0xd0f5f74b99a9fe669a3cbd88215c6ee175113e12a33158b8ad21ce39af5ad6fe",
    )
    .unwrap();

    assert_ne!(
        non_primary_store_address,
        primary_store_address(sender, APT_ADDRESS)
    );
    assert_eq!(
        expected_primary_store_address,
        primary_store_address(sender, APT_ADDRESS)
    );

    let (changes, events) =
        mint_fa_output(sender, APT_ADDRESS, non_primary_store_address, 0, amount);
    let input = test_transaction(sender, version, changes, events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");
    assert_eq!(2, expected_txn.operations.len(), "Ops: {:#?}", expected_txn);

    let deposit_op = expected_txn.operations.first().unwrap();
    assert_eq!(
        deposit_op.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        deposit_op.account.as_ref().unwrap(),
        &AccountIdentifier::secondary_store_account(&non_primary_store_address, &native_coin())
    );
    assert_eq!(
        deposit_op.amount.as_ref().unwrap().value,
        format!("{}", amount)
    );
    assert_eq!(deposit_op.amount.as_ref().unwrap().currency, native_coin());

    let fee_op = expected_txn.operations.get(1).unwrap();
    assert_eq!(fee_op.operation_type, OperationType::Fee.to_string());
    assert_eq!(
        fee_op.account.as_ref().unwrap().account_address().unwrap(),
        sender
    );
}

#[tokio::test]
async fn test_fa_transfer_other_currency() {
    let context = test_rosetta_context().await;

    let version = 0;
    let amount = 100;
    let sender = AccountAddress::random();
    let receiver = AccountAddress::random();
    let fa_metadata_address = AccountAddress::from_str(OTHER_CURRENCY_ADDRESS).unwrap();
    let store_address = primary_store_address(sender, fa_metadata_address);
    let receiver_store_address = primary_store_address(receiver, fa_metadata_address);
    let (changes, events) = transfer_fa_output(
        sender,
        fa_metadata_address,
        store_address,
        amount * 2,
        receiver,
        receiver_store_address,
        0,
        amount,
    );
    let input = test_transaction(sender, version, changes, events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");
    assert_eq!(3, expected_txn.operations.len(), "Ops: {:#?}", expected_txn);

    // TODO: Check that reading is working correctly
    // TODO: Do we want to order these?
    let operation_1 = expected_txn.operations.first().unwrap();
    assert_eq!(
        operation_1
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender
    );
    assert_eq!(
        operation_1.operation_type,
        OperationType::Withdraw.to_string()
    );
    assert_eq!(
        operation_1.amount.as_ref().unwrap().value,
        format!("-{}", amount)
    );
    assert_eq!(
        operation_1.amount.as_ref().unwrap().currency,
        OTHER_CURRENCY.to_owned()
    );
    let operation_2 = expected_txn.operations.get(1).unwrap();
    assert_eq!(
        operation_2.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        operation_2.amount.as_ref().unwrap().value,
        format!("{}", amount)
    );
    assert_eq!(
        operation_2
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        receiver
    );
    assert_eq!(
        operation_2.amount.as_ref().unwrap().currency,
        OTHER_CURRENCY.to_owned()
    );
    let operation_3 = expected_txn.operations.get(2).unwrap();
    assert_eq!(operation_3.operation_type, OperationType::Fee.to_string());
    assert_eq!(
        operation_3
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender
    );
    assert_eq!(operation_3.amount.as_ref().unwrap().currency, native_coin());
    // TODO: Check fee
}

fn test_fee_payer_transaction(
    sender: AccountAddress,
    fee_payer: AccountAddress,
    version: u64,
    changes: WriteSet,
    events: Vec<ContractEvent>,
) -> TransactionOnChainData {
    let sender_private_key = Ed25519PrivateKey::generate_for_testing();
    let fee_payer_private_key = Ed25519PrivateKey::generate_for_testing();

    let raw_txn = get_test_raw_transaction(sender, 0, None, None, Some(101), None);

    let sender_auth = aptos_types::transaction::authenticator::AccountAuthenticator::ed25519(
        sender_private_key.public_key(),
        sender_private_key.sign(&raw_txn).unwrap(),
    );
    let fee_payer_auth = aptos_types::transaction::authenticator::AccountAuthenticator::ed25519(
        fee_payer_private_key.public_key(),
        fee_payer_private_key.sign(&raw_txn).unwrap(),
    );

    TransactionOnChainData {
        version,
        transaction: aptos_types::transaction::Transaction::UserTransaction(
            aptos_types::transaction::SignedTransaction::new_fee_payer(
                raw_txn,
                sender_auth,
                vec![],
                vec![],
                fee_payer,
                fee_payer_auth,
            ),
        ),
        info: TransactionInfo::builder_v0()
            .transaction_hash(HashValue::random())
            .state_change_hash(HashValue::random())
            .event_root_hash(HashValue::random())
            .gas_used(178)
            .status(ExecutionStatus::Success)
            .build(),
        events,
        accumulator_root_hash: Default::default(),
        changes,
    }
}

#[tokio::test]
async fn test_fee_payer_transfer_attributes_fee_to_fee_payer() {
    let context = test_rosetta_context().await;

    let version = 0;
    let amount = 50000;
    let sender = AccountAddress::random();
    let fee_payer = AccountAddress::random();
    let receiver = AccountAddress::random();
    let store_address = primary_store_address(sender, APT_ADDRESS);
    let receiver_store_address = primary_store_address(receiver, APT_ADDRESS);
    let (changes, events) = transfer_fa_output(
        sender,
        APT_ADDRESS,
        store_address,
        amount * 2,
        receiver,
        receiver_store_address,
        0,
        amount,
    );
    let input = test_fee_payer_transaction(sender, fee_payer, version, changes, events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");
    assert_eq!(3, expected_txn.operations.len(), "Ops: {:#?}", expected_txn);

    let withdraw_op = expected_txn.operations.first().unwrap();
    assert_eq!(
        withdraw_op.operation_type,
        OperationType::Withdraw.to_string()
    );
    assert_eq!(
        withdraw_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender
    );

    let deposit_op = expected_txn.operations.get(1).unwrap();
    assert_eq!(
        deposit_op.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        deposit_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        receiver
    );

    let fee_op = expected_txn.operations.get(2).unwrap();
    assert_eq!(fee_op.operation_type, OperationType::Fee.to_string());
    assert_eq!(
        fee_op.account.as_ref().unwrap().account_address().unwrap(),
        fee_payer,
        "Fee should be attributed to the fee payer, not the sender"
    );
    assert_ne!(
        fee_op.account.as_ref().unwrap().account_address().unwrap(),
        sender,
        "Fee must not be attributed to the sender when a fee payer is present"
    );
}

#[tokio::test]
async fn test_fee_payer_mint_attributes_fee_to_fee_payer() {
    let context = test_rosetta_context().await;

    let version = 0;
    let amount = 100;
    let sender = AccountAddress::random();
    let fee_payer = AccountAddress::random();
    let store_address = primary_store_address(sender, APT_ADDRESS);
    let (mint_changes, mint_events) = mint_fa_output(sender, APT_ADDRESS, store_address, 0, amount);
    let input = test_fee_payer_transaction(sender, fee_payer, version, mint_changes, mint_events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");
    assert_eq!(2, expected_txn.operations.len());

    let deposit_op = expected_txn.operations.first().unwrap();
    assert_eq!(
        deposit_op.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        deposit_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender,
    );

    let fee_op = expected_txn.operations.get(1).unwrap();
    assert_eq!(fee_op.operation_type, OperationType::Fee.to_string());
    assert_eq!(
        fee_op.account.as_ref().unwrap().account_address().unwrap(),
        fee_payer,
        "Fee should be attributed to the fee payer, not the sender"
    );
}

#[tokio::test]
async fn test_fee_payer_storage_refund_attributes_to_fee_payer() {
    let context = test_rosetta_context().await;

    let version = 0;
    let amount = 100;
    let storage_refund = 500u64;
    let sender = AccountAddress::random();
    let fee_payer = AccountAddress::random();
    let store_address = primary_store_address(sender, APT_ADDRESS);

    let (mint_changes, mut mint_events) =
        mint_fa_output(sender, APT_ADDRESS, store_address, 0, amount);

    let fee_statement = FeeStatement::builder()
        .total_charge_gas_units(178)
        .execution_gas_units(100)
        .io_gas_units(50)
        .storage_fee_octas(28)
        .storage_fee_refund_octas(storage_refund)
        .build();
    mint_events.push(
        fee_statement
            .create_event_v2()
            .expect("Creating FeeStatement event should succeed"),
    );

    let input = test_fee_payer_transaction(sender, fee_payer, version, mint_changes, mint_events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");

    assert_eq!(
        3,
        expected_txn.operations.len(),
        "Expected deposit + storage refund deposit + fee, got: {:#?}",
        expected_txn
    );

    let deposit_op = expected_txn.operations.first().unwrap();
    assert_eq!(
        deposit_op.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        deposit_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender,
    );
    assert_eq!(
        deposit_op.amount.as_ref().unwrap().value,
        format!("{}", amount)
    );

    let refund_op = expected_txn.operations.get(1).unwrap();
    assert_eq!(refund_op.operation_type, OperationType::Deposit.to_string());
    assert_eq!(
        refund_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        fee_payer,
        "Storage fee refund should be attributed to the fee payer, not the sender"
    );
    assert_eq!(
        refund_op.amount.as_ref().unwrap().value,
        format!("{}", storage_refund)
    );

    let fee_op = expected_txn.operations.get(2).unwrap();
    assert_eq!(fee_op.operation_type, OperationType::Fee.to_string());
    assert_eq!(
        fee_op.account.as_ref().unwrap().account_address().unwrap(),
        fee_payer,
        "Gas fee should be attributed to the fee payer, not the sender"
    );
}

#[tokio::test]
async fn test_no_fee_payer_storage_refund_attributes_to_sender() {
    let context = test_rosetta_context().await;

    let version = 0;
    let amount = 100;
    let storage_refund = 500u64;
    let sender = AccountAddress::random();
    let store_address = primary_store_address(sender, APT_ADDRESS);

    let (mint_changes, mut mint_events) =
        mint_fa_output(sender, APT_ADDRESS, store_address, 0, amount);

    let fee_statement = FeeStatement::builder()
        .total_charge_gas_units(178)
        .execution_gas_units(100)
        .io_gas_units(50)
        .storage_fee_octas(28)
        .storage_fee_refund_octas(storage_refund)
        .build();
    mint_events.push(
        fee_statement
            .create_event_v2()
            .expect("Creating FeeStatement event should succeed"),
    );

    let input = test_transaction(sender, version, mint_changes, mint_events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");

    assert_eq!(
        3,
        expected_txn.operations.len(),
        "Expected deposit + storage refund deposit + fee, got: {:#?}",
        expected_txn
    );

    let deposit_op = expected_txn.operations.first().unwrap();
    assert_eq!(
        deposit_op.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        deposit_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender,
    );

    let refund_op = expected_txn.operations.get(1).unwrap();
    assert_eq!(refund_op.operation_type, OperationType::Deposit.to_string());
    assert_eq!(
        refund_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender,
        "Storage fee refund should fall back to sender when no fee payer is present"
    );
    assert_eq!(
        refund_op.amount.as_ref().unwrap().value,
        format!("{}", storage_refund)
    );

    let fee_op = expected_txn.operations.get(2).unwrap();
    assert_eq!(fee_op.operation_type, OperationType::Fee.to_string());
    assert_eq!(
        fee_op.account.as_ref().unwrap().account_address().unwrap(),
        sender,
        "Gas fee should fall back to sender when no fee payer is present"
    );
}

#[tokio::test]
async fn test_storage_refund_exceeds_gas_fee() {
    let context = test_rosetta_context().await;

    let version = 0;
    let amount = 100;
    let sender = AccountAddress::random();
    let fee_payer = AccountAddress::random();
    let store_address = primary_store_address(sender, APT_ADDRESS);

    // gas_used=178 and gas_unit_price=101 from test helpers, so gas fee = 17,978 octas.
    // Set storage refund to 25,000 so it exceeds the gas fee (net = +7,022 for fee payer).
    let gas_used: u64 = 178;
    let gas_unit_price: u64 = 101;
    let gas_fee = gas_used * gas_unit_price;
    let storage_refund: u64 = 25_000;
    assert!(
        storage_refund > gas_fee,
        "Test precondition: storage refund must exceed gas fee"
    );

    let (mint_changes, mut mint_events) =
        mint_fa_output(sender, APT_ADDRESS, store_address, 0, amount);

    let fee_statement = FeeStatement::builder()
        .total_charge_gas_units(gas_used)
        .execution_gas_units(100)
        .io_gas_units(50)
        .storage_fee_octas(28)
        .storage_fee_refund_octas(storage_refund)
        .build();
    mint_events.push(
        fee_statement
            .create_event_v2()
            .expect("Creating FeeStatement event should succeed"),
    );

    let input = test_fee_payer_transaction(sender, fee_payer, version, mint_changes, mint_events);

    let result = Transaction::from_transaction(&context, input).await;
    let expected_txn = result.expect("Must succeed");

    assert_eq!(
        3,
        expected_txn.operations.len(),
        "Expected deposit + storage refund deposit + fee, got: {:#?}",
        expected_txn
    );

    // Operation 0: the mint deposit to the sender
    let deposit_op = expected_txn.operations.first().unwrap();
    assert_eq!(
        deposit_op.operation_type,
        OperationType::Deposit.to_string()
    );
    assert_eq!(
        deposit_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        sender,
    );
    assert_eq!(
        deposit_op.amount.as_ref().unwrap().value,
        format!("{}", amount)
    );

    // Operation 1: storage refund deposit to the fee payer (positive value)
    let refund_op = expected_txn.operations.get(1).unwrap();
    assert_eq!(refund_op.operation_type, OperationType::Deposit.to_string());
    assert_eq!(
        refund_op
            .account
            .as_ref()
            .unwrap()
            .account_address()
            .unwrap(),
        fee_payer,
        "Storage refund should go to the fee payer"
    );
    assert_eq!(
        refund_op.amount.as_ref().unwrap().value,
        format!("{}", storage_refund),
        "Refund deposit should be the full storage_fee_refund amount"
    );

    // Operation 2: gas fee charged to the fee payer (negative value)
    let fee_op = expected_txn.operations.get(2).unwrap();
    assert_eq!(fee_op.operation_type, OperationType::Fee.to_string());
    assert_eq!(
        fee_op.account.as_ref().unwrap().account_address().unwrap(),
        fee_payer,
        "Gas fee should be charged to the fee payer"
    );
    assert_eq!(
        fee_op.amount.as_ref().unwrap().value,
        format!("-{}", gas_fee),
        "Gas fee should be gas_used * gas_unit_price"
    );

    // Verify the net effect: fee payer gets value back since refund > gas fee
    let fee_value: i128 = fee_op.amount.as_ref().unwrap().value.parse().unwrap();
    let refund_value: i128 = refund_op.amount.as_ref().unwrap().value.parse().unwrap();
    let net = fee_value + refund_value;
    assert!(
        net > 0,
        "Net effect on fee payer should be positive when refund ({storage_refund}) > gas fee ({gas_fee}), but got net={net}"
    );
    assert_eq!(
        net,
        storage_refund as i128 - gas_fee as i128,
        "Net should equal storage_refund - gas_fee"
    );
}
