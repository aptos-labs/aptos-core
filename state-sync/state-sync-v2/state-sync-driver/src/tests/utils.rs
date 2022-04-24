// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    HashValue, PrivateKey, Uniform,
};
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    chain_id::ChainId,
    contract_event::ContractEvent,
    event::EventKey,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::{
        SparseMerkleRangeProof, TransactionAccumulatorRangeProof, TransactionInfoListWithProof,
    },
    state_store::state_value::StateValueChunkWithProof,
    transaction::{
        RawTransaction, Script, SignedTransaction, Transaction, TransactionInfo,
        TransactionListWithProof, TransactionOutput, TransactionOutputListWithProof,
        TransactionPayload, TransactionStatus, Version,
    },
    vm_status::KeptVMStatus,
    write_set::WriteSet,
};
use move_core_types::language_storage::TypeTag;
use std::collections::BTreeMap;
use storage_interface::{StartupInfo, TreeState};

/// Creates a test epoch ending ledger info
pub fn create_epoch_ending_ledger_info() -> LedgerInfoWithSignatures {
    let ledger_info = LedgerInfo::new(BlockInfo::random(0), HashValue::random());
    LedgerInfoWithSignatures::new(ledger_info, BTreeMap::new())
}

/// Creates a single test event
pub fn create_event() -> ContractEvent {
    ContractEvent::new(
        EventKey::random(),
        0,
        TypeTag::Bool,
        bcs::to_bytes(&0).unwrap(),
    )
}

/// Creates a new ledger info with signatures at the specified version
pub fn create_ledger_info_at_version(version: Version) -> LedgerInfoWithSignatures {
    let block_info = BlockInfo::new(0, 0, HashValue::zero(), HashValue::zero(), version, 0, None);
    let ledger_info = LedgerInfo::new(block_info, HashValue::random());
    LedgerInfoWithSignatures::new(ledger_info, BTreeMap::new())
}

/// Creates a test transaction output list with proof
pub fn create_output_list_with_proof() -> TransactionOutputListWithProof {
    let transaction_info_list_with_proof = create_transaction_info_list_with_proof();
    let transaction_and_output = (create_transaction(), create_transaction_output());
    TransactionOutputListWithProof::new(
        vec![transaction_and_output],
        Some(0),
        transaction_info_list_with_proof,
    )
}

/// Creates a test state value chunk with proof
pub fn create_state_value_chunk_with_proof(last_chunk: bool) -> StateValueChunkWithProof {
    let right_siblings = if last_chunk {
        vec![]
    } else {
        vec![HashValue::random()]
    };
    StateValueChunkWithProof {
        first_index: 0,
        last_index: 100,
        first_key: HashValue::random(),
        last_key: HashValue::random(),
        raw_values: vec![],
        proof: SparseMerkleRangeProof::new(right_siblings),
        root_hash: HashValue::random(),
    }
}

/// Creates a startup info
pub fn create_startup_info() -> StartupInfo {
    let committed_tree_state = TreeState {
        num_transactions: 0,
        ledger_frozen_subtree_hashes: vec![],
        account_state_root_hash: Default::default(),
    };

    StartupInfo {
        latest_ledger_info: create_ledger_info_at_version(0),
        latest_epoch_state: None,
        committed_tree_state,
        synced_tree_state: None,
    }
}

/// Creates a single test transaction
pub fn create_transaction() -> Transaction {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();

    let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
    let raw_transaction = RawTransaction::new(
        AccountAddress::random(),
        0,
        transaction_payload,
        0,
        0,
        "".into(),
        0,
        ChainId::new(10),
    );
    let signed_transaction = SignedTransaction::new(
        raw_transaction,
        public_key,
        Ed25519Signature::dummy_signature(),
    );

    Transaction::UserTransaction(signed_transaction)
}

/// Creates a test transaction info
pub fn create_transaction_info() -> TransactionInfo {
    TransactionInfo::new(
        HashValue::random(),
        HashValue::random(),
        HashValue::random(),
        0,
        KeptVMStatus::Executed,
    )
}

/// Creates a test transaction info list with proof
pub fn create_transaction_info_list_with_proof() -> TransactionInfoListWithProof {
    TransactionInfoListWithProof::new(
        TransactionAccumulatorRangeProof::new_empty(),
        vec![create_transaction_info()],
    )
}

/// Creates a test transaction list with proof
pub fn create_transaction_list_with_proof() -> TransactionListWithProof {
    let transaction_info_list_with_proof = create_transaction_info_list_with_proof();
    TransactionListWithProof::new(
        vec![create_transaction()],
        None,
        Some(0),
        transaction_info_list_with_proof,
    )
}

/// Creates a single test transaction output
pub fn create_transaction_output() -> TransactionOutput {
    TransactionOutput::new(
        WriteSet::default(),
        vec![create_event()],
        0,
        TransactionStatus::Keep(KeptVMStatus::Executed),
    )
}
