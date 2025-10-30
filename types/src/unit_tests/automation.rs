// Copyright (c) 2024 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::chain_id::ChainId;
use crate::move_utils::MemberId;
use crate::on_chain_config::{FeatureFlag, Features};
use crate::transaction::automated_transaction::{AutomatedTransactionBuilder, BuilderResult};
use crate::transaction::automation::{
    AutomationRegistryAction, AutomationRegistryRecordBuilder, AutomationTaskMetaData,
    AutomationTaskType, RegistrationParams,
};
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
    let aux_data = vec![vec![1u8, 1, 2, 3]];
    // Includes task type prepended to the user specified one.
    let expected_aux_data = vec![vec![AutomationTaskType::User as u8], vec![], vec![1u8, 1, 2, 3]];
    let entry_function = EntryFunction::new(module_id, member_id, vec![], vec![]);
    let registration_params = RegistrationParams::new_user_automation_task_v1(
        entry_function.clone(),
        expiry_time,
        max_gas_amount,
        gas_price_cap,
        automation_fee_cap_for_epoch,
        aux_data.clone(),
    );
    let address = AccountAddress::random();
    let parent_hash = HashValue::random();
    let mut features = Features::default();
    let serialized = registration_params.serialized_args_with_sender_and_parent_hash(
        address,
        parent_hash.to_vec(),
        &features,
    );
    // 6 params + address and parent hash
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
    assert_eq!(expected_aux_data, v_aux_data);

    features.disable(FeatureFlag::SUPRA_AUTOMATION_V2);
    let serialized = registration_params.serialized_args_with_sender_and_parent_hash(
        address,
        parent_hash.to_vec(),
        &features,
    );
    // AuxData if feature is disabled then aux data is not extended
    let v_aux_data = bcs::from_bytes::<Vec<Vec<u8>>>(&serialized[7]).unwrap();
    assert_eq!(aux_data, v_aux_data);
}

#[test]
fn test_registration_params_v2_user_task_serde() {
    let MemberId {
        module_id,
        member_id,
    } = MemberId::from_str("0x1::timestamp::now_seconds").unwrap();
    let expiry_time = 3600;
    let max_gas_amount = 10_000;
    let gas_price_cap = 500;
    let automation_fee_cap_for_epoch = 50_000_000;
    let aux_data = vec![vec![1u8, 1, 2, 3]];
    let priority = 42;
    // Includes task type prepended to the user specified one.
    let expected_aux_data = vec![
        vec![AutomationTaskType::User as u8],
        bcs::to_bytes(&priority).unwrap(),
        vec![1u8, 1, 2, 3],
    ];
    let entry_function = EntryFunction::new(module_id, member_id, vec![], vec![]);
    let registration_params = RegistrationParams::new_user_automation_task_v2(
        entry_function.clone(),
        expiry_time,
        max_gas_amount,
        gas_price_cap,
        automation_fee_cap_for_epoch,
        aux_data.clone(),
        Some(priority),
    );
    let address = AccountAddress::random();
    let parent_hash = HashValue::random();
    let mut features = Features::default();
    let serialized = registration_params.serialized_args_with_sender_and_parent_hash(
        address,
        parent_hash.to_vec(),
        &features,
    );
    // 6 params + address and parent hash
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
    assert_eq!(expected_aux_data, v_aux_data);

    let registration_params = RegistrationParams::new_user_automation_task_v2(
        entry_function.clone(),
        expiry_time,
        max_gas_amount,
        gas_price_cap,
        automation_fee_cap_for_epoch,
        aux_data.clone(),
        None,
    );
    let expected_aux_data = vec![vec![AutomationTaskType::User as u8], vec![], vec![1u8, 1, 2, 3]];
    let serialized = registration_params.serialized_args_with_sender_and_parent_hash(
        address,
        parent_hash.to_vec(),
        &features,
    );
    // 4 params + address and parent hash
    assert_eq!(serialized.len(), 8);
    // AuxData
    let v_aux_data = bcs::from_bytes::<Vec<Vec<u8>>>(&serialized[7]).unwrap();
    assert_eq!(expected_aux_data, v_aux_data);

    features.disable(FeatureFlag::SUPRA_AUTOMATION_V2);
    let serialized = registration_params.serialized_args_with_sender_and_parent_hash(
        address,
        parent_hash.to_vec(),
        &features,
    );
    // 4 params + address and parent hash
    assert_eq!(serialized.len(), 8);
    // AuxData if feature is disabled then aux data is not extended
    let v_aux_data = bcs::from_bytes::<Vec<Vec<u8>>>(&serialized[7]).unwrap();
    assert_eq!(aux_data, v_aux_data);
}

