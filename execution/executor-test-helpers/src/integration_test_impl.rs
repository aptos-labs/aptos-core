// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{bootstrap_genesis, gen_block_id, gen_ledger_info_with_sigs};
use anyhow::{ensure, Result};
use aptos_cached_packages::aptos_stdlib;
use aptos_config::config::DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD;
use aptos_consensus_types::block::Block;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor_types::BlockExecutorTrait;
use aptos_sdk::{
    bcs,
    move_types::{
        account_address::AccountAddress, language_storage::StructTag, move_resource::MoveStructType,
    },
    transaction_builder::TransactionFactory,
    types::{get_apt_primary_store_address, AccountKey, LocalAccount},
};
use aptos_storage_interface::{
    state_store::state_view::db_state_view::{DbStateViewAtVersion, VerifiedStateViewAtVersion},
    DbReaderWriter,
};
use aptos_types::{
    account_config::{
        aptos_test_root_address, AccountResource, CoinStoreResource, FungibleStoreResource,
        ObjectGroupResource,
    },
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    ledger_info::LedgerInfo,
    state_store::{state_key::StateKey, MoveResourceExt, StateView},
    test_helpers::transaction_test_helpers::{block, TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG},
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        Transaction::{self, UserTransaction},
        TransactionListWithProof, TransactionWithProof, WriteSetPayload,
    },
    trusted_state::{TrustedState, TrustedStateChange},
    waypoint::Waypoint,
    AptosCoinType,
};
use aptos_vm::aptos_vm::AptosVMBlockExecutor;
use rand::SeedableRng;
use std::{collections::BTreeMap, path::Path, sync::Arc};

pub fn test_execution_with_storage_impl() -> Arc<AptosDB> {
    let path = aptos_temppath::TempPath::new();
    path.create_as_dir().unwrap();
    test_execution_with_storage_impl_inner(false, path.path())
}

