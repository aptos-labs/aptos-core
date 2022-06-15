// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{bootstrap_genesis, gen_block_id, gen_ledger_info_with_sigs};
use anyhow::{anyhow, ensure, Result};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{AccountKey, LocalAccount},
};
use aptos_state_view::account_with_state_view::{AccountWithStateView, AsAccountWithStateView};
use aptos_types::{
    account_config::aptos_root_address,
    account_view::AccountView,
    chain_id::ChainId,
    event::EventKey,
    test_helpers::transaction_test_helpers::block,
    transaction::{
        Transaction, Transaction::UserTransaction, TransactionListWithProof, TransactionWithProof,
        WriteSetPayload,
    },
    trusted_state::{TrustedState, TrustedStateChange},
    waypoint::Waypoint,
};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use executor::block_executor::BlockExecutor;
use executor_types::BlockExecutorTrait;
use rand::SeedableRng;
use std::sync::Arc;

use storage_interface::{state_view::DbStateViewAtVersion, DbReaderWriter, Order};

pub fn test_execution_with_storage_impl() -> Arc<AptosDB> {
    let (genesis, validators) = vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));

    let mut genesis_account: LocalAccount = LocalAccount::new(
        aptos_root_address(),
        AccountKey::from_private_key(vm_genesis::GENESIS_KEYPAIR.0.clone()),
        0,
    );

    let path = aptos_temppath::TempPath::new();
    path.create_as_dir().unwrap();
    let (aptos_db, db, executor, waypoint) = create_db_and_executor(path.path(), &genesis_txn);

    let parent_block_id = executor.committed_block_id();
    let signer = aptos_types::validator_signer::ValidatorSigner::new(
        validators[0].data.address,
        validators[0].key.clone(),
    );

    // This generates accounts that do not overlap with genesis
    let seed = [3u8; 32];
    let mut rng = ::rand::rngs::StdRng::from_seed(seed);

    let mut account1 = LocalAccount::generate(&mut rng);
    let mut account2 = LocalAccount::generate(&mut rng);
    let account3 = LocalAccount::generate(&mut rng);
    let account4 = LocalAccount::generate(&mut rng);

    let account1_address = account1.address();
    let account2_address = account2.address();
    let account3_address = account3.address();

    let txn_factory = TransactionFactory::new(ChainId::test());

    let tx1 = genesis_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account1.public_key()));
    let tx2 = genesis_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account2.public_key()));
    let tx3 = genesis_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account3.public_key()));

    // Create account1 with 2M coins.
    let txn1 = genesis_account
        .sign_with_transaction_builder(txn_factory.mint(account1.address(), 2_000_000));
    // Create account2 with 1.2M coins.
    let txn2 = genesis_account
        .sign_with_transaction_builder(txn_factory.mint(account2.address(), 1_200_000));
    // Create account3 with 1M coins.
    let txn3 = genesis_account
        .sign_with_transaction_builder(txn_factory.mint(account3.address(), 1_000_000));

    // Transfer 20k coins from account1 to account2.
    // balance: <1.98M, 1.22M, 1M
    let txn4 =
        account1.sign_with_transaction_builder(txn_factory.transfer(account2.address(), 20_000));

    // Transfer 10k coins from account2 to account3.
    // balance: <1.98M, <1.21M, 1.01M
    let txn5 =
        account2.sign_with_transaction_builder(txn_factory.transfer(account3.address(), 10_000));

    // Transfer 70k coins from account1 to account3.
    // balance: <1.91M, <1.21M, 1.08M
    let txn6 =
        account1.sign_with_transaction_builder(txn_factory.transfer(account3.address(), 70_000));

    let block1 = block(vec![
        UserTransaction(tx1),
        UserTransaction(tx2),
        UserTransaction(tx3),
        UserTransaction(txn1),
        UserTransaction(txn2),
        UserTransaction(txn3),
        UserTransaction(txn4),
        UserTransaction(txn5),
        UserTransaction(txn6),
    ]);
    let block1_id = gen_block_id(1);

    let mut block2 = vec![];
    let block2_id = gen_block_id(2);

    // Create 14 txns transferring 10k from account1 to account3 each.
    for _ in 2..=15 {
        block2.push(UserTransaction(account1.sign_with_transaction_builder(
            txn_factory.transfer(account3.address(), 10_000),
        )));
    }
    let block2 = block(block2);

    let output1 = executor
        .execute_block((block1_id, block1.clone()), parent_block_id)
        .unwrap();
    let ledger_info_with_sigs = gen_ledger_info_with_sigs(1, &output1, block1_id, vec![&signer]);
    executor
        .commit_blocks(vec![block1_id], ledger_info_with_sigs)
        .unwrap();

    let state_proof = db.reader.get_state_proof(0).unwrap();
    let trusted_state = TrustedState::from_epoch_waypoint(waypoint);
    let trusted_state = match trusted_state.verify_and_ratchet(&state_proof) {
        Ok(TrustedStateChange::Epoch { new_state, .. }) => new_state,
        _ => panic!("unexpected state change"),
    };
    let current_version = state_proof.latest_ledger_info().version();
    assert_eq!(trusted_state.version(), 10);

    let t1 = db
        .reader
        .get_account_transaction(genesis_account.address(), 3, false, current_version)
        .unwrap();
    verify_committed_txn_status(t1.as_ref(), &block1[3]).unwrap();

    let t2 = db
        .reader
        .get_account_transaction(genesis_account.address(), 4, false, current_version)
        .unwrap();
    verify_committed_txn_status(t2.as_ref(), &block1[4]).unwrap();

    let t3 = db
        .reader
        .get_account_transaction(genesis_account.address(), 5, false, current_version)
        .unwrap();
    verify_committed_txn_status(t3.as_ref(), &block1[5]).unwrap();

    let tn = db
        .reader
        .get_account_transaction(genesis_account.address(), 6, false, current_version)
        .unwrap();
    assert!(tn.is_none());

    let t4 = db
        .reader
        .get_account_transaction(account1.address(), 0, true, current_version)
        .unwrap();
    verify_committed_txn_status(t4.as_ref(), &block1[6]).unwrap();
    // We requested the events to come back from this one, so verify that they did
    assert_eq!(t4.unwrap().events.unwrap().len(), 2);

    let t5 = db
        .reader
        .get_account_transaction(account2.address(), 0, false, current_version)
        .unwrap();
    verify_committed_txn_status(t5.as_ref(), &block1[7]).unwrap();

    let t6 = db
        .reader
        .get_account_transaction(account1.address(), 1, true, current_version)
        .unwrap();
    verify_committed_txn_status(t6.as_ref(), &block1[8]).unwrap();

    let db_state_view = db
        .reader
        .state_view_at_version(Some(current_version))
        .unwrap();
    let account1_state_view = db_state_view.as_account_with_state_view(&account1_address);
    verify_account_balance(get_account_balance(&account1_state_view), |x| {
        x == 1_910_000
    })
    .unwrap();

    let account2_state_view = db_state_view.as_account_with_state_view(&account2_address);

    verify_account_balance(get_account_balance(&account2_state_view), |x| {
        x == 1_210_000
    })
    .unwrap();

    let account3_state_view = db_state_view.as_account_with_state_view(&account3_address);

    verify_account_balance(get_account_balance(&account3_state_view), |x| {
        x == 1_080_000
    })
    .unwrap();

    let transaction_list_with_proof = db
        .reader
        .get_transactions(3, 10, current_version, false)
        .unwrap();
    verify_transactions(&transaction_list_with_proof, &block1[2..]).unwrap();

    let account1_sent_events = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account1.address(), 2),
            0,
            Order::Ascending,
            10,
        )
        .unwrap();
    assert_eq!(account1_sent_events.len(), 2);

    let account2_sent_events = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account2.address(), 2),
            0,
            Order::Ascending,
            10,
        )
        .unwrap();
    assert_eq!(account2_sent_events.len(), 1);

    let account3_sent_events = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account3.address(), 2),
            0,
            Order::Ascending,
            10,
        )
        .unwrap();
    assert_eq!(account3_sent_events.len(), 0);

    let account1_received_events = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account1.address(), 1),
            0,
            Order::Ascending,
            10,
        )
        .unwrap();
    // Account1 has one deposit event since TestCoin was minted to it.
    assert_eq!(account1_received_events.len(), 1);

    let account2_received_events = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account2.address(), 1),
            0,
            Order::Ascending,
            10,
        )
        .unwrap();
    // Account2 has two deposit events: from being minted to and from one transfer.
    assert_eq!(account2_received_events.len(), 2);

    let account3_received_events = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account3.address(), 1),
            0,
            Order::Ascending,
            10,
        )
        .unwrap();
    // Account3 has three deposit events: from being minted to and from two transfers.
    assert_eq!(account3_received_events.len(), 3);
    let account4_resource = db
        .reader
        .state_view_at_version(Some(current_version))
        .unwrap()
        .as_account_with_state_view(&account4.address())
        .get_account_resource()
        .unwrap();
    assert!(account4_resource.is_none());

    let account4_transaction = db
        .reader
        .get_account_transaction(account4.address(), 0, true, current_version)
        .unwrap();
    assert!(account4_transaction.is_none());

    let account4_sent_events = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account4.address(), 2),
            0,
            Order::Ascending,
            10,
        )
        .unwrap();
    assert!(account4_sent_events.is_empty());

    // Execute the 2nd block.
    let output2 = executor
        .execute_block((block2_id, block2.clone()), block1_id)
        .unwrap();
    let ledger_info_with_sigs = gen_ledger_info_with_sigs(1, &output2, block2_id, vec![&signer]);
    executor
        .commit_blocks(vec![block2_id], ledger_info_with_sigs)
        .unwrap();

    let state_proof = db.reader.get_state_proof(trusted_state.version()).unwrap();
    let trusted_state_change = trusted_state.verify_and_ratchet(&state_proof).unwrap();
    assert!(matches!(
        trusted_state_change,
        TrustedStateChange::Version { .. }
    ));
    let current_version = state_proof.latest_ledger_info().version();
    assert_eq!(current_version, 25);

    let t7 = db
        .reader
        .get_account_transaction(account1.address(), 2, false, current_version)
        .unwrap();
    verify_committed_txn_status(t7.as_ref(), &block2[0]).unwrap();

    let t20 = db
        .reader
        .get_account_transaction(account1.address(), 15, false, current_version)
        .unwrap();
    verify_committed_txn_status(t20.as_ref(), &block2[13]).unwrap();

    let db_state_view = db
        .reader
        .state_view_at_version(Some(current_version))
        .unwrap();

    let account1_state_view = db_state_view.as_account_with_state_view(&account1_address);

    verify_account_balance(get_account_balance(&account1_state_view), |x| {
        x == 1_770_000
    })
    .unwrap();

    let account3_state_view = db_state_view.as_account_with_state_view(&account3_address);

    verify_account_balance(get_account_balance(&account3_state_view), |x| {
        x == 1_220_000
    })
    .unwrap();

    let transaction_list_with_proof = db
        .reader
        .get_transactions(11, 18, current_version, false)
        .unwrap();
    verify_transactions(&transaction_list_with_proof, &block2[..]).unwrap();

    let account1_sent_events_batch1 = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account1.address(), 2),
            0,
            Order::Ascending,
            10,
        )
        .unwrap();
    assert_eq!(account1_sent_events_batch1.len(), 10);

    let account1_sent_events_batch2 = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account1.address(), 2),
            10,
            Order::Ascending,
            10,
        )
        .unwrap();
    assert_eq!(account1_sent_events_batch2.len(), 6);

    let account3_received_events_batch1 = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account3.address(), 1),
            u64::MAX,
            Order::Descending,
            10,
        )
        .unwrap();
    assert_eq!(account3_received_events_batch1.len(), 10);
    // Account3 has one extra deposit event from being minted to.
    assert_eq!(
        account3_received_events_batch1[0].event.sequence_number(),
        16
    );

    let account3_received_events_batch2 = db
        .reader
        .get_events(
            &EventKey::new_from_address(&account3.address(), 1),
            6,
            Order::Descending,
            10,
        )
        .unwrap();
    assert_eq!(account3_received_events_batch2.len(), 7);
    assert_eq!(
        account3_received_events_batch2[0].event.sequence_number(),
        6
    );

    aptos_db
}

