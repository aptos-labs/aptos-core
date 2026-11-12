// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    bootstrap_async,
    client::RosettaClient,
    common::native_coin,
    routes,
    types::{
        AccountBalanceRequest, AccountIdentifier, Currency, CurrencyMetadata, NetworkIdentifier,
        NetworkRequest, OperationType, Transaction, FUNGIBLE_ASSET_MODULE, FUNGIBLE_STORE_RESOURCE,
        OBJECT_CORE_RESOURCE, OBJECT_MODULE, OBJECT_RESOURCE_GROUP,
    },
    RosettaContext,
};
use aptos_config::{
    config::{ApiConfig, RoleType},
    utils::get_available_port,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    HashValue, PrivateKey, SigningKey, Uniform,
};
use aptos_rest_client::aptos_api_types::{
    mime_types::BCS, BcsBlock, IndexResponseBcs, ResourceGroup, TransactionOnChainData,
    X_APTOS_BLOCK_HEIGHT, X_APTOS_CHAIN_ID, X_APTOS_EPOCH, X_APTOS_LEDGER_OLDEST_VERSION,
    X_APTOS_LEDGER_TIMESTAMP, X_APTOS_LEDGER_VERSION, X_APTOS_OLDEST_BLOCK_HEIGHT,
};
use aptos_types::{
    account_address::create_derived_object_address,
    account_config::{
        fungible_store::FungibleStoreResource, AccountResource, DepositFAEvent, ObjectCoreResource,
        WithdrawFAEvent,
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
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};
use url::Url;
use warp::{
    http::{
        header, HeaderMap, HeaderName, HeaderValue, Method, Response as HttpResponse, StatusCode,
    },
    path::FullPath,
    Filter,
};

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

    RosettaContext::new(None, ChainId::test(), None, currencies, HashSet::new()).await
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

#[derive(Clone, Debug)]
struct RecordedNodeCall {
    method: Method,
    path: String,
    headers: HeaderMap,
}

fn mock_node_bcs_response(body: Vec<u8>, chain_id: ChainId) -> HttpResponse<Vec<u8>> {
    HttpResponse::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, BCS)
        .header(X_APTOS_CHAIN_ID, chain_id.id().to_string())
        .header(X_APTOS_LEDGER_VERSION, "10")
        .header(X_APTOS_LEDGER_OLDEST_VERSION, "0")
        .header(X_APTOS_LEDGER_TIMESTAMP, "1700000000000000")
        .header(X_APTOS_EPOCH, "1")
        .header(X_APTOS_BLOCK_HEIGHT, "5")
        .header(X_APTOS_OLDEST_BLOCK_HEIGHT, "1")
        .body(body)
        .unwrap()
}

async fn spawn_mock_fullnode(
    chain_id: ChainId,
) -> (
    Url,
    Arc<Mutex<Vec<RecordedNodeCall>>>,
    tokio::task::JoinHandle<()>,
) {
    let recorded: Arc<Mutex<Vec<RecordedNodeCall>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded_filter = {
        let recorded = recorded.clone();
        warp::method()
            .and(warp::path::full())
            .and(warp::header::headers_cloned())
            .map(
                move |method: Method, full_path: FullPath, headers: HeaderMap| {
                    recorded.lock().unwrap().push(RecordedNodeCall {
                        method,
                        path: full_path.as_str().to_string(),
                        headers,
                    });
                },
            )
            .untuple_one()
    };

    let index_route = warp::path!("v1")
        .and(warp::path::end())
        .and(warp::get())
        .and(recorded_filter.clone())
        .map(move || {
            let index = IndexResponseBcs {
                chain_id: chain_id.id(),
                epoch: 1.into(),
                ledger_version: 10.into(),
                oldest_ledger_version: 0.into(),
                ledger_timestamp: 1_700_000_000_000_000.into(),
                node_role: RoleType::FullNode,
                oldest_block_height: 1.into(),
                block_height: 5.into(),
            };
            mock_node_bcs_response(bcs::to_bytes(&index).unwrap(), chain_id)
        });

    let block_route = warp::path!("v1" / "blocks" / "by_height" / u64)
        .and(warp::get())
        .and(recorded_filter.clone())
        .map(move |height: u64| {
            let block = BcsBlock {
                block_height: height,
                block_hash: HashValue::zero(),
                block_timestamp: 1_700_000_000_000_000,
                first_version: height,
                last_version: height,
                transactions: Some(vec![]),
            };
            mock_node_bcs_response(bcs::to_bytes(&block).unwrap(), chain_id)
        });

    let account_resource_route = warp::path!("v1" / "accounts" / String / "resource" / String)
        .and(warp::get())
        .and(recorded_filter.clone())
        .map(move |_addr: String, _resource: String| {
            let account = AccountResource::new_stateless(AccountAddress::ONE);
            mock_node_bcs_response(bcs::to_bytes(&account).unwrap(), chain_id)
        });

    let view_route = warp::path!("v1" / "view")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(recorded_filter.clone())
        .map(move |_body: warp::hyper::body::Bytes| {
            let balances = vec![42_000u64];
            mock_node_bcs_response(bcs::to_bytes(&balances).unwrap(), chain_id)
        });

    let healthy_route = warp::path!("v1" / "-" / "healthy")
        .and(warp::get())
        .and(recorded_filter)
        .map(move || mock_node_bcs_response(b"ok".to_vec(), chain_id));

    let all_routes = index_route
        .or(block_route)
        .or(account_resource_route)
        .or(view_route)
        .or(healthy_route);

    let (addr, server) = warp::serve(all_routes).bind_ephemeral(([127, 0, 0, 1], 0));
    let handle = tokio::spawn(server);
    let url = Url::parse(&format!("http://{}", addr)).unwrap();
    (url, recorded, handle)
}

