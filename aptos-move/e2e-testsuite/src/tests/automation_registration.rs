// Copyright (c) 2024 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::tests::vm_viewer::to_view_function;
use aptos_cached_packages::{aptos_framework_sdk_builder, aptos_stdlib};
use aptos_language_e2e_tests::{
    account::{Account, AccountData},
    data_store::FakeDataStore,
    executor::FakeExecutor,
};
use aptos_types::account_address::create_multisig_account_address;
use aptos_types::transaction::automation::Priority;
use aptos_types::transaction::{ExecutionError, Multisig, MultisigTransactionPayload};
use aptos_types::{
    on_chain_config::{
        AutomationCycleDetails, AutomationCycleInfo, AutomationCycleState, FeatureFlag,
        OnChainConfig,
    },
    transaction::{
        automation::{
            AutomationRegistryAction, AutomationRegistryRecord, AutomationTaskMetaData,
            RegistrationParams,
        },
        EntryFunction, ExecutionStatus, SignedTransaction, Transaction, TransactionOutput,
        TransactionPayload, TransactionStatus,
    },
};
use aptos_vm::aptos_vm_viewer::AptosVMViewer;
use move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveValue},
    vm_status::StatusCode,
};
use std::{
    ops::{Deref, DerefMut},
    time::Instant,
};

use serde::{Serialize, Deserialize};
use aptos_types::contract_event::ContractEvent;
use move_core_types::vm_status::StatusCode::FEATURE_UNDER_GATING;

const TIMESTAMP_NOW_SECONDS: &str = "0x1::timestamp::now_seconds";
const ACCOUNT_BALANCE: &str = "0x1::coin::balance";
const SUPRA_COIN: &str = "0x1::supra_coin::SupraCoin";
const ACCOUNT_SEQ_NUM: &str = "0x1::account::get_sequence_number";
const AUTOMATION_NEXT_TASK_ID: &str = "0x1::automation_registry::get_next_task_index";
const AUTOMATION_TASK_DETAILS: &str = "0x1::automation_registry::get_task_details";
const AUTOMATION_TASK_DETAILS_BULK: &str = "0x1::automation_registry::get_task_details_bulk";
const AUTOMATION_CYCLE_INFO: &str = "0x1::automation_registry::get_cycle_info";
const HAS_SENDER_ACTIVE_TASK_WITH_ID: &str =
    "0x1::automation_registry::has_sender_active_task_with_id";
const GET_TASK_IDS: &str = "0x1::automation_registry::get_task_ids";

struct MultisigAccountData {
    multisig_address: AccountAddress,
    owners: Vec<AccountData>,
}

impl MultisigAccountData {
    fn vote_txn(&self, account_index: usize, txn_idx: u64, seq_num: u64) -> SignedTransaction {
        self.owners[account_index]
            .account()
            .transaction()
            .max_gas_amount(1000)
            .gas_unit_price(0)
            .sequence_number(seq_num)
            .payload(aptos_stdlib::multisig_account_vote_transaction(
                self.multisig_address,
                txn_idx,
                true,
            ))
            .sign()
    }
}

pub(crate) struct AutomationRegistrationTestContext {
    executor: FakeExecutor,
    txn_sender: AccountData,
    multisig_account_data: MultisigAccountData,
}

impl AutomationRegistrationTestContext {
    pub(crate) fn sender_account_data(&self) -> &AccountData {
        &self.txn_sender
    }

    pub(crate) fn sender_account_address(&self) -> AccountAddress {
        *self.txn_sender.address()
    }
}

impl AutomationRegistrationTestContext {
    pub(crate) fn new() -> Self {
        let mut executor = FakeExecutor::from_head_genesis();
        let mut root = Account::new_aptos_root();
        let (private_key, public_key) = aptos_vm_genesis::GENESIS_KEYPAIR.clone();
        root.rotate_key(private_key, public_key);

        // Prepare automation registration transaction sender
        let txn_sender = executor.create_raw_account_data(100_000_000_000, 0);
        executor.add_account_data(&txn_sender);

        let multisig_account_data = Self::create_multisig_account_data(&mut executor);

        Self {
            executor,
            txn_sender,
            multisig_account_data,
        }
    }

