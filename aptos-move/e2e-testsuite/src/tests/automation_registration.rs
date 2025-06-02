// Copyright (c) 2024 Supra.
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_framework_sdk_builder;
use aptos_language_e2e_tests::{
    account::{Account, AccountData},
    executor::FakeExecutor,
};
use aptos_types::{
    on_chain_config::FeatureFlag,
    transaction::{
        automation::{AutomationTaskMetaData, RegistrationParams},
        EntryFunction, ExecutionStatus, SignedTransaction, TransactionOutput, TransactionPayload,
        TransactionStatus,
    },
};
use move_core_types::{account_address::AccountAddress, value::MoveValue, vm_status::StatusCode};
use std::ops::{Deref, DerefMut};
use std::time::Instant;
use aptos_language_e2e_tests::data_store::FakeDataStore;
use aptos_vm::aptos_vm_viewer::AptosVMViewer;
use crate::tests::vm_viewer::to_view_function;

const TIMESTAMP_NOW_SECONDS: &str = "0x1::timestamp::now_seconds";
const ACCOUNT_BALANCE: &str = "0x1::coin::balance";
const SUPRA_COIN: &str = "0x1::supra_coin::SupraCoin";
const ACCOUNT_SEQ_NUM: &str = "0x1::account::get_sequence_number";
const AUTOMATION_NEXT_TASK_ID: &str = "0x1::automation_registry::get_next_task_index";
const AUTOMATION_TASK_DETAILS: &str = "0x1::automation_registry::get_task_details";
const AUTOMATION_TASK_DETAILS_BULK: &str = "0x1::automation_registry::get_task_details_bulk";