async fn spawn_rosetta_with_mock_node(
    chain_id: ChainId,
    node_url: Url,
    forward_headers: HashSet<HeaderName>,
) -> (Url, RosettaClient, tokio::task::JoinHandle<()>) {
    let port = get_available_port();
    let rosetta_addr = format!("127.0.0.1:{}", port);
    let rosetta_url: Url = format!("http://{}", rosetta_addr).parse().unwrap();
    let api_config = ApiConfig {
        enabled: true,
        address: rosetta_addr.parse().unwrap(),
        ..Default::default()
    };

    let handle = bootstrap_async(
        chain_id,
        api_config,
        Some(aptos_rest_client::Client::new(node_url)),
        HashSet::new(),
        forward_headers,
    )
    .await
    .unwrap();

    let client = RosettaClient::new(rosetta_url.clone());
    for _ in 0..50 {
        if client.network_list().await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    (rosetta_url, client, handle)
}

#[tokio::test]
async fn test_default_empty_allowlist_drops_incoming_headers() {
    let chain_id = ChainId::test();
    let (node_url, recorded, _node_handle) = spawn_mock_fullnode(chain_id).await;
    let (_rosetta_url, rosetta_client, _rosetta_handle) =
        spawn_rosetta_with_mock_node(chain_id, node_url, HashSet::new()).await;

    // Clear startup calls (e.g. bootstrap chain_id check)
    recorded.lock().unwrap().clear();

    let mut incoming_headers = HeaderMap::new();
    incoming_headers.insert("x-client-id", HeaderValue::from_static("caller-1"));
    incoming_headers.insert("x-request-id", HeaderValue::from_static("req-1"));

    rosetta_client
        .with_headers(incoming_headers)
        .network_status(&NetworkRequest {
            network_identifier: NetworkIdentifier::from(chain_id),
        })
        .await
        .unwrap();

    let calls = recorded.lock().unwrap();
    assert!(
        calls.len() >= 3,
        "Expected /network/status to make at least 3 node calls, got {}",
        calls.len()
    );
    for call in calls.iter() {
        assert!(
            call.headers.get("x-client-id").is_none(),
            "Default empty allowlist must not forward x-client-id on {}",
            call.path
        );
        assert!(
            call.headers.get("x-request-id").is_none(),
            "Default empty allowlist must not forward x-request-id on {}",
            call.path
        );
    }
}

#[tokio::test]
async fn test_forward_allowlisted_headers_across_all_outbound_node_calls() {
    let chain_id = ChainId::test();
    let (node_url, recorded, _node_handle) = spawn_mock_fullnode(chain_id).await;
    let allowlist: HashSet<HeaderName> = [
        HeaderName::from_static("x-client-id"),
        HeaderName::from_static("x-request-id"),
    ]
    .into_iter()
    .collect();
    let (rosetta_url, rosetta_client, _rosetta_handle) =
        spawn_rosetta_with_mock_node(chain_id, node_url, allowlist).await;

    recorded.lock().unwrap().clear();

    // /account/balance triggers multiple outbound calls across both rest_client and block_cache:
    // 1) GET /v1 (get_ledger_information)
    // 2) GET /v1/blocks/by_height/5 (BlockRetriever::get_block_info_by_height)
    // 3) GET /v1/accounts/.../resource/0x1::account::Account (get_sequence_number)
    // 4) POST /v1/view (get_base_balances)
    let mut incoming_headers = HeaderMap::new();
    incoming_headers.insert("X-Client-ID", HeaderValue::from_static("billing-service"));
    incoming_headers.insert("x-request-id", HeaderValue::from_static("trace-abc-999"));
    incoming_headers.insert("x-unlisted-secret", HeaderValue::from_static("do-not-copy"));

    let balance_resp = rosetta_client
        .with_headers(incoming_headers)
        .account_balance(&AccountBalanceRequest {
            network_identifier: NetworkIdentifier::from(chain_id),
            account_identifier: AccountIdentifier::base_account(AccountAddress::ONE),
            block_identifier: None,
            currencies: Some(vec![native_coin()]),
        })
        .await
        .unwrap();
    assert_eq!(balance_resp.balances.len(), 1);
    assert_eq!(balance_resp.balances[0].value, "42000");

    {
        let calls = recorded.lock().unwrap();
        assert_eq!(
            calls.len(),
            4,
            "Expected 4 outbound node calls for /account/balance, got: {:?}",
            calls
        );
        for call in calls.iter() {
            assert_eq!(
                call.headers.get("x-client-id"),
                Some(&HeaderValue::from_static("billing-service")),
                "Missing x-client-id on outbound call {} {}",
                call.method,
                call.path
            );
            assert_eq!(
                call.headers.get("x-request-id"),
                Some(&HeaderValue::from_static("trace-abc-999")),
                "Missing x-request-id on outbound call {} {}",
                call.method,
                call.path
            );
            assert!(
                call.headers.get("x-unlisted-secret").is_none(),
                "Non-allowlisted header must be dropped on {} {}",
                call.method,
                call.path
            );
        }
    }

    // Also verify GET /-/healthy forwards allowlisted headers
    recorded.lock().unwrap().clear();
    let http_resp = reqwest::Client::new()
        .get(rosetta_url.join("-/healthy").unwrap())
        .header("x-client-id", "health-checker")
        .send()
        .await
        .unwrap();
    assert_eq!(http_resp.status(), StatusCode::OK);

    let calls = recorded.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/v1/-/healthy");
    assert_eq!(
        calls[0].headers.get("x-client-id"),
        Some(&HeaderValue::from_static("health-checker"))
    );
    assert!(calls[0].headers.get("x-request-id").is_none());
}

#[tokio::test]
async fn test_per_request_header_isolation_no_cross_request_leakage() {
    let chain_id = ChainId::test();
    let (node_url, recorded, _node_handle) = spawn_mock_fullnode(chain_id).await;
    let allowlist: HashSet<HeaderName> = [
        HeaderName::from_static("x-client-id"),
        HeaderName::from_static("x-request-id"),
    ]
    .into_iter()
    .collect();
    let (_rosetta_url, rosetta_client, _rosetta_handle) =
        spawn_rosetta_with_mock_node(chain_id, node_url, allowlist).await;

    recorded.lock().unwrap().clear();

    let net_req = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };

    // Phase 1: Fire concurrent requests with distinct headers, partial headers, and no headers
    let mut handles = Vec::new();
    for i in 0..8usize {
        // Group A: both allowlisted headers + one unlisted header
        let client_a = rosetta_client.clone();
        let req_a = net_req.clone();
        handles.push(tokio::spawn(async move {
            let mut h = HeaderMap::new();
            h.insert(
                "x-client-id",
                HeaderValue::from_str(&format!("client-{i}")).unwrap(),
            );
            h.insert(
                "x-request-id",
                HeaderValue::from_str(&format!("req-{i}")).unwrap(),
            );
            h.insert(
                "x-other",
                HeaderValue::from_str(&format!("secret-{i}")).unwrap(),
            );
            client_a
                .with_headers(h)
                .network_status(&req_a)
                .await
                .unwrap();
        }));

        // Group B: only x-client-id (no x-request-id)
        let client_b = rosetta_client.clone();
        let req_b = net_req.clone();
        handles.push(tokio::spawn(async move {
            let mut h = HeaderMap::new();
            h.insert(
                "x-client-id",
                HeaderValue::from_str(&format!("partial-{i}")).unwrap(),
            );
            client_b
                .with_headers(h)
                .network_status(&req_b)
                .await
                .unwrap();
        }));

        // Group C: no allowlisted headers at all
        let client_c = rosetta_client.clone();
        let req_c = net_req.clone();
        handles.push(tokio::spawn(async move {
            let mut h = HeaderMap::new();
            h.insert("x-other", HeaderValue::from_static("unlisted-only"));
            client_c
                .with_headers(h)
                .network_status(&req_c)
                .await
                .unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    {
        let calls = recorded.lock().unwrap();
        // 24 Rosetta /network/status requests * 3 outbound node calls each = 72 calls
        assert_eq!(calls.len(), 72);

        let mut anon_count = 0usize;
        let mut partial_counts = [0usize; 8];
        let mut full_counts = [0usize; 8];

        for call in calls.iter() {
            assert!(call.headers.get("x-other").is_none());
            let cid = call
                .headers
                .get("x-client-id")
                .and_then(|v| v.to_str().ok());
            let rid = call
                .headers
                .get("x-request-id")
                .and_then(|v| v.to_str().ok());

            match (cid, rid) {
                (None, None) => {
                    anon_count += 1;
                },
                (Some(c), None) => {
                    let idx: usize = c
                        .strip_prefix("partial-")
                        .expect("Single-header call must come from partial group")
                        .parse()
                        .unwrap();
                    partial_counts[idx] += 1;
                },
                (Some(c), Some(r)) => {
                    let c_idx: usize = c
                        .strip_prefix("client-")
                        .expect("Paired call must come from client group")
                        .parse()
                        .unwrap();
                    let r_idx: usize = r
                        .strip_prefix("req-")
                        .expect("Paired call must have matching req-id")
                        .parse()
                        .unwrap();
                    assert_eq!(
                        c_idx, r_idx,
                        "Header leakage detected: x-client-id={} paired with x-request-id={}",
                        c, r
                    );
                    full_counts[c_idx] += 1;
                },
                (None, Some(r)) => {
                    panic!("Unexpected orphan x-request-id={} without x-client-id", r);
                },
            }
        }

        assert_eq!(anon_count, 24);
        assert_eq!(partial_counts, [3; 8]);
        assert_eq!(full_counts, [3; 8]);
    }

    // Phase 2: Sequential headered -> unheadered on the shared connection pool
    recorded.lock().unwrap().clear();
    let mut seq_headers = HeaderMap::new();
    seq_headers.insert("x-client-id", HeaderValue::from_static("seq-caller"));
    seq_headers.insert("x-request-id", HeaderValue::from_static("seq-req"));
    rosetta_client
        .with_headers(seq_headers)
        .network_status(&net_req)
        .await
        .unwrap();
    rosetta_client.network_status(&net_req).await.unwrap();

    let calls = recorded.lock().unwrap();
    assert_eq!(calls.len(), 6);
    for call in &calls[0..3] {
        assert_eq!(
            call.headers.get("x-client-id"),
            Some(&HeaderValue::from_static("seq-caller"))
        );
        assert_eq!(
            call.headers.get("x-request-id"),
            Some(&HeaderValue::from_static("seq-req"))
        );
    }
    for call in &calls[3..6] {
        assert!(call.headers.get("x-client-id").is_none());
        assert!(call.headers.get("x-request-id").is_none());
    }
}

#[tokio::test]
async fn test_cors_preflight_allows_configured_forward_headers() {
    let allowlist: HashSet<HeaderName> = [
        HeaderName::from_static("x-client-id"),
        HeaderName::from_static("x-request-id"),
    ]
    .into_iter()
    .collect();
    let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new(), allowlist).await;
    let router = routes(context);

    let resp = warp::test::request()
        .method("OPTIONS")
        .path("/network/list")
        .header("origin", "https://example.com")
        .header("access-control-request-method", "POST")
        .header(
            "access-control-request-headers",
            "content-type, x-client-id, x-request-id",
        )
        .reply(&router)
        .await;

    assert_eq!(resp.status(), StatusCode::OK);
}