    fn create_multisig_account_data(executor: &mut FakeExecutor) -> MultisigAccountData {
        // Prepare multisig_account for system task registration
        let multisig_owner1 = executor.create_raw_account_data(1_000_000_000, 0);
        let multisig_owner2 = executor.create_raw_account_data(1_000_000_000, 0);
        executor.add_account_data(&multisig_owner1);
        executor.add_account_data(&multisig_owner2);
        let multisig_address = create_multisig_account_address(
            *multisig_owner1.address(),
            multisig_owner1.sequence_number(),
        );
        let create_multisig_payload = aptos_stdlib::multisig_account_create_with_owners(
            vec![*multisig_owner2.address()],
            1,
            vec![],
            vec![],
            u64::MAX,
        );
        let account_create_txn = multisig_owner1
            .account()
            .transaction()
            .max_gas_amount(1_000_000)
            .gas_unit_price(100)
            .payload(create_multisig_payload)
            .sequence_number(0)
            .sign();
        executor.execute_and_apply(account_create_txn);

        let transfer_txn = multisig_owner1
            .account()
            .transaction()
            .max_gas_amount(1000)
            .payload(aptos_stdlib::supra_account_transfer(
                multisig_address,
                10_000_000,
            ))
            .sequence_number(1)
            .sign();
        executor.execute_and_apply(transfer_txn);
        MultisigAccountData {
            multisig_address,
            owners: vec![multisig_owner1, multisig_owner2],
        }
    }

    pub(crate) fn set_supra_native_automation(&mut self, enable: bool) {
        self.toggle_feature_with_registry_reconfig(FeatureFlag::SUPRA_NATIVE_AUTOMATION, enable);
    }

    pub(crate) fn set_feature_flag(&mut self, flag: FeatureFlag, enable: bool) {
        let acc = AccountAddress::ONE;
        let flag_value = [flag].into_iter().map(|f| f as u64).collect::<Vec<_>>();
        let (enabled, disabled) = if enable {
            (flag_value, vec![])
        } else {
            (vec![], flag_value)
        };
        self.executor.exec(
            "features",
            "change_feature_flags_internal",
            vec![],
            vec![
                MoveValue::Signer(acc).simple_serialize().unwrap(),
                bcs::to_bytes(&enabled).unwrap(),
                bcs::to_bytes(&disabled).unwrap(),
            ],
        );
    }

    pub(crate) fn toggle_feature_with_registry_reconfig(
        &mut self,
        flag: FeatureFlag,
        enable: bool,
    ) {
        self.set_feature_flag(flag, enable);

        // Here we `automation_registry::on_new_epoch` to have the feature flag changes reflected.
        // For some reason `supra_governance` apis to reconfigure the chain state was causing issues
        // on writing the output changeset to the storage in the test-environment
        // when the function was called twice in the same context.
        self.executor
            .exec("automation_registry", "on_new_epoch", vec![], vec![]);
    }

    pub(crate) fn create_automation_registry_transaction(
        &self,
        record_index: u64,
        cycle_id: u64,
        block_height: u64,
        task_indexes: Vec<u64>,
    ) -> AutomationRegistryRecord {
        AutomationRegistryRecord::new(
            record_index,
            cycle_id,
            block_height,
            AutomationRegistryAction::process(task_indexes),
        )
    }

    pub(crate) fn new_account_data(&mut self, amount: u64, seq_num: u64) -> AccountData {
        let new_account_data = self.create_raw_account_data(amount, seq_num);
        self.add_account_data(&new_account_data);
        new_account_data
    }

    pub(crate) fn create_automation_txn(
        &self,
        seq_num: u64,
        inner_payload: EntryFunction,
        expiry_time: u64,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap: u64,
    ) -> SignedTransaction {
        let txn_arguments = RegistrationParams::new_user_automation_task_v1(
            inner_payload,
            expiry_time,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap,
            vec![],
        );
        let automation_txn = TransactionPayload::AutomationRegistration(txn_arguments);
        self.txn_sender
            .account()
            .transaction()
            .payload(automation_txn)
            .sequence_number(seq_num)
            .gas_unit_price(1)
            .sign()
    }

    pub(crate) fn create_automation_txn_v2(
        &self,
        seq_num: u64,
        inner_payload: EntryFunction,
        expiry_time: u64,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap: u64,
        priority: Option<Priority>,
    ) -> SignedTransaction {
        let txn_arguments = RegistrationParams::new_user_automation_task_v2(
            inner_payload,
            expiry_time,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap,
            vec![],
            priority,
        );
        let automation_txn = TransactionPayload::AutomationRegistration(txn_arguments);
        self.txn_sender
            .account()
            .transaction()
            .payload(automation_txn)
            .sequence_number(seq_num)
            .gas_unit_price(1)
            .sign()
    }

