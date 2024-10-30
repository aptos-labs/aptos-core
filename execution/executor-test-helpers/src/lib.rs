// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod integration_test_impl;

use aptos_config::config::NodeConfig;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    HashValue,
};
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_executor_types::state_compute_result::StateComputeResult;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures},
    test_helpers::transaction_test_helpers::get_test_signed_txn,
    transaction::{Transaction, TransactionPayload},
    validator_signer::ValidatorSigner,
    waypoint::Waypoint,
};
use aptos_vm::VMBlockExecutor;
use std::sync::Arc;

/// Helper function for test to blindly bootstrap without waypoint.
pub fn bootstrap_genesis<V: VMBlockExecutor>(
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
            output.expect_last_version(),
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
        Arc::new(sr_test.consensus_key.as_ref().unwrap().private_key()),
    )
}

pub fn get_test_signed_transaction(
    sender: AccountAddress,
    sequence_number: u64,
    private_key: Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    payload: Option<TransactionPayload>,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> Transaction {
    Transaction::UserTransaction(get_test_signed_txn(
        sender,
        sequence_number,
        &private_key,
        public_key,
        payload,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    ))
}
