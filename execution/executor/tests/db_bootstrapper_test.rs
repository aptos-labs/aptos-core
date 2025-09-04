// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use velor_cached_packages::velor_stdlib;
use velor_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, Uniform};
use velor_db::VelorDB;
use velor_executor::{
    block_executor::BlockExecutor,
    db_bootstrapper::{generate_waypoint, maybe_bootstrap},
};
use velor_executor_test_helpers::{
    bootstrap_genesis, gen_ledger_info_with_sigs, get_test_signed_transaction,
};
use velor_executor_types::BlockExecutorTrait;
use velor_storage_interface::{
    state_store::state_view::db_state_view::LatestDbStateCheckpointView, DbReaderWriter,
};
use velor_temppath::TempPath;
use velor_types::{
    account_address::AccountAddress,
    account_config::{
        velor_test_root_address, new_block_event_key, primary_apt_store, FungibleStoreResource,
        NewBlockEvent, ObjectGroupResource, NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG,
    },
    contract_event::ContractEvent,
    on_chain_config::{ConfigurationResource, OnChainConfig, ValidatorSet},
    state_store::{state_key::StateKey, MoveResourceExt},
    test_helpers::transaction_test_helpers::{block, TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG},
    transaction::{authenticator::AuthenticationKey, ChangeSet, Transaction, WriteSetPayload},
    trusted_state::TrustedState,
    validator_signer::ValidatorSigner,
    waypoint::Waypoint,
    write_set::{WriteOp, WriteSetMut},
};
use velor_vm::velor_vm::VelorVMBlockExecutor;
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use rand::SeedableRng;
use std::sync::Arc;

#[test]
fn test_empty_db() {
    let genesis = velor_vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis.0));
    let tmp_dir = TempPath::new();
    let db_rw = DbReaderWriter::new(VelorDB::new_for_test(&tmp_dir));

    assert!(db_rw
        .reader
        .get_latest_ledger_info_option()
        .unwrap()
        .is_none());

    // Bootstrap empty DB.
    let waypoint =
        generate_waypoint::<VelorVMBlockExecutor>(&db_rw, &genesis_txn).expect("Should not fail.");
    maybe_bootstrap::<VelorVMBlockExecutor>(&db_rw, &genesis_txn, waypoint).unwrap();
    let ledger_info = db_rw.reader.get_latest_ledger_info().unwrap();
    assert_eq!(
        Waypoint::new_epoch_boundary(ledger_info.ledger_info()).unwrap(),
        waypoint
    );

    let trusted_state = TrustedState::from_epoch_waypoint(waypoint);
    let state_proof = db_rw
        .reader
        .get_state_proof(trusted_state.version())
        .unwrap();
    let trusted_state_change = trusted_state.verify_and_ratchet(&state_proof).unwrap();
    assert!(trusted_state_change.is_epoch_change());

    // `maybe_bootstrap()` does nothing on non-empty DB.
    assert!(
        maybe_bootstrap::<VelorVMBlockExecutor>(&db_rw, &genesis_txn, waypoint)
            .unwrap()
            .is_none()
    );
}