    pub(crate) fn create_system_automation_task_registration_txn(
        &self,
        seq_num: u64,
        multisig: Multisig,
        without_payload: bool,
    ) -> SignedTransaction {
        let payload = if without_payload {
            Multisig {
                multisig_address: multisig.multisig_address,
                transaction_payload: None,
            }
        } else {
            multisig
        };
        let automation_txn = TransactionPayload::Multisig(payload);
        self.multisig_account_data.owners[0]
            .account()
            .transaction()
            .payload(automation_txn)
            .sequence_number(seq_num)
            .gas_unit_price(1)
            .sign()
    }

    pub(crate) fn create_system_automation_task_registration_proposal(
        &self,
        seq_num: u64,
        multisig: &Multisig,
    ) -> SignedTransaction {
        self.multisig_account_data.owners[0]
            .account()
            .transaction()
            .payload(aptos_stdlib::multisig_account_create_transaction(
                self.multisig_account_data.multisig_address,
                bcs::to_bytes(multisig.transaction_payload.as_ref().unwrap()).unwrap(),
            ))
            .sequence_number(seq_num)
            .gas_unit_price(1)
            .sign()
    }

    pub(crate) fn vote_for_multisig_txn(&mut self, account_idx: usize, txn_idx: u64, seq_num: u64) {
        let _ = self
            .executor
            .execute_and_apply_transaction(Transaction::UserTransaction(
                self.multisig_account_data
                    .vote_txn(account_idx, txn_idx, seq_num),
            ));
    }

    pub(crate) fn create_system_automation_task_registration_payload(
        &self,
        inner_payload: EntryFunction,
        expiry_time: u64,
        max_gas_amount: u64,
    ) -> Multisig {
        let txn_arguments = RegistrationParams::new_system_automation_task(
            inner_payload,
            expiry_time,
            max_gas_amount,
            vec![],
            None,
        );
        let mutlisig_txn_payload =
            MultisigTransactionPayload::AutomationRegistration(txn_arguments);
        Multisig {
            multisig_address: self.multisig_account_data.multisig_address,
            transaction_payload: Some(mutlisig_txn_payload),
        }
    }

