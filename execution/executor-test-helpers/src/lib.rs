// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod integration_test_impl;

use aptos_config::config::NodeConfig;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    HashValue,
};
use aptos_types::ledger_info::generate_ledger_info_with_sig;
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    test_helpers::transaction_test_helpers::get_test_signed_txn,
    transaction::{Transaction, TransactionPayload},
    validator_signer::ValidatorSigner,
    waypoint::Waypoint,
};
use aptos_vm::VMExecutor;
use executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use executor_types::StateComputeResult;
use storage_interface::DbReaderWriter;

/// Helper function for test to blindly bootstrap without waypoint.
pub fn bootstrap_genesis<V: VMExecutor>(
    db: &DbReaderWriter,
    genesis_txn: &Transaction,
) -> anyhow::Result<Waypoint> {
    let waypoint = generate_waypoint::<V>(db, genesis_txn)?;
    maybe_bootstrap::<V>(db, genesis_txn, waypoint)?;
    Ok(waypoint)
}

pub fn gen_block_id(index: u8) -> HashValue {
    HashValue::new([index; HashValue::LENGTH])
}

pub fn gen_ledger_info_with_sigs(
    epoch: u64,
    output: &StateComputeResult,
    commit_block_id: HashValue,
    signer: &[ValidatorSigner],
) -> LedgerInfoWithSignatures {
    let ledger_info = LedgerInfo::new(
        BlockInfo::new(
            epoch,
            0, /* round */
            commit_block_id,
            output.root_hash(),
            output.version(),
            0, /* timestamp */
            output.epoch_state().clone(),
        ),
        HashValue::zero(),
    );
    generate_ledger_info_with_sig(signer, ledger_info)
}

pub fn extract_signer(config: &mut NodeConfig) -> ValidatorSigner {
    let sr_test = config.consensus.safety_rules.test.as_ref().unwrap();
    ValidatorSigner::new(
        sr_test.author,
        sr_test.consensus_key.as_ref().unwrap().private_key(),
    )
}

pub fn get_test_signed_transaction(
    sender: AccountAddress,
    sequence_number: u64,
    private_key: Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    payload: Option<TransactionPayload>,
) -> Transaction {
    Transaction::UserTransaction(get_test_signed_txn(
        sender,
        sequence_number,
        &private_key,
        public_key,
        payload,
    ))
}
