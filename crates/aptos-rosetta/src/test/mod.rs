// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::native_coin,
    types::{
        Currency, CurrencyMetadata, OperationType, Transaction, FUNGIBLE_ASSET_MODULE,
        FUNGIBLE_STORE_RESOURCE, OBJECT_CORE_RESOURCE, OBJECT_MODULE, OBJECT_RESOURCE_GROUP,
    },
    RosettaContext,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    HashValue, PrivateKey, Uniform,
};
use aptos_rest_client::aptos_api_types::{ResourceGroup, TransactionOnChainData};
use aptos_types::{
    account_config::{
        fungible_store::FungibleStoreResource, DepositFAEvent, ObjectCoreResource, WithdrawFAEvent,
    },
    chain_id::ChainId,
    contract_event::ContractEvent,
    event::{EventHandle, EventKey},
    move_utils::move_event_v2::MoveEventV2Type,
    on_chain_config::CurrentTimeMicroseconds,
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    test_helpers::transaction_test_helpers::get_test_raw_transaction,
    transaction::{ExecutionStatus, TransactionInfo, TransactionInfoV0},
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

fn test_transaction(
    sender: AccountAddress,
    version: u64,
    changes: WriteSet,
    events: Vec<ContractEvent>,
) -> TransactionOnChainData {
    // generate random key
    let private_key = Ed25519PrivateKey::generate_for_testing();

    // TODO[Orderless]: Also test with orderless transactions.
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
                    false,     // Use txn payload v2 format
                    false,     // Use orderless transactions
                ),
                // Dummy keys and signatures
                private_key.public_key(),
                Ed25519Signature::dummy_signature(),
            ),
        ),
        info: TransactionInfo::V0(TransactionInfoV0::new(
            HashValue::random(),
            HashValue::random(),
            HashValue::random(),
            None,
            178,                      // gas used, chosen arbitrarily
            ExecutionStatus::Success, // TODO: Add other statuses
        )),
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
            (self.previous_balance + self.amount, event.create_event_v2())
        } else {
            let event = WithdrawFAEvent {
                store: self.store_address,
                amount: self.amount,
            };

            (self.previous_balance - self.amount, event.create_event_v2())
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
    let store_address = AccountAddress::random();
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
    let store_address = AccountAddress::random();
    let receiver_store_address = AccountAddress::random();
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
async fn test_fa_transfer_other_currency() {
    let context = test_rosetta_context().await;

    let version = 0;
    let amount = 100;
    let sender = AccountAddress::random();
    let receiver = AccountAddress::random();
    let store_address = AccountAddress::random();
    let receiver_store_address = AccountAddress::random();
    let (changes, events) = transfer_fa_output(
        sender,
        AccountAddress::from_str(OTHER_CURRENCY_ADDRESS).unwrap(),
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