    pub(crate) fn check_miscellaneous_output(
        output: TransactionOutput,
        expected_status_code: StatusCode,
    ) {
        match output.status() {
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(maybe_status_code)) => {
                assert_eq!(
                    maybe_status_code.as_ref().unwrap(),
                    &expected_status_code,
                    "{output:?}"
                );
            },
            _ => panic!("Unexpected transaction status: {output:?}"),
        }
    }

    pub(crate) fn check_discarded_output(
        output: TransactionOutput,
        expected_status_code: StatusCode,
    ) {
        match output.status() {
            TransactionStatus::Discard(status_code) => {
                assert_eq!(status_code, &expected_status_code, "{output:?}");
            },
            _ => panic!("Unexpected transaction status: {output:?}"),
        }
    }

    pub(crate) fn chain_time_now(&mut self) -> u64 {
        let view_output =
            self.execute_view_function(str::parse(TIMESTAMP_NOW_SECONDS).unwrap(), vec![], vec![]);
        let result = view_output.values.expect("Valid result");
        assert_eq!(result.len(), 1);
        bcs::from_bytes::<u64>(&result[0]).unwrap()
    }

    pub(crate) fn advance_chain_time_in_secs(&mut self, secs: u64) {
        let chain_current_time = self.chain_time_now();
        self.set_block_time((chain_current_time + secs) * 1_000_000);
        self.new_block()
    }

    pub(crate) fn account_balance(&mut self, account_address: AccountAddress) -> u64 {
        let view_output = self.execute_view_function(
            str::parse(ACCOUNT_BALANCE).unwrap(),
            vec![str::parse(SUPRA_COIN).unwrap()],
            vec![account_address.to_vec()],
        );
        let result = view_output.values.expect("Valid result");
        assert_eq!(result.len(), 1);
        bcs::from_bytes::<u64>(&result[0]).unwrap()
    }

    pub(crate) fn account_sequence_number(&mut self, account_address: AccountAddress) -> u64 {
        let view_output = self.execute_view_function(
            str::parse(ACCOUNT_SEQ_NUM).unwrap(),
            vec![],
            vec![account_address.to_vec()],
        );
        let result = view_output.values.expect("Valid result");
        assert_eq!(result.len(), 1);
        bcs::from_bytes::<u64>(&result[0]).unwrap()
    }

    pub(crate) fn get_next_task_index_from_registry(&mut self) -> u64 {
        let view_output = self.execute_view_function(
            str::parse(AUTOMATION_NEXT_TASK_ID).unwrap(),
            vec![],
            vec![],
        );
        let result = view_output.values.expect("Valid result");
        assert_eq!(result.len(), 1);
        bcs::from_bytes::<u64>(&result[0]).unwrap()
    }

    pub(crate) fn get_task_details(&mut self, index: u64) -> AutomationTaskMetaData {
        let view_output = self.execute_view_function(
            str::parse(AUTOMATION_TASK_DETAILS).unwrap(),
            vec![],
            vec![MoveValue::U64(index)
                .simple_serialize()
                .expect("Successful serialization")],
        );
        let result = view_output.values.expect("Valid result");
        assert!(!result.is_empty());
        bcs::from_bytes::<AutomationTaskMetaData>(&result[0])
            .expect("Successful deserialization of AutomationTaskMetaData")
    }

    pub(crate) fn get_task_details_with_vm_viewer(
        index: u64,
        vm_viewer: &AptosVMViewer<FakeDataStore>,
    ) -> AutomationTaskMetaData {
        let view_output = vm_viewer.execute_view_function(
            to_view_function(
                str::parse(AUTOMATION_TASK_DETAILS).unwrap(),
                vec![],
                vec![MoveValue::U64(index)
                    .simple_serialize()
                    .expect("Successful serialization")],
            ),
            50_000,
        );
        let result = view_output.values.expect("Valid result");
        assert!(!result.is_empty());
        bcs::from_bytes::<AutomationTaskMetaData>(&result[0])
            .expect("Successful deserialization of AutomationTaskMetaData")
    }

    pub(crate) fn get_task_details_bulk(
        indexes: Vec<u64>,
        vm_viewer: &AptosVMViewer<FakeDataStore>,
    ) -> Vec<AutomationTaskMetaData> {
        let view_output = vm_viewer.execute_view_function(
            to_view_function(
                str::parse(AUTOMATION_TASK_DETAILS_BULK).unwrap(),
                vec![],
                vec![
                    MoveValue::Vector(indexes.into_iter().map(MoveValue::U64).collect())
                        .simple_serialize()
                        .expect("Successful serialization"),
                ],
            ),
            50_000,
        );
        let result = view_output.values.expect("Valid result");
        assert!(!result.is_empty());
        bcs::from_bytes::<Vec<AutomationTaskMetaData>>(&result[0])
            .expect("Successful deserialization of Vec<AutomationTaskMetaData>")
    }

    pub(crate) fn get_cycle_info(&mut self) -> AutomationCycleInfo {
        let view_output =
            self.execute_view_function(str::parse(AUTOMATION_CYCLE_INFO).unwrap(), vec![], vec![]);
        let result = view_output.values.expect("Valid result");
        assert!(!result.is_empty());
        bcs::from_bytes::<AutomationCycleInfo>(&result[0])
            .expect("Successful deserialization of AutomationCycleInfo")
    }

    pub(crate) fn has_sender_active_task_with_id(
        &mut self,
        sender: AccountAddress,
        task_idx: u64,
    ) -> bool {
        let view_output = self.execute_view_function(
            str::parse(HAS_SENDER_ACTIVE_TASK_WITH_ID).unwrap(),
            vec![],
            serialize_values(&[MoveValue::Address(sender), MoveValue::U64(task_idx)]),
        );
        let result = view_output.values.expect("Valid result");
        assert!(!result.is_empty());
        bcs::from_bytes::<bool>(&result[0])
            .expect("Successful deserialization of AutomationCycleInfo")
    }

    pub(crate) fn get_task_ids(&mut self) -> Vec<u64> {
        let view_output =
            self.execute_view_function(str::parse(GET_TASK_IDS).unwrap(), vec![], vec![]);
        let result = view_output.values.expect("Valid result");
        assert!(!result.is_empty());
        bcs::from_bytes::<Vec<u64>>(&result[0])
            .expect("Successful deserialization of AutomationCycleInfo")
    }
}

impl Deref for AutomationRegistrationTestContext {
    type Target = FakeExecutor;

    fn deref(&self) -> &Self::Target {
        &self.executor
    }
}

impl DerefMut for AutomationRegistrationTestContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.executor
    }
}