#[test]
fn test_registration_params_system_task_serde() {
    let MemberId {
        module_id,
        member_id,
    } = MemberId::from_str("0x1::timestamp::now_seconds").unwrap();
    let expiry_time = 3600;
    let max_gas_amount = 10_000;
    let aux_data = vec![vec![1u8, 1, 2, 3]];
    let priority = 42;
    // Includes task type prepended to the user specified one.
    let expected_aux_data = vec![
        vec![AutomationTaskType::System as u8],
        bcs::to_bytes(&priority).unwrap(),
        vec![1u8, 1, 2, 3],
    ];
    let entry_function = EntryFunction::new(module_id, member_id, vec![], vec![]);
    let registration_params = RegistrationParams::new_system_automation_task(
        entry_function.clone(),
        expiry_time,
        max_gas_amount,
        aux_data.clone(),
        Some(priority),
    );
    let address = AccountAddress::random();
    let parent_hash = HashValue::random();
    let mut features = Features::default();
    let serialized = registration_params.serialized_args_with_sender_and_parent_hash(
        address,
        parent_hash.to_vec(),
        &features,
    );
    // 4 params + address and parent hash
    assert_eq!(serialized.len(), 6);
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
    // ParentHash
    let v_parent_hash_bytes = bcs::from_bytes::<Vec<u8>>(&serialized[4]).unwrap();
    let v_parent_hash = HashValue::from_slice(&v_parent_hash_bytes).unwrap();
    assert_eq!(parent_hash, v_parent_hash);
    // AuxData
    let v_aux_data = bcs::from_bytes::<Vec<Vec<u8>>>(&serialized[5]).unwrap();
    assert_eq!(expected_aux_data, v_aux_data);

    let registration_params = RegistrationParams::new_system_automation_task(
        entry_function.clone(),
        expiry_time,
        max_gas_amount,
        aux_data.clone(),
        None,
    );
    let expected_aux_data = vec![vec![AutomationTaskType::System as u8], vec![], vec![1u8, 1, 2, 3]];
    let serialized = registration_params.serialized_args_with_sender_and_parent_hash(
        address,
        parent_hash.to_vec(),
        &features,
    );
    // 4 params + address and parent hash
    assert_eq!(serialized.len(), 6);
    // AuxData
    let v_aux_data = bcs::from_bytes::<Vec<Vec<u8>>>(&serialized[5]).unwrap();
    assert_eq!(expected_aux_data, v_aux_data);

    features.disable(FeatureFlag::SUPRA_AUTOMATION_V2);
    let serialized = registration_params.serialized_args_with_sender_and_parent_hash(
        address,
        parent_hash.to_vec(),
        &features,
    );
    // 4 params + address and parent hash
    assert_eq!(serialized.len(), 6);
    // AuxData if feature is disabled then aux data is not extended
    let v_aux_data = bcs::from_bytes::<Vec<Vec<u8>>>(&serialized[5]).unwrap();
    assert_eq!(aux_data, v_aux_data);
}

