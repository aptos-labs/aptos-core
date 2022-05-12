// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::driver::DriverConfiguration;
use aptos_config::config::{RoleType, StateSyncDriverConfig};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    HashValue, PrivateKey, Uniform,
};
use aptos_data_client::GlobalDataSummary;
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    chain_id::ChainId,
    contract_event::ContractEvent,
    epoch_state::EpochState,
    event::EventKey,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::{
        SparseMerkleRangeProof, TransactionAccumulatorRangeProof, TransactionInfoListWithProof,
    },
    state_store::state_value::StateValueChunkWithProof,
    transaction::{
        ExecutionStatus, RawTransaction, Script, SignedTransaction, Transaction, TransactionInfo,
        TransactionListWithProof, TransactionOutput, TransactionOutputListWithProof,
        TransactionPayload, TransactionStatus, Version,
    },
    waypoint::Waypoint,
    write_set::WriteSet,
};
use channel::{aptos_channel, aptos_channel::Sender, message_queues::QueueStyle};
use data_streaming_service::{
    data_notification::DataNotification, data_stream::DataStreamListener, streaming_client::Epoch,
};
use move_deps::move_core_types::language_storage::TypeTag;
use std::collections::BTreeMap;
use storage_interface::{StartupInfo, TreeState};
use storage_service_types::CompleteDataRange;

/// Creates a new data stream listener and notification sender pair
pub fn create_data_stream_listener() -> (Sender<(), DataNotification>, DataStreamListener) {
    let (notification_sender, notification_receiver) =
        aptos_channel::new(QueueStyle::KLAST, 100, None);
    let data_stream_listener = DataStreamListener::new(notification_receiver);

    (notification_sender, data_stream_listener)
}

/// Creates a test epoch ending ledger info
pub fn create_epoch_ending_ledger_info() -> LedgerInfoWithSignatures {
    let ledger_info = LedgerInfo::new(BlockInfo::random(0), HashValue::zero());
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

/// Creates a test driver configuration for full nodes
pub fn create_full_node_driver_configuration() -> DriverConfiguration {
    let config = StateSyncDriverConfig::default();
    let role = RoleType::FullNode;
    let waypoint = Waypoint::default();

    DriverConfiguration {
        config,
        role,
        waypoint,
    }
}

/// Creates a global data summary with the highest ended epoch
pub fn create_global_summary(highest_ended_epoch: Epoch) -> GlobalDataSummary {
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary
        .advertised_data
        .epoch_ending_ledger_infos = vec![CompleteDataRange::new(0, highest_ended_epoch).unwrap()];
    global_data_summary
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

/// Creates a random epoch ending ledger info with the specified values
pub fn create_random_epoch_ending_ledger_info(
    version: Version,
    epoch: Epoch,
) -> LedgerInfoWithSignatures {
    let block_info = BlockInfo::new(
        epoch,
        0,
        HashValue::zero(),
        HashValue::random(),
        version,
        0,
        Some(EpochState::empty()),
    );
    let ledger_info = LedgerInfo::new(block_info, HashValue::random());
    LedgerInfoWithSignatures::new(ledger_info, BTreeMap::new())
}

/// Creates a test startup info
pub fn create_startup_info() -> StartupInfo {
    StartupInfo::new(
        create_epoch_ending_ledger_info(),
        Some(EpochState::empty()),
        TreeState::new(0, vec![], HashValue::random()),
        None,
    )
}

/// Creates a test startup info at the given version and epoch
pub fn create_startup_info_at_version_epoch(version: Version, epoch: Epoch) -> StartupInfo {
    let mut startup_info = create_startup_info();

    // Set the latest ledger info
    let block_info = BlockInfo::new(
        epoch,
        0,
        HashValue::zero(),
        HashValue::random(),
        version,
        0,
        None,
    );
    let ledger_info = LedgerInfo::new(block_info, HashValue::random());
    startup_info.latest_ledger_info = LedgerInfoWithSignatures::new(ledger_info, BTreeMap::new());

    // Set the latest epoch state
    let mut latest_epoch_state = EpochState::empty();
    latest_epoch_state.epoch = epoch;
    startup_info.latest_epoch_state = Some(latest_epoch_state);

    startup_info
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
        Some(HashValue::random()),
        0,
        ExecutionStatus::Success,
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
        TransactionStatus::Keep(ExecutionStatus::Success),
    )
}