#[test]
fn check_successful_registration() {
    // Feature flag is not enabled yet.
    let mut test_context = AutomationRegistrationTestContext::new();
    // Prepare inner-entry-function to be automated.
    let dest_account = test_context.new_account_data(0, 0);
    let inner_entry_function =
        aptos_framework_sdk_builder::supra_coin_mint(dest_account.address().clone(), 100)
            .into_entry_function();

    let automation_fee_cap = 100_000;
    let expiration_time = test_context.chain_time_now() + 4000;
    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        expiration_time,
        100,
        100,
        automation_fee_cap,
    );
    let automation_txn_2 = test_context.create_automation_txn_v2(
        1,
        inner_entry_function.clone(),
        expiration_time,
        100,
        100,
        automation_fee_cap,
        Some(32),
    );

    let sender_address = test_context.sender_account_address();
    let sender_seq_num_old = test_context.account_sequence_number(sender_address);

    // When the SUPRA_NATIVE_AUTOMATION feature flag is not enabled registration requests must fail
    // to validate. This ensures that they won't be accepted by the RPC nodes or the Mempool.
    let validation_result = test_context.validate_transaction(automation_txn.clone());
    assert_eq!(
        validation_result.status(),
        Some(StatusCode::FEATURE_UNDER_GATING)
    );

    // When the SUPRA_NATIVE_AUTOMATION feature flag is not enabled registration requests must fail
    // to execute.
    let result = test_context.execute_transaction(automation_txn.clone());
    assert!(matches!(
        result.status().status(),
        Err(StatusCode::FEATURE_UNDER_GATING)
    ));

    // enable the supra native automation, registration should succeed.
    test_context.set_supra_native_automation(true);

    let output = test_context.execute_and_apply(automation_txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "{output:?}"
    );

    // Check automation registry state.
    let next_task_id = test_context.get_next_task_index_from_registry();
    assert_eq!(next_task_id, 1);
    let sender_seq_num = test_context.account_sequence_number(sender_address);
    assert_eq!(sender_seq_num, sender_seq_num_old + 1);

    let output = test_context.execute_and_apply(automation_txn_2);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "{output:?}"
    );

    // Check automation registry state.
    let next_task_id = test_context.get_next_task_index_from_registry();
    assert_eq!(next_task_id, 2);
    let sender_seq_num = test_context.account_sequence_number(sender_address);
    assert_eq!(sender_seq_num, sender_seq_num_old + 2);
}

#[test]
fn check_invalid_automation_txn() {
    let mut test_context = AutomationRegistrationTestContext::new();
    test_context.set_supra_native_automation(true);
    // Create automation registration transaction with entry-function with invalid arguments.
    let dest_account = test_context.new_account_data(0, 0);
    let (m_id, f_id, _, _) =
        aptos_framework_sdk_builder::supra_coin_mint(dest_account.address().clone(), 100)
            .into_entry_function()
            .into_inner();
    let inner_entry_function = EntryFunction::new(m_id, f_id, vec![], vec![]);
    let automation_fee_cap = 100_000;
    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function,
        3600,
        100,
        100,
        automation_fee_cap,
    );

    let output = test_context.execute_transaction(automation_txn);
    AutomationRegistrationTestContext::check_miscellaneous_output(
        output,
        StatusCode::INVALID_AUTOMATION_INNER_PAYLOAD,
    );
}

#[test]
fn check_invalid_gas_params_of_automation_task() {
    let mut test_context = AutomationRegistrationTestContext::new();
    test_context.set_supra_native_automation(true);
    // Create automation registration transaction with entry-function with invalid arguments.
    let dest_account = test_context.new_account_data(0, 0);
    let inner_entry_function =
        aptos_framework_sdk_builder::supra_coin_mint(dest_account.address().clone(), 100)
            .into_entry_function();
    let automation_fee_cap = 100_000;
    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        14400,
        2,
        100,
        automation_fee_cap,
    );

    let output = test_context.execute_transaction(automation_txn.clone());
    AutomationRegistrationTestContext::check_discarded_output(
        output,
        StatusCode::AUTOMATION_TASK_MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,
    );
    let validation_output = test_context.validate_transaction(automation_txn);
    assert_eq!(
        validation_output.status(),
        Some(StatusCode::AUTOMATION_TASK_MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS)
    );

    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        14400,
        aptos_global_constants::MAX_GAS_AMOUNT + 1,
        100,
        automation_fee_cap,
    );

    let output = test_context.execute_transaction(automation_txn.clone());
    AutomationRegistrationTestContext::check_discarded_output(
        output,
        StatusCode::AUTOMATION_TASK_MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,
    );
    let validation_output = test_context.validate_transaction(automation_txn);
    assert_eq!(
        validation_output.status(),
        Some(StatusCode::AUTOMATION_TASK_MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND)
    );

    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        14400,
        100,
        10_000_000_001,
        automation_fee_cap,
    );

    let output = test_context.execute_transaction(automation_txn.clone());
    AutomationRegistrationTestContext::check_discarded_output(
        output,
        StatusCode::AUTOMATION_TASK_GAS_PRICE_CAP_ABOVE_MAX_BOUND,
    );
    let validation_output = test_context.validate_transaction(automation_txn.clone());
    assert_eq!(
        validation_output.status(),
        Some(StatusCode::AUTOMATION_TASK_GAS_PRICE_CAP_ABOVE_MAX_BOUND)
    );

    // Check the gas check of inner payload is skipped if feature flag is not enabled
    test_context.set_feature_flag(FeatureFlag::SUPRA_AUTOMATION_PAYLOAD_GAS_CHECK, false);

    let output = test_context.execute_transaction(automation_txn.clone());
    let status = output.status().status().unwrap();
    assert!(matches!(status, ExecutionStatus::Success), "{status:?}");
    let validation_output = test_context.validate_transaction(automation_txn);
    assert!(validation_output.status().is_none());
}