pub fn test_execution_with_storage_impl_inner(
    force_sharding: bool,
    db_path: &Path,
) -> Arc<AptosDB> {
    const B: u64 = 1_000_000_000;

    let (genesis, validators) = aptos_vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));

    let core_resources_account: LocalAccount = LocalAccount::new(
        aptos_test_root_address(),
        AccountKey::from_private_key(aptos_vm_genesis::GENESIS_KEYPAIR.0.clone()),
        0,
    );

    let (aptos_db, db, executor, waypoint) =
        create_db_and_executor(db_path, &genesis_txn, force_sharding);

    let parent_block_id = executor.committed_block_id();
    let signer = aptos_types::validator_signer::ValidatorSigner::new(
        validators[0].data.owner_address,
        Arc::new(validators[0].consensus_key.clone()),
    );

    // This generates accounts that do not overlap with genesis
    let seed = [3u8; 32];
    let mut rng = ::rand::rngs::StdRng::from_seed(seed);

    let account1 = LocalAccount::generate(&mut rng);
    let account2 = LocalAccount::generate(&mut rng);
    let account3 = LocalAccount::generate(&mut rng);
    let account4 = LocalAccount::generate(&mut rng);

    let addr1 = account1.address();
    let addr2 = account2.address();
    let addr3 = account3.address();
    let addr4 = account4.address();

    let txn_factory = TransactionFactory::new(ChainId::test());

    let block1_id = gen_block_id(1);
    let block1_meta = Transaction::BlockMetadata(BlockMetadata::new(
        block1_id,
        1,
        0,
        signer.author(),
        vec![0],
        vec![],
        1,
    ));
    let tx1 = core_resources_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account1.public_key()));
    let tx2 = core_resources_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account2.public_key()));
    let tx3 = core_resources_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account3.public_key()));

    // Create account1 with 2T coins.
    let txn1 = core_resources_account
        .sign_with_transaction_builder(txn_factory.mint(account1.address(), 2_000 * B));
    // Create account2 with 1.2T coins.
    let txn2 = core_resources_account
        .sign_with_transaction_builder(txn_factory.mint(account2.address(), 1_200 * B));
    // Create account3 with 1T coins.
    let txn3 = core_resources_account
        .sign_with_transaction_builder(txn_factory.mint(account3.address(), 1_000 * B));

    // Transfer 20B coins from account1 to account2.
    // balance: <1.98T, 1.22T, 1T
    let txn4 =
        account1.sign_with_transaction_builder(txn_factory.transfer(account2.address(), 20 * B));

    // Transfer 10B coins from account2 to account3.
    // balance: <1.98T, <1.21T, 1.01T
    let txn5 =
        account2.sign_with_transaction_builder(txn_factory.transfer(account3.address(), 10 * B));

    // Transfer 70B coins from account1 to account3.
    // balance: <1.91T, <1.21T, 1.08T
    let txn6 =
        account1.sign_with_transaction_builder(txn_factory.transfer(account3.address(), 70 * B));

    let reconfig1 = core_resources_account.sign_with_transaction_builder(
        txn_factory.payload(aptos_stdlib::aptos_governance_force_end_epoch_test_only()),
    );

    let block1: Vec<_> = into_signature_verified_block(vec![
        block1_meta,
        UserTransaction(tx1),
        UserTransaction(tx2),
        UserTransaction(tx3),
        UserTransaction(txn1),
        UserTransaction(txn2),
        UserTransaction(txn3),
        UserTransaction(txn4),
        UserTransaction(txn5),
        UserTransaction(txn6),
        UserTransaction(reconfig1),
    ]);

    let block2_id = gen_block_id(2);
    let block2_meta = Transaction::BlockMetadata(BlockMetadata::new(
        block2_id,
        2,
        0,
        signer.author(),
        vec![0],
        vec![],
        2,
    ));
    let reconfig2 = core_resources_account.sign_with_transaction_builder(
        txn_factory.payload(aptos_stdlib::aptos_governance_force_end_epoch_test_only()),
    );
    let block2 = vec![block2_meta, UserTransaction(reconfig2)];

    let block3_id = gen_block_id(3);
    let block3_meta = Transaction::BlockMetadata(BlockMetadata::new(
        block3_id,
        2,
        1,
        signer.author(),
        vec![0],
        vec![],
        3,
    ));
    let mut block3 = vec![block3_meta];
    // Create 14 txns transferring 10k from account1 to account3 each.
    for _ in 2..=15 {
        block3.push(UserTransaction(account1.sign_with_transaction_builder(
            txn_factory.transfer(account3.address(), 10 * B),
        )));
    }
    let block3 = block(block3); // append state checkpoint txn

    let output1 = executor
        .execute_block(
            (block1_id, block1.clone()).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let li1 = gen_ledger_info_with_sigs(1, &output1, block1_id, &[signer.clone()]);
    let epoch2_genesis_id = Block::make_genesis_block_from_ledger_info(li1.ledger_info()).id();
    executor.commit_blocks(vec![block1_id], li1).unwrap();

    let state_proof = db.reader.get_state_proof(0).unwrap();
    let trusted_state = TrustedState::from_epoch_waypoint(waypoint);
    let trusted_state = match trusted_state.verify_and_ratchet(&state_proof) {
        Ok(TrustedStateChange::Epoch { new_state, .. }) => new_state,
        _ => panic!("unexpected state change"),
    };
    let latest_li = state_proof.latest_ledger_info();
    let current_version = latest_li.version();
    assert_eq!(trusted_state.version(), current_version);
    assert_eq!(current_version, 11);

    let t5 = db
        .reader
        .get_transaction_by_version(5, current_version, false)
        .unwrap();
    verify_committed_txn_status(latest_li, &t5, &block1[4]).unwrap();

    let t6 = db
        .reader
        .get_transaction_by_version(6, current_version, false)
        .unwrap();
    verify_committed_txn_status(latest_li, &t6, &block1[5]).unwrap();

    let t7 = db
        .reader
        .get_transaction_by_version(7, current_version, false)
        .unwrap();
    verify_committed_txn_status(latest_li, &t7, &block1[6]).unwrap();

    let reconfig1 = db
        .reader
        .get_transaction_by_version(11, current_version, false)
        .unwrap();
    verify_committed_txn_status(latest_li, &reconfig1, &block1[10]).unwrap();

    let t8 = db
        .reader
        .get_transaction_by_version(8, current_version, true)
        .unwrap();
    verify_committed_txn_status(latest_li, &t8, &block1[7]).unwrap();
    // We requested the events to come back from this one, so verify that they did
    assert_eq!(t8.events.unwrap().len(), 3);

    let t9 = db
        .reader
        .get_transaction_by_version(9, current_version, false)
        .unwrap();
    verify_committed_txn_status(latest_li, &t9, &block1[8]).unwrap();

    let t10 = db
        .reader
        .get_transaction_by_version(10, current_version, true)
        .unwrap();
    verify_committed_txn_status(latest_li, &t10, &block1[9]).unwrap();

    // test the initial balance.
    // not a state checkpoint, can't get verified view
    let view = db.reader.state_view_at_version(Some(7)).unwrap();
    verify_account_balance(get_account_balance(&view, &addr1), |x| x == 2_000 * B).unwrap();
    verify_account_balance(get_account_balance(&view, &addr2), |x| x == 1_200 * B).unwrap();
    verify_account_balance(get_account_balance(&view, &addr3), |x| x == 1_000 * B).unwrap();

    // test the final balance.
    let view = db
        .reader
        .verified_state_view_at_version(Some(current_version), latest_li)
        .unwrap();
    verify_account_balance(get_account_balance(&view, &addr1), |x| {
        approx_eq(x, 1_910 * B)
    })
    .unwrap();
    verify_account_balance(get_account_balance(&view, &addr2), |x| {
        approx_eq(x, 1_210 * B)
    })
    .unwrap();
    verify_account_balance(get_account_balance(&view, &addr3), |x| {
        approx_eq(x, 1_080 * B)
    })
    .unwrap();

    let transaction_list_with_proof = db
        .reader
        .get_transactions(3, 12, current_version, false)
        .unwrap();
    let expected_txns: Vec<Transaction> = block1[2..]
        .iter()
        .map(|t| t.expect_valid().clone())
        .collect();
    verify_transactions(&transaction_list_with_proof, &expected_txns).unwrap();

    // With sharding enabled, we won't have indices for event, skip the checks.
    if !force_sharding {
        let view = db
            .reader
            .verified_state_view_at_version(Some(current_version), latest_li)
            .unwrap();
        let account4_resource = AccountResource::fetch_move_resource(&view, &addr4).unwrap();
        assert!(account4_resource.is_none());
    }

    // Execute block 2, 3, 4
    let output2 = executor
        .execute_block(
            (block2_id, block2).into(),
            epoch2_genesis_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let li2 = gen_ledger_info_with_sigs(2, &output2, block2_id, &[signer.clone()]);
    let epoch3_genesis_id = Block::make_genesis_block_from_ledger_info(li2.ledger_info()).id();
    executor.commit_blocks(vec![block2_id], li2).unwrap();

    let state_proof = db.reader.get_state_proof(trusted_state.version()).unwrap();
    let trusted_state = match trusted_state.verify_and_ratchet(&state_proof) {
        Ok(TrustedStateChange::Epoch { new_state, .. }) => new_state,
        _ => panic!("unexpected state change"),
    };
    let latest_li = state_proof.latest_ledger_info();
    let current_version = latest_li.version();
    assert_eq!(trusted_state.version(), current_version);
    assert_eq!(current_version, 13);

    let output3 = executor
        .execute_block(
            (block3_id, block3.clone()).into(),
            epoch3_genesis_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let li3 = gen_ledger_info_with_sigs(3, &output3, block3_id, &[signer]);
    executor.commit_blocks(vec![block3_id], li3).unwrap();

    let state_proof = db.reader.get_state_proof(trusted_state.version()).unwrap();
    let _trusted_state = match trusted_state.verify_and_ratchet(&state_proof) {
        Ok(TrustedStateChange::Version { new_state, .. }) => new_state,
        _ => panic!("unexpected state change"),
    };
    let latest_li = state_proof.latest_ledger_info();
    let current_version = latest_li.version();
    assert_eq!(current_version, 29);

    // More verification
    let t15 = db
        .reader
        .get_transaction_by_version(15, current_version, false)
        .unwrap();
    verify_committed_txn_status(latest_li, &t15, &block3[1]).unwrap();

    let t28 = db
        .reader
        .get_transaction_by_version(28, current_version, false)
        .unwrap();
    verify_committed_txn_status(latest_li, &t28, &block3[14]).unwrap();

    let view = db
        .reader
        .verified_state_view_at_version(Some(current_version), latest_li)
        .unwrap();

    verify_account_balance(get_account_balance(&view, &addr1), |x| {
        approx_eq(x, 1_770 * B)
    })
    .unwrap();
    verify_account_balance(get_account_balance(&view, &addr3), |x| {
        approx_eq(x, 1_220 * B)
    })
    .unwrap();

    let transaction_list_with_proof = db
        .reader
        .get_transactions(14, 15, current_version, false)
        .unwrap();
    let expected_txns: Vec<Transaction> = block3.iter().map(|t| t.expect_valid().clone()).collect();
    verify_transactions(&transaction_list_with_proof, &expected_txns).unwrap();

    aptos_db
}

fn approx_eq(a: u64, b: u64) -> bool {
    const M: u64 = 10_000_000;
    a + M > b && b + M > a
}

pub fn create_db_and_executor<P: AsRef<std::path::Path>>(
    path: P,
    genesis: &Transaction,
    force_sharding: bool, // if true force sharding db otherwise using default db
) -> (
    Arc<AptosDB>,
    DbReaderWriter,
    BlockExecutor<AptosVMBlockExecutor>,
    Waypoint,
) {
    let (db, dbrw) = force_sharding
        .then(|| {
            DbReaderWriter::wrap(AptosDB::new_for_test_with_sharding(
                &path,
                DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            ))
        })
        .unwrap_or_else(|| DbReaderWriter::wrap(AptosDB::new_for_test(&path)));
    let waypoint = bootstrap_genesis::<AptosVMBlockExecutor>(&dbrw, genesis).unwrap();
    let executor = BlockExecutor::new(dbrw.clone());

    (db, dbrw, executor, waypoint)
}

pub fn get_account_balance(state_view: &dyn StateView, address: &AccountAddress) -> u64 {
    CoinStoreResource::<AptosCoinType>::fetch_move_resource(state_view, address)
        .unwrap()
        .map_or(0, |coin_store| coin_store.coin())
        + {
            let bytes_opt = state_view
                .get_state_value_bytes(&StateKey::resource_group(
                    &get_apt_primary_store_address(*address),
                    &ObjectGroupResource::struct_tag(),
                ))
                .expect("account must exist in data store");

            let group: Option<BTreeMap<StructTag, Vec<u8>>> = bytes_opt
                .map(|bytes| bcs::from_bytes(&bytes))
                .transpose()
                .unwrap();
            group
                .and_then(|g| {
                    g.get(&FungibleStoreResource::struct_tag())
                        .map(|b| bcs::from_bytes(b))
                })
                .transpose()
                .unwrap()
                .map(|x: FungibleStoreResource| x.balance())
                .unwrap_or(0)
        }
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
    latest_li: &LedgerInfo,
    txn_with_proof: &TransactionWithProof,
    expected_txn: &SignatureVerifiedTransaction,
) -> Result<()> {
    txn_with_proof
        .proof
        .verify(latest_li, txn_with_proof.version)?;

    let txn = &txn_with_proof.transaction;

    ensure!(
        expected_txn.expect_valid() == txn,
        "The two transactions do not match. Expected txn: {:?}, returned txn: {:?}",
        expected_txn,
        txn,
    );

    Ok(())
}
