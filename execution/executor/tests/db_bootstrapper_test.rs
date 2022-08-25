// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, Uniform};
use aptos_state_view::account_with_state_view::AsAccountWithStateView;
use aptos_temppath::TempPath;
use aptos_types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    account_config::{
        aptos_test_root_address, new_block_event_key, CoinStoreResource, NewBlockEvent,
        CORE_CODE_ADDRESS,
    },
    account_view::AccountView,
    contract_event::ContractEvent,
    event::EventHandle,
    on_chain_config::{access_path_for_config, ConfigurationResource, OnChainConfig, ValidatorSet},
    state_store::state_key::StateKey,
    test_helpers::transaction_test_helpers::block,
    transaction::{authenticator::AuthenticationKey, ChangeSet, Transaction, WriteSetPayload},
    trusted_state::TrustedState,
    validator_signer::ValidatorSigner,
    waypoint::Waypoint,
    write_set::{WriteOp, WriteSetMut},
};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use cached_packages::aptos_stdlib;
use executor::{
    block_executor::BlockExecutor,
    db_bootstrapper::{generate_waypoint, maybe_bootstrap},
};
use executor_test_helpers::{
    bootstrap_genesis, gen_ledger_info_with_sigs, get_test_signed_transaction,
};
use executor_types::BlockExecutorTrait;
use move_deps::move_core_types::{
    language_storage::TypeTag,
    move_resource::{MoveResource, MoveStructType},
};
use rand::SeedableRng;
use storage_interface::{state_view::LatestDbStateCheckpointView, DbReaderWriter};

#[test]
fn test_empty_db() {
    let genesis = vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis.0));
    let tmp_dir = TempPath::new();
    let db_rw = DbReaderWriter::new(AptosDB::new_for_test(&tmp_dir));

    assert!(db_rw
        .reader
        .get_latest_ledger_info_option()
        .unwrap()
        .is_none());

    // Bootstrap empty DB.
    let waypoint = generate_waypoint::<AptosVM>(&db_rw, &genesis_txn).expect("Should not fail.");
    maybe_bootstrap::<AptosVM>(&db_rw, &genesis_txn, waypoint).unwrap();
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
    assert!(!maybe_bootstrap::<AptosVM>(&db_rw, &genesis_txn, waypoint).unwrap());
}

fn execute_and_commit(txns: Vec<Transaction>, db: &DbReaderWriter, signer: &ValidatorSigner) {
    let block_id = HashValue::random();
    let li = db.reader.get_latest_ledger_info().unwrap();
    let version = li.ledger_info().version();
    let epoch = li.ledger_info().next_block_epoch();
    let target_version = version + txns.len() as u64;
    let executor = BlockExecutor::<AptosVM>::new(db.clone());
    let output = executor
        .execute_block((block_id, txns), executor.committed_block_id())
        .unwrap();
    assert_eq!(output.num_leaves(), target_version + 1);
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
    let account1 = account1_auth_key.derived_address();

    let privkey2 = Ed25519PrivateKey::generate(&mut rng);
    let pubkey2 = privkey2.public_key();
    let account2_auth_key = AuthenticationKey::ed25519(&pubkey2);
    let account2 = account2_auth_key.derived_address();

    (account1, privkey1, account2, privkey2)
}

fn get_aptos_coin_mint_transaction(
    aptos_root_key: &Ed25519PrivateKey,
    aptos_root_seq_num: u64,
    account: &AccountAddress,
    amount: u64,
) -> Transaction {
    get_test_signed_transaction(
        aptos_test_root_address(),
        /* sequence_number = */ aptos_root_seq_num,
        aptos_root_key.clone(),
        aptos_root_key.public_key(),
        Some(aptos_stdlib::aptos_coin_mint(*account, amount)),
    )
}

fn get_account_transaction(
    aptos_root_key: &Ed25519PrivateKey,
    aptos_root_seq_num: u64,
    account: &AccountAddress,
    _account_key: &Ed25519PrivateKey,
) -> Transaction {
    get_test_signed_transaction(
        aptos_test_root_address(),
        /* sequence_number = */ aptos_root_seq_num,
        aptos_root_key.clone(),
        aptos_root_key.public_key(),
        Some(aptos_stdlib::aptos_account_create_account(*account)),
    )
}

fn get_aptos_coin_transfer_transaction(
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
        Some(aptos_stdlib::aptos_coin_transfer(recipient, amount)),
    )
}

fn get_balance(account: &AccountAddress, db: &DbReaderWriter) -> u64 {
    let db_state_view = db.reader.latest_state_checkpoint_view().unwrap();
    let account_state_view = db_state_view.as_account_with_state_view(account);
    account_state_view
        .get_coin_store_resource()
        .unwrap()
        .unwrap()
        .coin()
}