#[test]
fn check_task_retrieval_performance() {
    // Register 500 tasks
    let mut test_context = AutomationRegistrationTestContext::new();
    test_context.set_supra_native_automation(true);
    let task_count = 500;
    let expiration_time = test_context.chain_time_now() + 4000;
    for i in 0..task_count {
        // Prepare inner-entry-function to be automated.
        let dest_account = test_context.new_account_data(0, 0);
        let inner_entry_function = aptos_framework_sdk_builder::supra_coin_mint(
            dest_account.address().clone(),
            (i + 1) * 10,
        )
        .into_entry_function();

        let automation_fee_cap = 1000;
        let automation_txn = test_context.create_automation_txn(
            i,
            inner_entry_function.clone(),
            expiration_time,
            25,
            100,
            automation_fee_cap,
        );
        let output = test_context.execute_and_apply(automation_txn);
        assert_eq!(
            output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success),
            "{output:?}"
        );
    }

    let vm_viewer = AptosVMViewer::new(test_context.data_store());

    let step_by_step = Instant::now();

    for i in 0..task_count {
        AutomationRegistrationTestContext::get_task_details_with_vm_viewer(i, &vm_viewer);
    }

    println!("Step by Step load time: {:?}", step_by_step.elapsed());

    let bulk_load = Instant::now();
    let mut i = 0;
    let step: u64 = 25;
    while i < task_count {
        AutomationRegistrationTestContext::get_task_details_bulk(
            (i..i + step).collect(),
            &vm_viewer,
        );
        i = i + step;
    }
    println!("Bulk load time: {:?}", bulk_load.elapsed());
}

#[test]
fn check_automation_registry_actions_on_cycle_transition() {
    // Feature flag is not enabled yet.
    let mut test_context = AutomationRegistrationTestContext::new();
    test_context.set_supra_native_automation(true);
    // Prepare inner-entry-function to be automated.
    let dest_account = test_context.new_account_data(0, 0);
    let inner_entry_function =
        aptos_framework_sdk_builder::supra_coin_mint(dest_account.address().clone(), 100)
            .into_entry_function();

    let automation_fee_cap = 100_000;
    let expiration_time = test_context.chain_time_now() + 4000;
    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        expiration_time,
        100,
        100,
        automation_fee_cap,
    );
    // Expires in the first cycle
    let task_2_expiry_time = test_context.chain_time_now() + 1500;
    let automation_txn_2 = test_context.create_automation_txn(
        1,
        inner_entry_function.clone(),
        task_2_expiry_time,
        100,
        100,
        automation_fee_cap,
    );

    let sender_address = test_context.sender_account_address();

    test_context.execute_and_apply(automation_txn);
    test_context.execute_and_apply(automation_txn_2);

    test_context.advance_chain_time_in_secs(600);
    // Any task processing request in started state will fail
    let registry_action = test_context.create_automation_registry_transaction(0, 1, 1, vec![0]);
    let result = test_context
        .execute_tagged_transaction(Transaction::AutomationRegistryTransaction(registry_action));
    let status = result.status().status().expect("Expected execution status");
    assert!(matches!(
        status,
        ExecutionStatus::MoveAbort {
            location: _,
            code: _,
            info: _
        }
    ));

    test_context.advance_chain_time_in_secs(600);

    // Execute registry action to have tasks activated
    let cycle_info = test_context.get_cycle_info();
    assert_eq!(cycle_info.state, AutomationCycleState::FINISHED);
    let registry_action =
        test_context.create_automation_registry_transaction(0, cycle_info.index + 1, 2, vec![0, 1]);
    test_context
        .execute_and_apply_transaction(Transaction::AutomationRegistryTransaction(registry_action));
    let cycle_info = test_context.get_cycle_info();
    assert_eq!(cycle_info.state, AutomationCycleState::STARTED);
    assert!(test_context.has_sender_active_task_with_id(sender_address, 0));
    assert!(test_context.has_sender_active_task_with_id(sender_address, 1));

    // Move to next cycle
    test_context.advance_chain_time_in_secs(1200);

    // Execute registry actions to have tasks activated
    let cycle_info = test_context.get_cycle_info();
    assert_eq!(cycle_info.state, AutomationCycleState::FINISHED);
    let registry_action_for_task1 =
        test_context.create_automation_registry_transaction(0, cycle_info.index + 1, 1, vec![1]);

    let registry_action_for_task0 =
        test_context.create_automation_registry_transaction(0, cycle_info.index + 1, 1, vec![0]);

    // Check that out of order execution will fail
    let result = test_context.execute_tagged_transaction(
        Transaction::AutomationRegistryTransaction(registry_action_for_task1.clone()),
    );
    let status = result.status().status().expect("Expected execution status");

    assert!(matches!(
        status,
        ExecutionStatus::MoveAbort {
            location: _,
            code: _,
            info: _
        }
    ));

    test_context.execute_and_apply_transaction(Transaction::AutomationRegistryTransaction(
        registry_action_for_task0,
    ));
    let cycle_info = test_context.get_cycle_info();
    assert_eq!(cycle_info.state, AutomationCycleState::FINISHED);
    let cycle_details = AutomationCycleDetails::fetch_config(test_context.data_store())
        .expect("Expected a cycle details");
    let transition_state = cycle_details
        .transition_state
        .as_ref()
        .expect("Transition state");
    assert_eq!(transition_state.next_task_index_position, 1);

    test_context.execute_and_apply_transaction(Transaction::AutomationRegistryTransaction(
        registry_action_for_task1,
    ));
    let cycle_info = test_context.get_cycle_info();
    assert_eq!(cycle_info.state, AutomationCycleState::STARTED);

    // the second task is removed from registry , whereas the first task is still available
    assert!(test_context.has_sender_active_task_with_id(sender_address, 0));
    assert!(!test_context.has_sender_active_task_with_id(sender_address, 1));
    let task_ids = test_context.get_task_ids();
    assert!(task_ids.contains(&0));
    assert!(!task_ids.contains(&1));
}