fn execute_and_commit(txns: Vec<Transaction>, db: &DbReaderWriter, signer: &ValidatorSigner) {
    let block_id = HashValue::random();
    let li = db.reader.get_latest_ledger_info().unwrap();
    let version = li.ledger_info().version();
    let epoch = li.ledger_info().next_block_epoch();
    let target_version = version + txns.len() as u64 + 1; // Due to StateCheckpoint txn
    let executor = BlockExecutor::<VelorVMBlockExecutor>::new(db.clone());
    let output = executor
        .execute_block(
            (block_id, block(txns)).into(),
            executor.committed_block_id(),
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    assert_eq!(output.next_version(), target_version + 1);
    let ledger_info_with_sigs =
        gen_ledger_info_with_sigs(epoch, &output, block_id, &[signer.clone()]);
    executor
        .commit_blocks(vec![block_id], ledger_info_with_sigs)
        .unwrap();
}

fn get_demo_accounts() -> (
    AccountAddress,
    Ed25519PrivateKey,
    AccountAddress,
    Ed25519PrivateKey,
) {
    // This seed avoids collisions with other accounts
    let seed = [3u8; 32];
    let mut rng = ::rand::rngs::StdRng::from_seed(seed);

    let privkey1 = Ed25519PrivateKey::generate(&mut rng);
    let pubkey1 = privkey1.public_key();
    let account1_auth_key = AuthenticationKey::ed25519(&pubkey1);
    let account1 = account1_auth_key.account_address();

    let privkey2 = Ed25519PrivateKey::generate(&mut rng);
    let pubkey2 = privkey2.public_key();
    let account2_auth_key = AuthenticationKey::ed25519(&pubkey2);
    let account2 = account2_auth_key.account_address();

    (account1, privkey1, account2, privkey2)
}

fn get_velor_coin_mint_transaction(
    velor_root_key: &Ed25519PrivateKey,
    velor_root_seq_num: u64,
    account: &AccountAddress,
    amount: u64,
) -> Transaction {
    get_test_signed_transaction(
        velor_test_root_address(),
        /* sequence_number = */ velor_root_seq_num,
        velor_root_key.clone(),
        velor_root_key.public_key(),
        Some(velor_stdlib::velor_coin_mint(*account, amount)),
    )
}

fn get_account_transaction(
    velor_root_key: &Ed25519PrivateKey,
    velor_root_seq_num: u64,
    account: &AccountAddress,
    _account_key: &Ed25519PrivateKey,
) -> Transaction {
    get_test_signed_transaction(
        velor_test_root_address(),
        /* sequence_number = */ velor_root_seq_num,
        velor_root_key.clone(),
        velor_root_key.public_key(),
        Some(velor_stdlib::velor_account_create_account(*account)),
    )
}

fn get_velor_coin_transfer_transaction(
    sender: AccountAddress,
    sender_seq_number: u64,
    sender_key: &Ed25519PrivateKey,
    recipient: AccountAddress,
    amount: u64,
) -> Transaction {
    get_test_signed_transaction(
        sender,
        sender_seq_number,
        sender_key.clone(),
        sender_key.public_key(),
        Some(velor_stdlib::velor_coin_transfer(recipient, amount)),
    )
}

fn get_balance(account: &AccountAddress, db: &DbReaderWriter) -> u64 {
    let db_state_view = db.reader.latest_state_checkpoint_view().unwrap();
    FungibleStoreResource::fetch_move_resource_from_group(
        &db_state_view,
        &primary_apt_store(*account),
        &ObjectGroupResource::struct_tag(),
    )
    .unwrap()
    .unwrap()
    .balance()
}

fn get_configuration(db: &DbReaderWriter) -> ConfigurationResource {
    let db_state_view = db.reader.latest_state_checkpoint_view().unwrap();
    ConfigurationResource::fetch_config(&db_state_view).unwrap()
}

#[test]
#[cfg_attr(feature = "consensus-only-perf-test", ignore)]
fn test_new_genesis() {
    let genesis = velor_vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_key = &velor_vm_genesis::GENESIS_KEYPAIR.0;
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis.0));
    // Create bootstrapped DB.
    let tmp_dir = TempPath::new();
    let db = DbReaderWriter::new(VelorDB::new_for_test(&tmp_dir));
    let waypoint = bootstrap_genesis::<VelorVMBlockExecutor>(&db, &genesis_txn).unwrap();
    let signer = ValidatorSigner::new(
        genesis.1[0].data.owner_address,
        Arc::new(genesis.1[0].consensus_key.clone()),
    );

    // Mint for 2 demo accounts.
    let (account1, account1_key, account2, account2_key) = get_demo_accounts();
    let txn1 = get_account_transaction(genesis_key, 0, &account1, &account1_key);
    let txn2 = get_account_transaction(genesis_key, 1, &account2, &account2_key);
    let txn3 = get_velor_coin_mint_transaction(genesis_key, 2, &account1, 200_000_000);
    let txn4 = get_velor_coin_mint_transaction(genesis_key, 3, &account2, 200_000_000);
    execute_and_commit(vec![txn1, txn2, txn3, txn4], &db, &signer);
    assert_eq!(get_balance(&account1, &db), 200_000_000);
    assert_eq!(get_balance(&account2, &db), 200_000_000);

    let trusted_state = TrustedState::from_epoch_waypoint(waypoint);
    let state_proof = db.reader.get_state_proof(trusted_state.version()).unwrap();
    let trusted_state_change = trusted_state.verify_and_ratchet(&state_proof).unwrap();
    assert!(trusted_state_change.is_epoch_change());

    // New genesis transaction: set validator set, bump epoch and overwrite account1 balance.
    let configuration = get_configuration(&db);
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(ChangeSet::new(
        WriteSetMut::new(vec![
            (
                StateKey::on_chain_config::<ValidatorSet>().unwrap(),
                WriteOp::legacy_modification(
                    bcs::to_bytes(&ValidatorSet::new(vec![])).unwrap().into(),
                ),
            ),
            (
                StateKey::on_chain_config::<ConfigurationResource>().unwrap(),
                WriteOp::legacy_modification(
                    bcs::to_bytes(&configuration.bump_epoch_for_test())
                        .unwrap()
                        .into(),
                ),
            ),
            (
                StateKey::resource_group(
                    &primary_apt_store(account1),
                    &ObjectGroupResource::struct_tag(),
                ),
                WriteOp::legacy_modification(
                    velor_transaction_simulation::FungibleStore::new(
                        account1,
                        AccountAddress::TEN,
                        100_000_000,
                        false,
                        false,
                    )
                    .to_bytes()
                    .into(),
                ),
            ),
        ])
        .freeze()
        .unwrap(),
        vec![
            ContractEvent::new_v2(NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG.clone(), vec![]).unwrap(),
            ContractEvent::new_v1(
                new_block_event_key(),
                0,
                TypeTag::Struct(Box::new(NewBlockEvent::struct_tag())),
                vec![],
            )
            .expect("Should always be able to create a new block event"),
        ],
    )));

    // Bootstrap DB into new genesis.
    let waypoint = generate_waypoint::<VelorVMBlockExecutor>(&db, &genesis_txn).unwrap();
    assert!(
        maybe_bootstrap::<VelorVMBlockExecutor>(&db, &genesis_txn, waypoint)
            .unwrap()
            .is_some()
    );
    assert_eq!(waypoint.version(), 6);

    // Client bootable from waypoint.
    let trusted_state = TrustedState::from_epoch_waypoint(waypoint);
    let state_proof = db.reader.get_state_proof(trusted_state.version()).unwrap();
    let trusted_state_change = trusted_state.verify_and_ratchet(&state_proof).unwrap();
    assert!(trusted_state_change.is_epoch_change());
    let trusted_state = trusted_state_change.new_state().unwrap();
    assert_eq!(trusted_state.version(), 6);

    // Effect of bootstrapping reflected.
    assert_eq!(get_balance(&account1, &db), 100_000_000);
    // State before new genesis accessible.
    assert_eq!(get_balance(&account2, &db), 200_000_000);

    println!("FINAL TRANSFER");
    // Transfer some money.
    let txn = get_velor_coin_transfer_transaction(account1, 0, &account1_key, account2, 50_000_000);
    execute_and_commit(vec![txn], &db, &signer);

    // And verify.
    assert_eq!(get_balance(&account2, &db), 250_000_000);
}