#[test]
fn automation_task_metadata_type_priority_expansion() {
    let MemberId {
        module_id,
        member_id,
    } = MemberId::from_str("0x1::timestamp::now_seconds").unwrap();
    let entry_function = EntryFunction::new(module_id, member_id, vec![], vec![]);
    let address = AccountAddress::random();
    let parent_hash = HashValue::random();
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
        task_type: Default::default(),
        priority: Default::default(),
    };

    // Empty aux data leads to type to be User, priority task-index
    let task_type = task_meta.get_task_type().unwrap();
    let task_priority = task_meta.get_task_priority().unwrap();
    assert_eq!(task_type, AutomationTaskType::User);
    assert_eq!(task_priority, task_meta.id);

    // Aux data with only type info, results with priority equal to task-index
    let task_meta_with_valid_type_aux = AutomationTaskMetaData {
        aux_data: vec![vec![AutomationTaskType::System as u8]],
        task_type: Default::default(),
        priority: Default::default(),
        ..task_meta.clone()
    };
    let task_type = task_meta_with_valid_type_aux.get_task_type().unwrap();
    let task_priority = task_meta_with_valid_type_aux.get_task_priority().unwrap();
    assert_eq!(task_type, AutomationTaskType::System);
    assert_eq!(task_priority, task_meta.id);

    // Aux data with type info, and valid priority results with specified priority and type
    let task_meta_with_valid_type_priority = AutomationTaskMetaData {
        aux_data: vec![vec![AutomationTaskType::System as u8], bcs::to_bytes(&42u64).unwrap()],
        task_type: Default::default(),
        priority: Default::default(),
        ..task_meta.clone()
    };
    let task_type = task_meta_with_valid_type_priority.get_task_type().unwrap();
    let task_priority = task_meta_with_valid_type_priority
        .get_task_priority()
        .unwrap();
    assert_eq!(task_type, AutomationTaskType::System);
    assert_eq!(task_priority, 42);

    // Aux data with invalid type info, and valid priority results with specified valid priority and no type
    let task_meta_with_invalid_type_and_valid_priority = AutomationTaskMetaData {
        aux_data: vec![vec![4], bcs::to_bytes(&24u64).unwrap()],
        task_type: Default::default(),
        priority: Default::default(),
        ..task_meta.clone()
    };
    assert!(task_meta_with_invalid_type_and_valid_priority
        .get_task_type()
        .is_none());
    // Double check that result is all the same
    assert!(task_meta_with_invalid_type_and_valid_priority
        .get_task_type()
        .is_none());

    let task_priority = task_meta_with_invalid_type_and_valid_priority
        .get_task_priority()
        .unwrap();
    assert_eq!(task_priority, 24);

    // Aux data with invalid type info and priority results with specified no priority and no type
    let task_meta_with_invalid_type_and_valid_priority = AutomationTaskMetaData {
        aux_data: vec![vec![4; 2], vec![1, 2, 3]],
        task_type: Default::default(),
        priority: Default::default(),
        ..task_meta.clone()
    };
    assert!(task_meta_with_invalid_type_and_valid_priority
        .get_task_type()
        .is_none());
    // Double check that result is all the same
    assert!(task_meta_with_invalid_type_and_valid_priority
        .get_task_type()
        .is_none());

    assert!(task_meta_with_invalid_type_and_valid_priority
        .get_task_priority()
        .is_none());
    // Double check that result is all the same
    assert!(task_meta_with_invalid_type_and_valid_priority
        .get_task_priority()
        .is_none());
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
        task_type: Default::default(),
        priority: Default::default(),
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
    } = task_meta_valid.clone();
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
    assert_eq!(builder.task_type, Some(AutomationTaskType::User));
    assert_eq!(builder.task_priority, Some(task_meta_valid.id));

    // Check builder construction when type is specified but not priority
    let task_meta_with_valid_type = AutomationTaskMetaData {
        aux_data: vec![vec![AutomationTaskType::System as u8]],
        ..task_meta_valid.clone()
    };
    let builder = AutomatedTransactionBuilder::try_from(task_meta_with_valid_type).unwrap();
    assert_eq!(builder.task_type, Some(AutomationTaskType::System));
    assert_eq!(builder.task_priority, Some(task_meta_valid.id));

    // Check builder construction when type is specified and priority is invalid.
    let task_meta_with_valid_type_and_none_priority = AutomationTaskMetaData {
        aux_data: vec![vec![1u8], vec![]],
        ..task_meta_valid.clone()
    };
    let builder =
        AutomatedTransactionBuilder::try_from(task_meta_with_valid_type_and_none_priority);
    assert!(builder.is_err());

    // Check builder construction when type is specified and priority is valid data.
    let task_meta_with_valid_type_and_priority = AutomationTaskMetaData {
        aux_data: vec![vec![AutomationTaskType::System as u8], bcs::to_bytes(&45u64).unwrap()],
        ..task_meta_valid.clone()
    };
    let builder =
        AutomatedTransactionBuilder::try_from(task_meta_with_valid_type_and_priority).unwrap();
    assert_eq!(builder.task_type, Some(AutomationTaskType::System));
    assert_eq!(builder.task_priority, Some(45));

    // Check builder construction when invalid type is specified
    let task_meta_with_invalid_type = AutomationTaskMetaData {
        aux_data: vec![vec![3u8]],
        ..task_meta_valid.clone()
    };
    let builder = AutomatedTransactionBuilder::try_from(task_meta_with_invalid_type);
    assert!(builder.is_err());

    // Check builder construction when invalid type is specified
    let task_meta_with_invalid_type = AutomationTaskMetaData {
        aux_data: vec![vec![1u8; 2]],
        ..task_meta_valid
    };
    let builder = AutomatedTransactionBuilder::try_from(task_meta_with_invalid_type);
    assert!(builder.is_err());
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
        task_type: Default::default(),
        priority: Default::default(),
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
            assert_eq!(*txn.task_type(), AutomationTaskType::User);
            assert_eq!(*txn.priority(), task_meta.id);
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