#[test]
fn check_automation_registry_actions_on_cycle_suspension() {
    // Feature flag is not enabled yet.
    let mut test_context = AutomationRegistrationTestContext::new();
    test_context.set_supra_native_automation(true);
    // Prepare inner-entry-function to be automated.
    let dest_account = test_context.new_account_data(0, 0);
    let inner_entry_function =
        aptos_framework_sdk_builder::supra_coin_mint(dest_account.address().clone(), 100)
            .into_entry_function();

    let automation_fee_cap = 100_000;
    let expiration_time = test_context.chain_time_now() + 4000;
    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        expiration_time,
        100,
        100,
        automation_fee_cap,
    );

    let sender_address = test_context.sender_account_address();

    test_context.execute_and_apply(automation_txn);
    test_context.advance_chain_time_in_secs(1200);

    // Execute registry action to have tasks activated
    let cycle_info = test_context.get_cycle_info();
    assert_eq!(cycle_info.state, AutomationCycleState::FINISHED);
    let registry_action =
        test_context.create_automation_registry_transaction(0, cycle_info.index + 1, 1, vec![0]);
    test_context
        .execute_and_apply_transaction(Transaction::AutomationRegistryTransaction(registry_action));
    let cycle_info = test_context.get_cycle_info();
    assert_eq!(cycle_info.state, AutomationCycleState::STARTED);
    assert_eq!(cycle_info.start_time, 1200);
    assert!(test_context.has_sender_active_task_with_id(sender_address, 0));

    // Advance the half of the cycle and disable feature
    test_context.advance_chain_time_in_secs(600);
    test_context.set_supra_native_automation(false);

    // Execute registry action to have tasks activated
    let cycle_info = test_context.get_cycle_info();
    assert_eq!(cycle_info.state, AutomationCycleState::SUSPENDED);
    let registry_action =
        test_context.create_automation_registry_transaction(0, cycle_info.index, 2, vec![0]);
    test_context
        .execute_and_apply_transaction(Transaction::AutomationRegistryTransaction(registry_action));
    let cycle_info = test_context.get_cycle_info();
    assert_eq!(cycle_info.state, AutomationCycleState::READY);

    assert!(test_context.get_task_ids().is_empty());

    // Any task processing request in READY state will fail
    let registry_action =
        test_context.create_automation_registry_transaction(0, cycle_info.index, 2, vec![0]);
    let result = test_context
        .execute_tagged_transaction(Transaction::AutomationRegistryTransaction(registry_action));
    let status = result.status().status().expect("Expected execution status");

    assert!(matches!(
        status,
        ExecutionStatus::MoveAbort {
            location: _,
            code: _,
            info: _
        }
    ));
}

