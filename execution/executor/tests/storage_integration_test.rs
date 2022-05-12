// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::PrivateKey;
use aptos_state_view::account_with_state_view::AsAccountWithStateView;
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::{
    access_path::AccessPath,
    account_config::{aptos_root_address, AccountResource},
    account_view::AccountView,
    block_metadata::BlockMetadata,
    state_store::state_key::StateKey,
    transaction::{Transaction, WriteSetPayload},
    trusted_state::TrustedState,
    validator_signer::ValidatorSigner,
};
use executor_test_helpers::{
    gen_block_id, gen_ledger_info_with_sigs, get_test_signed_transaction,
    integration_test_impl::{
        create_db_and_executor, test_execution_with_storage_impl, verify_committed_txn_status,
    },
};
use executor_types::BlockExecutorTrait;
use move_deps::move_core_types::move_resource::MoveStructType;
use storage_interface::state_view::DbStateViewAtVersion;

#[test]
fn test_genesis() {
    let path = aptos_temppath::TempPath::new();
    path.create_as_dir().unwrap();
    let genesis = vm_genesis::test_genesis_transaction();
    let (_, db, _executor, waypoint) = create_db_and_executor(path.path(), &genesis);

    let trusted_state = TrustedState::from_epoch_waypoint(waypoint);
    let initial_accumulator = db
        .reader
        .get_accumulator_summary(trusted_state.version())
        .unwrap();
    let state_proof = db.reader.get_state_proof(trusted_state.version()).unwrap();

    trusted_state
        .verify_and_ratchet(&state_proof, Some(&initial_accumulator))
        .unwrap();
    let li = state_proof.latest_ledger_info();
    assert_eq!(li.version(), 0);

    let account_resource_path = StateKey::AccessPath(AccessPath::new(
        aptos_root_address(),
        AccountResource::struct_tag().access_vector(),
    ));
    let aptos_root_account_resource = db
        .reader
        .get_state_value_with_proof(account_resource_path.clone(), 0, 0)
        .unwrap();
    aptos_root_account_resource
        .verify(li, 0, account_resource_path)
        .unwrap();
}

#[test]
fn test_reconfiguration() {
    // When executing a transaction emits a validator set change,
    // storage should propagate the new validator set

    let path = aptos_temppath::TempPath::new();
    path.create_as_dir().unwrap();
    let (genesis, validators) = vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_key = &vm_genesis::GENESIS_KEYPAIR.0;
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
    let (_, db, executor, _waypoint) = create_db_and_executor(path.path(), &genesis_txn);
    let parent_block_id = executor.committed_block_id();
    let signer = ValidatorSigner::new(validators[0].data.address, validators[0].key.clone());
    let validator_account = signer.author();

    // test the current keys in the validator's account equals to the key in the validator set
    let state_proof = db.reader.get_state_proof(0).unwrap();
    let current_version = state_proof.latest_ledger_info().version();
    let db_state_view = db
        .reader
        .state_view_at_version(Some(current_version))
        .unwrap();
    let validator_account_state_view = db_state_view.as_account_with_state_view(&validator_account);
    let root_address = aptos_root_address();
    let root_account_state_view = db_state_view.as_account_with_state_view(&root_address);

    assert_eq!(
        root_account_state_view
            .get_validator_set()
            .unwrap()
            .unwrap()
            .payload()
            .next()
            .unwrap()
            .consensus_public_key(),
        &validator_account_state_view
            .get_validator_config_resource()
            .unwrap()
            .unwrap()
            .consensus_public_key
    );

    // txn1 = give the validator some money so they can send a tx
    let txn1 = get_test_signed_transaction(
        aptos_root_address(),
        /* sequence_number = */ 0,
        genesis_key.clone(),
        genesis_key.public_key(),
        Some(aptos_stdlib::encode_test_coin_mint(
            validator_account,
            1_000_000,
        )),
    );
    // txn2 = a dummy block prologue to bump the timer.
    let txn2 = Transaction::BlockMetadata(BlockMetadata::new(
        gen_block_id(1),
        0,
        1,
        vec![false],
        validator_account,
        300000001,
    ));

    // txn3 = set the aptos version
    let txn3 = get_test_signed_transaction(
        aptos_root_address(),
        /* sequence_number = */ 1,
        genesis_key.clone(),
        genesis_key.public_key(),
        Some(aptos_stdlib::encode_version_set_version(42)),
    );

    let txn_block = vec![txn1, txn2, txn3];
    let block_id = gen_block_id(1);
    let vm_output = executor
        .execute_block((block_id, txn_block.clone()), parent_block_id)
        .unwrap();

    // Make sure the execution result sees the reconfiguration
    assert!(
        vm_output.has_reconfiguration(),
        "StateComputeResult does not see a reconfiguration"
    );
    let ledger_info_with_sigs = gen_ledger_info_with_sigs(1, &vm_output, block_id, vec![&signer]);
    executor
        .commit_blocks(vec![block_id], ledger_info_with_sigs)
        .unwrap();

    let state_proof = db.reader.get_state_proof(0).unwrap();
    let current_version = state_proof.latest_ledger_info().version();

    let t3 = db
        .reader
        .get_account_transaction(aptos_root_address(), 1, true, current_version)
        .unwrap();
    verify_committed_txn_status(t3.as_ref(), &txn_block[2]).unwrap();

    let db_state_view = db
        .reader
        .state_view_at_version(Some(current_version))
        .unwrap();

    let root_account_state_view2 = db_state_view.as_account_with_state_view(&root_address);

    assert_eq!(
        root_account_state_view2
            .get_version()
            .unwrap()
            .unwrap()
            .major,
        42
    );
}

#[test]
fn test_execution_with_storage() {
    test_execution_with_storage_impl();
}
