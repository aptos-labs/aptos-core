// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::peer_to_peer::{check_and_apply_transfer_output, create_cyclic_transfers};
use compiler::Compiler;
use diem_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, Uniform};
use diem_framework_releases::current_modules;
use diem_parallel_executor::errors::Error;
use diem_types::{
    block_metadata::BlockMetadata,
    on_chain_config::{OnChainConfig, ParallelExecutionConfig, ValidatorSet},
    transaction::{
        authenticator::AuthenticationKey, Script, Transaction, TransactionArgument,
        TransactionStatus, WriteSetPayload,
    },
    vm_status::{KeptVMStatus, StatusCode},
};
use diem_vm::{parallel_executor::ParallelDiemVM, read_write_set_analysis::add_on_functions_list};
use diem_writeset_generator::encode_initialize_parallel_execution;
use language_e2e_tests::{account, common_transactions::rotate_key_txn, executor::FakeExecutor};
use read_write_set::analyze;

#[test]
fn peer_to_peer_with_prologue_parallel() {
    let mut executor = FakeExecutor::from_fresh_genesis();
    let account_size = 1000usize;
    let initial_balance = 2_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, initial_seq_num);

    // set up the transactions
    let transfer_amount = 1_000;

    // insert a block prologue transaction
    let (txns_info, transfer_txns) = create_cyclic_transfers(&executor, &accounts, transfer_amount);

    let analyze_result = analyze(current_modules().iter())
        .unwrap()
        .normalize_all_scripts(add_on_functions_list());

    let mut txns = transfer_txns
        .into_iter()
        .map(Transaction::UserTransaction)
        .collect::<Vec<_>>();
    let validator_set = ValidatorSet::fetch_config(executor.get_state_view())
        .expect("Unable to retrieve the validator set from storage");
    let new_block = BlockMetadata::new(
        HashValue::zero(),
        0,
        1,
        vec![],
        *validator_set.payload()[0].account_address(),
    );

    txns.insert(0, Transaction::BlockMetadata(new_block));

    let (mut results, parallel_status) =
        ParallelDiemVM::execute_block(&analyze_result, txns, executor.get_state_view()).unwrap();

    assert!(parallel_status.is_none());

    results.remove(0);

    check_and_apply_transfer_output(&mut executor, &txns_info, &results)
}

#[test]
fn rotate_ed25519_key() {
    let balance = 1_000_000;
    let mut executor = FakeExecutor::from_fresh_genesis();

    // create and publish sender
    let mut sender = executor.create_raw_account_data(balance, 10);
    executor.add_account_data(&sender);

    let privkey = Ed25519PrivateKey::generate_for_testing();
    let pubkey = privkey.public_key();
    let new_key_hash = AuthenticationKey::ed25519(&pubkey).to_vec();
    let txn = rotate_key_txn(sender.account(), new_key_hash.clone(), 10);

    let analyze_result = analyze(current_modules().iter())
        .unwrap()
        .normalize_all_scripts(add_on_functions_list());

    // execute transaction
    let (mut results, parallel_status) = ParallelDiemVM::execute_block(
        &analyze_result,
        vec![Transaction::UserTransaction(txn)],
        executor.get_state_view(),
    )
    .unwrap();

    assert!(parallel_status.is_none());

    let output = results.pop().unwrap();
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(KeptVMStatus::Executed),
    );
    executor.apply_write_set(output.write_set());

    // Check that numbers in store are correct.
    let updated_sender = executor
        .read_account_resource(sender.account())
        .expect("sender must exist");
    let updated_sender_balance = executor
        .read_balance_resource(sender.account(), account::xus_currency_code())
        .expect("sender balance must exist");
    assert_eq!(new_key_hash, updated_sender.authentication_key().to_vec());
    assert_eq!(balance, updated_sender_balance.coin());
    assert_eq!(11, updated_sender.sequence_number());

    // Check that transactions cannot be sent with the old key any more.
    let old_key_txn = rotate_key_txn(sender.account(), vec![], 11);
    let old_key_output = &executor.execute_transaction(old_key_txn);
    assert_eq!(
        old_key_output.status(),
        &TransactionStatus::Discard(StatusCode::INVALID_AUTH_KEY),
    );

    // Check that transactions can be sent with the new key.
    sender.rotate_key(privkey, pubkey);
    let new_key_txn = rotate_key_txn(sender.account(), new_key_hash, 11);
    let new_key_output = &executor.execute_transaction(new_key_txn);
    assert_eq!(
        new_key_output.status(),
        &TransactionStatus::Keep(KeptVMStatus::Executed),
    );
}

