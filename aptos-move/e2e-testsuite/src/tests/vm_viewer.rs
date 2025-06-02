// Copyright (c) 2024 Supra.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_types::move_utils::MemberId;
use aptos_types::transaction::{ViewFunction, ViewFunctionOutput};
use aptos_vm::aptos_vm_viewer::AptosVMViewer;
use move_core_types::language_storage::TypeTag;
use std::time::Instant;
use aptos_logger::debug;

const TIMESTAMP_NOW_SECONDS: &str = "0x1::timestamp::now_seconds";
const ACCOUNT_BALANCE: &str = "0x1::coin::balance";
const ACCOUNT_SEQ_NUM: &str = "0x1::account::get_sequence_number";
const SUPRA_COIN: &str = "0x1::supra_coin::SupraCoin";

pub(crate) fn to_view_function(fn_ref: MemberId, ty_args: Vec<TypeTag>, args: Vec<Vec<u8>>) -> ViewFunction {
    ViewFunction::new(fn_ref.module_id, fn_ref.member_id, ty_args, args)
}

fn extract_view_output(output: ViewFunctionOutput) -> Vec<u8> {
    output.values.unwrap().pop().unwrap()
}
#[test]
fn test_vm_viewer() {
    let mut test_executor = FakeExecutor::from_head_genesis();
    let timestamp_now_ref: MemberId = str::parse(TIMESTAMP_NOW_SECONDS).unwrap();
    let account_seq_ref: MemberId = str::parse(ACCOUNT_SEQ_NUM).unwrap();
    let account_balance_ref: MemberId = str::parse(ACCOUNT_BALANCE).unwrap();
    let supra_coin_ty_tag: TypeTag = str::parse(SUPRA_COIN).unwrap();

    // Prepare 5 accounts with different balance
    let accounts = (1..5)
        .map(|i| {
            let account = test_executor.create_raw_account_data(100 * i, i);
            test_executor.add_account_data(&account);
            account
        })
        .collect::<Vec<_>>();
    // Query account seq number and balance using direct AptosVM one-time interface
    let one_time_ifc_time = Instant::now();
    let expected_results = accounts
        .iter()
        .map(|account| {
            let time = Instant::now();
            let timestamp = extract_view_output(test_executor.execute_view_function(
                timestamp_now_ref.clone(),
                vec![],
                vec![],
            ));
            debug!("AptosVM step: {}", time.elapsed().as_secs_f64());
            let time = Instant::now();
            let address_arg = account.address().to_vec();
            let account_balance = extract_view_output(test_executor.execute_view_function(
                account_balance_ref.clone(),
                vec![supra_coin_ty_tag.clone()],
                vec![address_arg.clone()],
            ));
            debug!("AptosVM step: {}", time.elapsed().as_secs_f64());
            let time = Instant::now();
            let account_seq_num = extract_view_output(test_executor.execute_view_function(
                account_seq_ref.clone(),
                vec![],
                vec![address_arg],
            ));
            debug!("AptosVM step: {}", time.elapsed().as_secs_f64());
            (timestamp, account_seq_num, account_balance)
        })
        .collect::<Vec<_>>();
    let one_time_ifc_time = one_time_ifc_time.elapsed().as_secs_f64();

    // Now do the same with AptosVMViewer interface
    let viewer_ifc_time = Instant::now();
    let time = Instant::now();
    let vm_viewer = AptosVMViewer::new(test_executor.data_store());
    debug!("AptosVMViewer creation time: {}", time.elapsed().as_secs_f64());
    let actual_results = accounts
        .iter()
        .map(|account| {
            let time = Instant::now();
            let timestamp = extract_view_output(vm_viewer.execute_view_function(
                to_view_function(timestamp_now_ref.clone(), vec![], vec![]),
                u64::MAX,
            ));
            debug!("AptosVMViewer step: {}", time.elapsed().as_secs_f64());
            let time = Instant::now();
            let address_arg = account.address().to_vec();
            let account_balance = extract_view_output(vm_viewer.execute_view_function(
                to_view_function(
                    account_balance_ref.clone(),
                    vec![supra_coin_ty_tag.clone()],
                    vec![address_arg.clone()],
                ),
                u64::MAX,
            ));
            debug!("AptosVMViewer step: {}", time.elapsed().as_secs_f64());
            let time = Instant::now();
            let account_seq_num = extract_view_output(vm_viewer.execute_view_function(
                to_view_function(account_seq_ref.clone(), vec![], vec![address_arg]),
                u64::MAX,
            ));
            debug!("AptosVMViewer step: {}", time.elapsed().as_secs_f64());
            (timestamp, account_seq_num, account_balance)
        })
        .collect::<Vec<_>>();
    let viewer_ifc_time = viewer_ifc_time.elapsed().as_secs_f64();
    assert_eq!(actual_results, expected_results);
    debug!("AptosVM: {one_time_ifc_time} - AptosVMViewer: {viewer_ifc_time}")
}
