// Copyright (c) 2024 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::chain_id::ChainId;
use crate::move_utils::MemberId;
use crate::transaction::automated_transaction::{AutomatedTransactionBuilder, BuilderResult};
use crate::transaction::automation::{AutomationTaskMetaData, RegistrationParams};
use crate::transaction::{EntryFunction, TransactionPayload};
use aptos_crypto::HashValue;
use move_core_types::account_address::AccountAddress;
use std::str::FromStr;

#[test]
fn test_registration_params_serde() {
    let MemberId {
        module_id,
        member_id,
    } = MemberId::from_str("0x1::timestamp::now_seconds").unwrap();
    let expiry_time = 3600;
    let max_gas_amount = 10_000;
    let gas_price_cap = 500;
    let automation_fee_cap_for_epoch = 50_000_000;
    let aux_data = vec![vec![1u8, 2], vec![3, 4], vec![5, 6]];
    let entry_function = EntryFunction::new(module_id, member_id, vec![], vec![]);
    let registration_params = RegistrationParams::new_v1(
        entry_function.clone(),
        expiry_time,
        max_gas_amount,
        gas_price_cap,
        automation_fee_cap_for_epoch,
        aux_data.clone(),
    );
    let address = AccountAddress::random();
    let parent_hash = HashValue::random();
    let serialized = registration_params
        .serialized_args_with_sender_and_parent_hash(address, parent_hash.to_vec());
    // 4 params + address and parent hash
    assert_eq!(serialized.len(), 8);
    // Check the order fo serialized items
    // Address
    let v_address = bcs::from_bytes::<AccountAddress>(&serialized[0]).unwrap();
    assert_eq!(address, v_address);
    // EntryFunction double serialized
    let v_entry_bytes = bcs::from_bytes::<Vec<u8>>(&serialized[1]).unwrap();
    let v_entry = bcs::from_bytes::<EntryFunction>(&v_entry_bytes).unwrap();
    assert_eq!(entry_function, v_entry);
    // Timestamp
    let v_time = bcs::from_bytes::<u64>(&serialized[2]).unwrap();
    assert_eq!(expiry_time, v_time);
    // MaxGasAmount
    let v_max_gas_amount = bcs::from_bytes::<u64>(&serialized[3]).unwrap();
    assert_eq!(max_gas_amount, v_max_gas_amount);
    // GasPriceCap
    let v_gas_price_cap = bcs::from_bytes::<u64>(&serialized[4]).unwrap();
    assert_eq!(gas_price_cap, v_gas_price_cap);
    // AutomationFeeCap
    let v_automation_fee_cap = bcs::from_bytes::<u64>(&serialized[5]).unwrap();
    assert_eq!(automation_fee_cap_for_epoch, v_automation_fee_cap);
    // ParentHash
    let v_parent_hash_bytes = bcs::from_bytes::<Vec<u8>>(&serialized[6]).unwrap();
    let v_parent_hash = HashValue::from_slice(&v_parent_hash_bytes).unwrap();
    assert_eq!(parent_hash, v_parent_hash);
    // AuxData
    let v_aux_data = bcs::from_bytes::<Vec<Vec<u8>>>(&serialized[7]).unwrap();
    assert_eq!(aux_data, v_aux_data);
}