#[test]
fn parallel_execution_config() {
    let mut executor = FakeExecutor::from_fresh_genesis();
    let account_size = 1000usize;
    let initial_balance = 2_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, initial_seq_num);

    // set up the transactions
    let transfer_amount = 1_000;

    // insert a block prologue transaction
    let (txns_info, transfer_txns) = create_cyclic_transfers(&executor, &accounts, transfer_amount);

    executor.enable_parallel_execution();

    let outputs = executor.execute_block(transfer_txns).unwrap();

    check_and_apply_transfer_output(&mut executor, &txns_info, &outputs);

    executor.disable_parallel_execution();

    assert_eq!(
        ParallelExecutionConfig::fetch_config(executor.get_state_view()),
        Some(ParallelExecutionConfig {
            read_write_analysis_result: None,
        })
    );
}

#[test]
fn parallel_config_sequential_check() {
    let mut executor = FakeExecutor::from_fresh_genesis();

    let diem_root = account::Account::new_diem_root();
    let seq_num = executor
        .read_account_resource_at_address(diem_root.address())
        .unwrap()
        .sequence_number();

    let txn = diem_root
        .transaction()
        .write_set(encode_initialize_parallel_execution())
        .sequence_number(seq_num)
        .sign();

    let txns = vec![Transaction::UserTransaction(txn)];

    let analyze_result = analyze(current_modules().iter())
        .unwrap()
        .normalize_all_scripts(add_on_functions_list());
    let (mut results, parallel_status) =
        ParallelDiemVM::execute_block(&analyze_result, txns, executor.get_state_view()).unwrap();

    let output = results.pop().unwrap();

    // Making sure that an infer error is triggered and the transaction is executed in sequential mode.
    match parallel_status {
        Some(Error::InferencerError) => (),
        _ => panic!(
            "Unexpected parallel execution status, expected sequential mode due to inference error"
        ),
    }

    assert!(results.is_empty());

    executor.apply_write_set(output.write_set());
    assert_eq!(
        ParallelExecutionConfig::fetch_config(executor.get_state_view()),
        Some(ParallelExecutionConfig {
            read_write_analysis_result: None,
        })
    );
}

#[test]
fn parallel_execution_with_bad_config() {
    let mut executor = FakeExecutor::from_fresh_genesis();
    let account_size = 1000usize;
    let initial_balance = 2_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, initial_seq_num);

    // set up the transactions
    let transfer_amount = 1_000;

    // insert a block prologue transaction
    let (txns_info, transfer_txns) = create_cyclic_transfers(&executor, &accounts, transfer_amount);

    let diem_root = account::Account::new_diem_root();
    let seq_num = executor
        .read_account_resource_at_address(diem_root.address())
        .unwrap()
        .sequence_number();

    // Enable parallel execution with a malformed config

    let script_body = {
        let code = r#"
import 0x1.ParallelExecutionConfig;

main(dr_account: signer, account: signer, payload: vector<u8>) {
  ParallelExecutionConfig.enable_parallel_execution_with_config(&dr_account, move(payload));
  return;
}
"#;

        let compiler = Compiler {
            deps: diem_framework_releases::current_modules().iter().collect(),
        };
        compiler.into_script_blob(code).expect("Failed to compile")
    };

    let txn = diem_root
        .transaction()
        .write_set(WriteSetPayload::Script {
            script: Script::new(
                script_body,
                vec![],
                vec![TransactionArgument::U8Vector(vec![])],
            ),
            execute_as: *diem_root.address(),
        })
        .sequence_number(seq_num)
        .sign();
    executor.execute_and_apply(txn);

    // Make sure transactions can still be processed correctly in sequential mode.

    let outputs = executor.execute_block(transfer_txns).unwrap();

    check_and_apply_transfer_output(&mut executor, &txns_info, &outputs);
}