#[test]
fn check_automation_registry_actions_when_automation_cycle_disabled() {
    // Feature flag is not enabled yet.
    let mut test_context = AutomationRegistrationTestContext::new();
    test_context.set_supra_native_automation(true);
    // Disable feature and check that no automation registry action is processed.
    test_context.toggle_feature_with_registry_reconfig(FeatureFlag::SUPRA_AUTOMATION_V2, false);
    let registry_action = test_context.create_automation_registry_transaction(0, 1, 1, vec![0]);
    let result = test_context
        .execute_tagged_transaction(Transaction::AutomationRegistryTransaction(registry_action));
    assert!(matches!(
        result.status().status(),
        Err(StatusCode::FEATURE_UNDER_GATING)
    ));
}


#[derive(Clone, Debug, Serialize, Deserialize)]
struct TransactionExecutionFailed {
    multisig_address: AccountAddress,
    executor: AccountAddress,
    sequence_number: u64,
    transaction_payload: Vec<u8>,
    num_approvals: u64,
    execution_error: ExecutionError,
}

fn find_transaction_error(events: &[ContractEvent]) -> Vec<TransactionExecutionFailed> {
    events.iter().filter(|e| e.is_v2())
        .map(|e| bcs::from_bytes::<TransactionExecutionFailed>(e.event_data()))
        .filter_map(|d| d.ok())
        .collect()

}

#[test]
fn check_system_automation_task_registration() {
    // Feature flag is not enabled yet.
    let mut test_context = AutomationRegistrationTestContext::new();
    test_context.set_supra_native_automation(true);
    let inner_payload = aptos_stdlib::supra_account_transfer(
        *test_context.multisig_account_data.owners[1].address(),
        10,
    )
    .into_entry_function();
    let expiry_time = test_context.chain_time_now() + 7200;
    let multisig = test_context.create_system_automation_task_registration_payload(
        inner_payload,
        expiry_time,
        100,
    );
    let proposal_txn =
        test_context.create_system_automation_task_registration_proposal(2, &multisig);
    test_context.execute_and_apply(proposal_txn);
    test_context.vote_for_multisig_txn(0, 1, 3);
    let multisig_execute_txn =
        test_context.create_system_automation_task_registration_txn(4, multisig.clone(), false);
    let output = test_context.execute_and_apply(multisig_execute_txn);
    let failed_event = find_transaction_error(output.events());
    assert_eq!(failed_event.len(), 1);
    let expected_execution_error = ExecutionError {
        abort_location: "0000000000000000000000000000000000000000000000000000000000000001::automation_registry".to_string(),
        error_type: "MoveAbort".to_string(),
        error_code: 41,
    };
    assert_eq!(expected_execution_error, failed_event[0].execution_error);

    // When feature flag is disabled for V2 transaction execution fails
    test_context.set_feature_flag(FeatureFlag::SUPRA_AUTOMATION_V2, false);

    // Try with multisig payload specified
    let proposal_txn =
        test_context.create_system_automation_task_registration_proposal(5, &multisig);
    test_context.execute_and_apply(proposal_txn);
    test_context.vote_for_multisig_txn(0, 2, 6);
    let multisig_execute_txn =
        test_context.create_system_automation_task_registration_txn(7, multisig.clone(), false);
    // When payload is not provided validation of the multisig does only simple checks of the transaction and not inner one.
    let result = test_context.validate_transaction(multisig_execute_txn.clone());
    assert!(result.status().is_none(),);

    let result = test_context.execute_transaction(multisig_execute_txn);
    let failed_event = find_transaction_error(result.events());
    assert_eq!(failed_event.len(), 1);
    let expected_execution_error = ExecutionError {
        abort_location: "".to_string(),
        error_type: "VMError".to_string(),
        error_code: FEATURE_UNDER_GATING as u64,
    };
    assert_eq!(expected_execution_error, failed_event[0].execution_error);


    // Try without multisig payload specified
    let proposal_txn =
        test_context.create_system_automation_task_registration_proposal(7, &multisig);
    test_context.execute_and_apply(proposal_txn);
    test_context.vote_for_multisig_txn(0, 3, 8);
    let multisig_execute_txn =
        test_context.create_system_automation_task_registration_txn(9, multisig, true);

    // When payload is not provided validation of the multisig does only simple checks of the transaction and not inner one.
    let result = test_context.validate_transaction(multisig_execute_txn.clone());
    assert!(result.status().is_none(),);

    let result = test_context.execute_transaction(multisig_execute_txn);
    let failed_event = find_transaction_error(result.events());
    assert_eq!(failed_event.len(), 1);
    assert_eq!(expected_execution_error, failed_event[0].execution_error);
}
