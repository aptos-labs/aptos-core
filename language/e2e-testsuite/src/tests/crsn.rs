// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
use diem_transaction_builder::stdlib;
use diem_types::{account_config, vm_status::StatusCode};
use language_e2e_tests::{
    account::Account, compile::compile_script, current_function_name, executor::FakeExecutor,
};

// The CRSN size used throughout the tests
const K: u64 = 10;

fn init(executor: &mut FakeExecutor) {
    let dr = Account::new_diem_root();
    let program = r#"
        import 0x1.CRSN;
        main(account: signer) {
          CRSN.allow_crsns(&account);
          return;
        }
    "#;
    let script = compile_script(program, vec![]);
    let txn = dr.transaction().script(script).sequence_number(1).sign();
    executor.execute_and_apply(txn);
}

#[test]
fn can_opt_in_to_crsn() {
    let mut executor = FakeExecutor::from_genesis_file();
    // set the diem version back
    let sender = executor.create_raw_account_data(1_000_010, 0);
    executor.add_account_data(&sender);
    executor.set_golden_file(current_function_name!());
    init(&mut executor);

    // This should apply
    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_opt_in_to_crsn_script_function(K))
        .sequence_number(0)
        .sign();
    executor.execute_and_apply(txn);

    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_peer_to_peer_with_metadata_script_function(
            account_config::xus_tag(),
            *sender.address(),
            100,
            vec![],
            vec![],
        ))
        .sequence_number(1)
        .sign();
    executor.execute_and_apply(txn);
}

#[test]
fn crsns_prevent_replay_window_shift() {
    let mut executor = FakeExecutor::from_genesis_file();
    // set the diem version back
    let sender = executor.create_raw_account_data(1_000_010, 0);
    executor.add_account_data(&sender);
    executor.set_golden_file(current_function_name!());
    init(&mut executor);

    // This should apply
    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_opt_in_to_crsn_script_function(K))
        .sequence_number(0)
        .sign();
    executor.execute_and_apply(txn);

    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_peer_to_peer_with_metadata_script_function(
            account_config::xus_tag(),
            *sender.address(),
            100,
            vec![],
            vec![],
        ))
        .sequence_number(1)
        .sign();
    executor.execute_and_apply(txn.clone());
    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status().status().unwrap_err(),
        StatusCode::SEQUENCE_NONCE_INVALID
    );
}

#[test]
fn crsns_prevent_replay_no_window_shift() {
    let mut executor = FakeExecutor::from_genesis_file();
    // set the diem version back
    let sender = executor.create_raw_account_data(1_000_010, 0);
    executor.add_account_data(&sender);
    executor.set_golden_file(current_function_name!());
    init(&mut executor);

    // This should apply
    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_opt_in_to_crsn_script_function(K))
        .sequence_number(0)
        .sign();
    executor.execute_and_apply(txn);

    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_peer_to_peer_with_metadata_script_function(
            account_config::xus_tag(),
            *sender.address(),
            100,
            vec![],
            vec![],
        ))
        .sequence_number(10)
        .sign();
    executor.execute_and_apply(txn.clone());

    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status().status().unwrap_err(),
        StatusCode::SEQUENCE_NONCE_INVALID
    );
}

#[test]
fn crsns_can_be_executed_out_of_order() {
    let mut executor = FakeExecutor::from_genesis_file();
    // set the diem version back
    let sender = executor.create_raw_account_data(1_000_010, 0);
    executor.add_account_data(&sender);
    executor.set_golden_file(current_function_name!());
    init(&mut executor);

    // This should apply
    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_opt_in_to_crsn_script_function(K))
        .sequence_number(0)
        .sign();
    executor.execute_and_apply(txn);

    let mut txns = Vec::new();

    // worst-case scenario for out-of-order execution
    for i in 0..K {
        let txn = sender
            .account()
            .transaction()
            .payload(stdlib::encode_peer_to_peer_with_metadata_script_function(
                account_config::xus_tag(),
                *sender.address(),
                100,
                vec![],
                vec![],
            ))
            .sequence_number(K - i)
            .sign();
        txns.push(txn);
    }

    for output in executor.execute_block_and_keep_vm_status(txns).unwrap() {
        assert_eq!(output.0.status_code(), StatusCode::EXECUTED);
    }
}

#[test]
fn force_expiration_of_crsns() {
    let mut executor = FakeExecutor::from_genesis_file();
    // set the diem version back
    let sender = executor.create_raw_account_data(1_000_010, 0);
    executor.add_account_data(&sender);
    executor.set_golden_file(current_function_name!());
    init(&mut executor);

    // This should apply
    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_opt_in_to_crsn_script_function(K))
        .sequence_number(0)
        .sign();
    executor.execute_and_apply(txn);

    // worst-case scenario for out-of-order execution
    for i in K / 2..K {
        let txn = sender
            .account()
            .transaction()
            .payload(stdlib::encode_peer_to_peer_with_metadata_script_function(
                account_config::xus_tag(),
                *sender.address(),
                100,
                vec![],
                vec![],
            ))
            .sequence_number(i)
            .sign();
        executor.execute_and_apply(txn);
    }

    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_peer_to_peer_with_metadata_script_function(
            account_config::xus_tag(),
            *sender.address(),
            100,
            vec![],
            vec![],
        ))
        .sequence_number(K + 1)
        .sign();
    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status().status().unwrap_err(),
        StatusCode::SEQUENCE_NONCE_INVALID
    );

    let txn = sender
        .account()
        .transaction()
        .payload(stdlib::encode_force_expire_script_function(2 * K))
        .sequence_number(2)
        .sign();
    let output = executor.execute_and_apply(txn);

    // Make sure a force shift event is emitted, that we can deserialize it to the event Rust struct
    // and that it is what we expect
    let x = &output.events()[0];
    let force_shift =
        account_config::force_shift::ForceShiftEvent::try_from_bytes(x.event_data()).unwrap();
    assert_eq!(force_shift.current_min_nonce(), 1);
    assert_eq!(force_shift.shift_amount(), 2 * K);
    assert_eq!(
        force_shift.bits_at_shift(),
        vec![false, false, false, false, true, true, true, true, true, false]
    );

    let mut txns = Vec::new();

    // Check that the old range is expired
    for i in 0..2 * K + 1 {
        let txn = sender
            .account()
            .transaction()
            .payload(stdlib::encode_peer_to_peer_with_metadata_script_function(
                account_config::xus_tag(),
                *sender.address(),
                100,
                vec![],
                vec![],
            ))
            .sequence_number(i)
            .sign();
        txns.push(txn);
    }

    for output in executor.execute_block_and_keep_vm_status(txns).unwrap() {
        assert_eq!(output.0.status_code(), StatusCode::SEQUENCE_NONCE_INVALID);
    }

    let mut txns = Vec::new();

    // and that the new range works
    for i in 0..K {
        let txn = sender
            .account()
            .transaction()
            .payload(stdlib::encode_peer_to_peer_with_metadata_script_function(
                account_config::xus_tag(),
                *sender.address(),
                100,
                vec![],
                vec![],
            ))
            .sequence_number(3 * K - i)
            .sign();

        txns.push(txn);
    }

    for output in executor.execute_block_and_keep_vm_status(txns).unwrap() {
        assert_eq!(output.0.status_code(), StatusCode::EXECUTED);
    }
}