pub(crate) struct AutomationRegistrationTestContext {
    executor: FakeExecutor,
    txn_sender: AccountData,
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
        Self {
            executor,
            txn_sender,
        }
    }

    pub(crate) fn set_supra_native_automation(&mut self, enable: bool) {
        self.set_feature_flag(FeatureFlag::SUPRA_NATIVE_AUTOMATION, enable);
    }


    pub(crate) fn set_feature_flag(&mut self, flag: FeatureFlag, enable: bool) {
        let acc = AccountAddress::ONE;
        let flag_value = [flag]
            .into_iter()
            .map(|f| f as u64)
            .collect::<Vec<_>>();
        let (enabled, disabled) = if enable {
            (flag_value, vec![])
        } else {
            (vec![], flag_value)
        };
        self.executor
            .exec("features", "change_feature_flags_internal", vec![], vec![
                MoveValue::Signer(acc).simple_serialize().unwrap(),
                bcs::to_bytes(&enabled).unwrap(),
                bcs::to_bytes(&disabled).unwrap(),
            ]);
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
        aux_data: Vec<Vec<u8>>,
    ) -> SignedTransaction {
        let txn_arguments = RegistrationParams::new_v1(
            inner_payload,
            expiry_time,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap,
            aux_data,
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
        self.set_block_time(secs * 1_000_000);
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
        let view_output =
            self.execute_view_function(str::parse(ACCOUNT_SEQ_NUM).unwrap(), vec![], vec![
                account_address.to_vec(),
            ]);
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
        let view_output =
            self.execute_view_function(str::parse(AUTOMATION_TASK_DETAILS).unwrap(), vec![], vec![
                MoveValue::U64(index)
                    .simple_serialize()
                    .expect("Successful serialization"),
            ]);
        let result = view_output.values.expect("Valid result");
        assert!(!result.is_empty());
        bcs::from_bytes::<AutomationTaskMetaData>(&result[0])
            .expect("Successful deserialization of AutomationTaskMetaData")
    }

    pub(crate) fn get_task_details_with_vm_viewer(index: u64, vm_viewer: &AptosVMViewer<FakeDataStore>) -> AutomationTaskMetaData {
        let view_output =
            vm_viewer.execute_view_function(to_view_function(str::parse(AUTOMATION_TASK_DETAILS).unwrap(), vec![], vec![
                MoveValue::U64(index)
                    .simple_serialize()
                    .expect("Successful serialization"),
            ]), 50_000);
        let result = view_output.values.expect("Valid result");
        assert!(!result.is_empty());
        bcs::from_bytes::<AutomationTaskMetaData>(&result[0])
            .expect("Successful deserialization of AutomationTaskMetaData")
    }

    pub(crate) fn get_task_details_bulk(indexes: Vec<u64>, vm_viewer: &AptosVMViewer<FakeDataStore>) -> Vec<AutomationTaskMetaData> {
        let view_output =
            vm_viewer.execute_view_function(to_view_function(str::parse(AUTOMATION_TASK_DETAILS_BULK).unwrap(), vec![], vec![
                MoveValue::Vector(indexes.into_iter().map(MoveValue::U64).collect())
                    .simple_serialize()
                    .expect("Successful serialization"),
            ]), 50_000);
        let result = view_output.values.expect("Valid result");
        assert!(!result.is_empty());
        bcs::from_bytes::<Vec<AutomationTaskMetaData>>(&result[0])
            .expect("Successful deserialization of Vec<AutomationTaskMetaData>")
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
    let aux_data = Vec::new();
    let expiration_time = test_context.chain_time_now() + 4000;
    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        expiration_time,
        100,
        100,
        automation_fee_cap,
        aux_data,
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
    let aux_data = Vec::new();
    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function,
        3600,
        100,
        100,
        automation_fee_cap,
        aux_data,
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
    let aux_data = Vec::new();
    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        14400,
        2,
        100,
        automation_fee_cap,
        aux_data.clone(),
    );

    let output = test_context.execute_transaction(automation_txn.clone());
    AutomationRegistrationTestContext::check_discarded_output(
        output,
        StatusCode::AUTOMATION_TASK_MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,
    );
    let validation_output = test_context.validate_transaction(automation_txn);
    assert_eq!(validation_output.status(), Some(StatusCode::AUTOMATION_TASK_MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS));

    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        14400,
        aptos_global_constants::MAX_GAS_AMOUNT + 1,
        100,
        automation_fee_cap,
        aux_data.clone(),
    );

    let output = test_context.execute_transaction(automation_txn.clone());
    AutomationRegistrationTestContext::check_discarded_output(
        output,
        StatusCode::AUTOMATION_TASK_MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,
    );
    let validation_output = test_context.validate_transaction(automation_txn);
    assert_eq!(validation_output.status(), Some(StatusCode::AUTOMATION_TASK_MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND));

    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        14400,
        100,
        10_000_000_001,
        automation_fee_cap,
        aux_data,
    );

    let output = test_context.execute_transaction(automation_txn.clone());
    AutomationRegistrationTestContext::check_discarded_output(
        output,
        StatusCode::AUTOMATION_TASK_GAS_PRICE_CAP_ABOVE_MAX_BOUND,
    );
    let validation_output = test_context.validate_transaction(automation_txn.clone());
    assert_eq!(validation_output.status(), Some(StatusCode::AUTOMATION_TASK_GAS_PRICE_CAP_ABOVE_MAX_BOUND));

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
        let inner_entry_function =
            aptos_framework_sdk_builder::supra_coin_mint(dest_account.address().clone(), (i + 1) * 10)
                .into_entry_function();

        let automation_fee_cap = 1000;
        let aux_data = Vec::new();
        let automation_txn = test_context.create_automation_txn(
            i,
            inner_entry_function.clone(),
            expiration_time,
            25,
            100,
            automation_fee_cap,
            aux_data,
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
        AutomationRegistrationTestContext::get_task_details_bulk((i .. i + step).collect(), &vm_viewer);
        i = i + step ;
    }
    println!("Bulk load time: {:?}", bulk_load.elapsed());

}