pub fn create_db_and_executor<P: AsRef<std::path::Path>>(
    path: P,
    genesis: &Transaction,
) -> (
    Arc<AptosDB>,
    DbReaderWriter,
    BlockExecutor<AptosVM>,
    Waypoint,
) {
    let (db, dbrw) = DbReaderWriter::wrap(AptosDB::new_for_test(&path));
    let waypoint = bootstrap_genesis::<AptosVM>(&dbrw, genesis).unwrap();
    let executor = BlockExecutor::new(dbrw.clone());

    (db, dbrw, executor, waypoint)
}

pub fn get_account_balance(account_state_view: &AccountWithStateView) -> u64 {
    account_state_view
        .get_coin_store_resource()
        .unwrap()
        .map(|b| b.coin())
        .unwrap_or(0)
}

pub fn verify_account_balance<F>(balance: u64, f: F) -> Result<()>
where
    F: Fn(u64) -> bool,
{
    ensure!(
        f(balance),
        "balance {} doesn't satisfy the condition passed in",
        balance
    );
    Ok(())
}

pub fn verify_transactions(
    txn_list_with_proof: &TransactionListWithProof,
    expected_txns: &[Transaction],
) -> Result<()> {
    let txns = &txn_list_with_proof.transactions;
    ensure!(
        *txns == expected_txns,
        "expected txns {:?} doesn't equal to returned txns {:?}",
        expected_txns,
        txns
    );
    Ok(())
}

pub fn verify_committed_txn_status(
    txn_with_proof: Option<&TransactionWithProof>,
    expected_txn: &Transaction,
) -> Result<()> {
    let txn = &txn_with_proof
        .ok_or_else(|| anyhow!("Transaction is not committed."))?
        .transaction;

    ensure!(
        expected_txn == txn,
        "The two transactions do not match. Expected txn: {:?}, returned txn: {:?}",
        expected_txn,
        txn,
    );

    Ok(())
}