#[test]
fn automated_transaction_ordering() {
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
        task_type: Default::default(),
        priority: Default::default(),
    };

    let builder = AutomatedTransactionBuilder::try_from(task_meta.clone())
        .unwrap()
        .with_chain_id(chain_id)
        .with_gas_unit_price(15)
        .with_block_height(5);

    let BuilderResult::Success(user_auto_txn_100) = builder
        .clone()
        .with_task_type(AutomationTaskType::User)
        .with_task_priority(100)
        .build()
    else {
        panic!("Expected successful result");
    };
    let BuilderResult::Success(user_auto_txn_200) = builder
        .clone()
        .with_task_type(AutomationTaskType::User)
        .with_task_priority(200)
        .build()
    else {
        panic!("Expected successful result");
    };
    let BuilderResult::Success(system_auto_txn_100) = builder
        .clone()
        .with_task_type(AutomationTaskType::System)
        .with_task_priority(100)
        .build()
    else {
        panic!("Expected successful result");
    };
    let BuilderResult::Success(system_auto_txn_200) = builder
        .clone()
        .with_task_type(AutomationTaskType::System)
        .with_task_priority(200)
        .build()
    else {
        panic!("Expected successful result");
    };
    let BuilderResult::Success(system_auto_txn_300) = builder
        .clone()
        .with_task_type(AutomationTaskType::System)
        .with_task_priority(200)
        .build()
    else {
        panic!("Expected successful result");
    };

    let expected_auto_txns = vec![
        user_auto_txn_100.clone(),
        user_auto_txn_200.clone(),
        system_auto_txn_100.clone(),
        system_auto_txn_200.clone(),
        system_auto_txn_300.clone(),
    ];
    let mut auto_txns = vec![
        system_auto_txn_100,
        system_auto_txn_300,
        user_auto_txn_200,
        system_auto_txn_200,
        user_auto_txn_100,
    ];
    auto_txns.sort();
    assert_eq!(auto_txns, expected_auto_txns);
}

#[test]
fn check_automation_registry_record() {
    let r_builder = AutomationRegistryRecordBuilder::new(1);
    assert_eq!(r_builder.task_range(), (u64::MAX, u64::MAX));
    assert!(r_builder.clone().build().is_err());

    let r_builder = r_builder.with_record_index(2);
    assert_eq!(r_builder.task_range(), (u64::MAX, u64::MAX));
    assert!(r_builder.clone().build().is_err());

    let r_builder = r_builder.with_block_height(42);
    assert_eq!(r_builder.task_range(), (u64::MAX, u64::MAX));
    assert!(r_builder.clone().build().is_err());

    let action_tasks = vec![12, 36, 45];
    let action = AutomationRegistryAction::process(action_tasks.clone());

    let r_builder = r_builder.with_action(action);
    assert_eq!(r_builder.task_count(), 3);
    let task_indexes = r_builder.clone().into_task_indexes();
    assert_eq!(task_indexes, action_tasks);
    assert_eq!(r_builder.task_range(), (12, 45));

    let record = r_builder.clone().build().unwrap();
    assert_eq!(record.index(), 2);
    assert_eq!(record.cycle_id(), 1);
    assert_eq!(record.action().task_range(), (12, 45));

    let split_builders = r_builder.split();
    assert_eq!(split_builders.len(), 3);

    // Fails as the record index is reset, but cycle_id and block height is reserved
    split_builders.iter().for_each(|builder| {
        assert!(builder.clone().build().is_err());
    });
    split_builders
        .into_iter()
        .enumerate()
        .for_each(|(i, builder)| {
            let builder = builder.with_record_index(i as u64);
            let single_task_record = builder.build().unwrap();
            assert_eq!(single_task_record.cycle_id(), record.cycle_id());
            assert_eq!(single_task_record.block_height(), record.block_height());
            let task_range = single_task_record.action().task_range();
            assert_eq!(task_range.0, task_range.1);
        });

    let mut task_indexes = vec![50, 68, 12, 36, 45];
    let r_builder = AutomationRegistryRecordBuilder::new(2)
        .with_action(AutomationRegistryAction::process(task_indexes.clone()));
    assert_eq!(r_builder.task_range(), (12, 68));

    let mut split_builders = r_builder.split();
    split_builders.sort_by_key(|b| b.task_range());
    task_indexes.sort();
    split_builders
        .into_iter()
        .enumerate()
        .for_each(|(idx, builder)| {
            let task_index = task_indexes[idx];
            assert_eq!(builder.task_range(), (task_index, task_index));
        });
}