#[test]
fn automated_txn_builder_from_task_meta() {
    let task_meta_invalid_payload = AutomationTaskMetaData {
        id: 4,
        owner: AccountAddress::random(),
        payload_tx: vec![0, 1, 2],
        expiry_time: 7200,
        tx_hash: vec![42; 32],
        max_gas_amount: 10,
        gas_price_cap: 20,
        automation_fee_cap_for_epoch: 300,
        aux_data: vec![],
        registration_time: 1,
        is_active: false,
        locked_fee_for_next_epoch: 0,
    };
    assert!(AutomatedTransactionBuilder::try_from(task_meta_invalid_payload.clone()).is_err());

    let MemberId {
        module_id,
        member_id,
    } = MemberId::from_str("0x1::timestamp::now_seconds").unwrap();
    let entry_function = EntryFunction::new(module_id, member_id, vec![], vec![]);

    let task_meta_invalid_parent_hash = AutomationTaskMetaData {
        payload_tx: bcs::to_bytes(&entry_function).unwrap(),
        tx_hash: vec![42; 24],
        ..task_meta_invalid_payload
    };
    assert!(AutomatedTransactionBuilder::try_from(task_meta_invalid_parent_hash.clone()).is_err());

    let task_meta_valid = AutomationTaskMetaData {
        tx_hash: vec![42; 32],
        ..task_meta_invalid_parent_hash
    };
    let builder = AutomatedTransactionBuilder::try_from(task_meta_valid.clone()).unwrap();
    let AutomationTaskMetaData {
        id,
        owner,
        expiry_time,
        max_gas_amount,
        gas_price_cap,
        ..
    } = task_meta_valid;
    assert_eq!(builder.gas_price_cap, gas_price_cap);
    assert_eq!(builder.sender, Some(owner));
    assert_eq!(builder.sequence_number, Some(id));
    assert_eq!(
        builder.payload,
        Some(TransactionPayload::EntryFunction(entry_function))
    );
    assert_eq!(builder.max_gas_amount, Some(max_gas_amount));
    assert_eq!(builder.gas_unit_price, None);
    assert_eq!(builder.expiration_timestamp_secs, Some(expiry_time));
    assert_eq!(builder.chain_id, None);
    assert_eq!(
        builder.authenticator,
        Some(HashValue::from_slice(&[42; 32]).unwrap())
    );
    assert_eq!(builder.block_height, None);
}

#[test]
fn automated_txn_build() {
    let MemberId {
        module_id,
        member_id,
    } = MemberId::from_str("0x1::timestamp::now_seconds").unwrap();
    let entry_function = EntryFunction::new(module_id, member_id, vec![], vec![]);
    let address = AccountAddress::random();
    let parent_hash = HashValue::random();
    let chain_id = ChainId::new(1);
    let expiry_time = 7200;
    let task_meta = AutomationTaskMetaData {
        id: 0,
        owner: address,
        payload_tx: bcs::to_bytes(&entry_function).unwrap(),
        expiry_time,
        tx_hash: parent_hash.to_vec(),
        max_gas_amount: 10,
        gas_price_cap: 20,
        automation_fee_cap_for_epoch: 500,
        aux_data: vec![],
        registration_time: 1,
        is_active: false,
        locked_fee_for_next_epoch: 0,
    };

    let builder = AutomatedTransactionBuilder::try_from(task_meta.clone()).unwrap();
    // chain id and gas-unit-price  and block_height are missing
    assert!(matches!(
        builder.clone().build(),
        BuilderResult::MissingValue(_)
    ));

    // gas-unit-price & block height are missing
    let builder_with_chain_id = builder.with_chain_id(chain_id);
    assert!(matches!(
        builder_with_chain_id.clone().build(),
        BuilderResult::MissingValue(_)
    ));

    // block_height is missing, gas-unit-price < gas_price-cap
    let builder_with_gas_price = builder_with_chain_id.with_gas_unit_price(15);
    assert!(matches!(
        builder_with_gas_price.clone().build(),
        BuilderResult::MissingValue(_)
    ));

    let builder_valid = builder_with_gas_price.with_block_height(5);
    let automated_txn = builder_valid.clone().build();
    match &automated_txn {
        BuilderResult::Success(txn) => {
            assert_eq!(txn.expiration_timestamp_secs(), task_meta.expiry_time);
            assert_eq!(txn.sender(), address);
            assert_eq!(
                txn.payload(),
                &TransactionPayload::EntryFunction(entry_function.clone())
            );
            assert_eq!(txn.gas_unit_price(), 15);
            assert_eq!(txn.authenticator(), parent_hash);
            assert_eq!(txn.block_height(), 5);
            assert_eq!(txn.chain_id(), chain_id);
            assert_eq!(txn.max_gas_amount(), task_meta.max_gas_amount);
            assert_eq!(txn.sequence_number(), task_meta.id);
        },
        _ => panic!("Expected successful result, got: {automated_txn:?}"),
    }

    // Gas unit price cap is greater than gas-price-cap
    let builder_with_higher_gas_unit_price = builder_valid.clone().with_gas_unit_price(30);
    assert!(matches!(
        builder_with_higher_gas_unit_price.clone().build(),
        BuilderResult::GasPriceThresholdExceeded { .. }
    ));

    // Any other field if missing build will fail
    let mut builder_with_no_expiry_time = builder_valid.clone();
    builder_with_no_expiry_time.expiration_timestamp_secs = None;
    assert!(matches!(
        builder_with_no_expiry_time.clone().build(),
        BuilderResult::MissingValue(_)
    ));
}