fn get_configuration(db: &DbReaderWriter) -> ConfigurationResource {
    let db_state_view = db.reader.latest_state_checkpoint_view().unwrap();
    let aptos_framework_account_state_view =
        db_state_view.as_account_with_state_view(&CORE_CODE_ADDRESS);
    aptos_framework_account_state_view
        .get_configuration_resource()
        .unwrap()
        .unwrap()
}

#[test]
fn test_new_genesis() {
    let genesis = vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_key = &vm_genesis::GENESIS_KEYPAIR.0;
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis.0));
    // Create bootstrapped DB.
    let tmp_dir = TempPath::new();
    let db = DbReaderWriter::new(AptosDB::new_for_test(&tmp_dir));
    let waypoint = bootstrap_genesis::<AptosVM>(&db, &genesis_txn).unwrap();
    let signer = ValidatorSigner::new(
        genesis.1[0].data.owner_address,
        genesis.1[0].consensus_key.clone(),
    );

    // Mint for 2 demo accounts.
    let (account1, account1_key, account2, account2_key) = get_demo_accounts();
    let txn1 = get_account_transaction(genesis_key, 0, &account1, &account1_key);
    let txn2 = get_account_transaction(genesis_key, 1, &account2, &account2_key);
    let txn3 = get_aptos_coin_mint_transaction(genesis_key, 2, &account1, 2_000_000);
    let txn4 = get_aptos_coin_mint_transaction(genesis_key, 3, &account2, 2_000_000);
    execute_and_commit(block(vec![txn1, txn2, txn3, txn4]), &db, &signer);
    assert_eq!(get_balance(&account1, &db), 2_000_000);
    assert_eq!(get_balance(&account2, &db), 2_000_000);

    let trusted_state = TrustedState::from_epoch_waypoint(waypoint);
    let state_proof = db.reader.get_state_proof(trusted_state.version()).unwrap();
    let trusted_state_change = trusted_state.verify_and_ratchet(&state_proof).unwrap();
    assert!(trusted_state_change.is_epoch_change());

    // New genesis transaction: set validator set, bump epoch and overwrite account1 balance.
    let configuration = get_configuration(&db);
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(ChangeSet::new(
        WriteSetMut::new(vec![
            (
                StateKey::AccessPath(access_path_for_config(ValidatorSet::CONFIG_ID)),
                WriteOp::Modification(bcs::to_bytes(&ValidatorSet::new(vec![])).unwrap()),
            ),
            (
                StateKey::AccessPath(AccessPath::new(
                    CORE_CODE_ADDRESS,
                    ConfigurationResource::resource_path(),
                )),
                WriteOp::Modification(bcs::to_bytes(&configuration.bump_epoch_for_test()).unwrap()),
            ),
            (
                StateKey::AccessPath(AccessPath::new(
                    account1,
                    CoinStoreResource::resource_path(),
                )),
                WriteOp::Modification(
                    bcs::to_bytes(&CoinStoreResource::new(
                        1_000_000,
                        false,
                        EventHandle::random(0),
                        EventHandle::random(0),
                    ))
                    .unwrap(),
                ),
            ),
        ])
        .freeze()
        .unwrap(),
        vec![
            ContractEvent::new(
                *configuration.events().key(),
                0,
                TypeTag::Struct(ConfigurationResource::struct_tag()),
                vec![],
            ),
            ContractEvent::new(
                new_block_event_key(),
                0,
                TypeTag::Struct(NewBlockEvent::struct_tag()),
                vec![],
            ),
        ],
    )));

    // Bootstrap DB into new genesis.
    let waypoint = generate_waypoint::<AptosVM>(&db, &genesis_txn).unwrap();
    assert!(maybe_bootstrap::<AptosVM>(&db, &genesis_txn, waypoint).unwrap());
    assert_eq!(waypoint.version(), 6);

    // Client bootable from waypoint.
    let trusted_state = TrustedState::from_epoch_waypoint(waypoint);
    let state_proof = db.reader.get_state_proof(trusted_state.version()).unwrap();
    let trusted_state_change = trusted_state.verify_and_ratchet(&state_proof).unwrap();
    assert!(trusted_state_change.is_epoch_change());
    let trusted_state = trusted_state_change.new_state().unwrap();
    assert_eq!(trusted_state.version(), 6);

    // Effect of bootstrapping reflected.
    assert_eq!(get_balance(&account1, &db), 1_000_000);
    // State before new genesis accessible.
    assert_eq!(get_balance(&account2, &db), 2_000_000);

    // Transfer some money.
    let txn = get_aptos_coin_transfer_transaction(account1, 0, &account1_key, account2, 500_000);
    execute_and_commit(block(vec![txn]), &db, &signer);

    // And verify.
    assert_eq!(get_balance(&account2, &db), 2_500_000);
}
