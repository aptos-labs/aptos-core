// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::driver::DriverConfiguration;
use aptos_config::config::{RoleType, StateSyncDriverConfig};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    HashValue, PrivateKey, Uniform,
};
use aptos_data_client::GlobalDataSummary;
use aptos_types::aggregate_signature::AggregateSignature;
use aptos_types::on_chain_config::ValidatorSet;
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
use data_streaming_service::{
    data_notification::DataNotification, data_stream::DataStreamListener, streaming_client::Epoch,
};
use event_notifications::EventNotificationListener;
use futures::channel::mpsc;
use futures::StreamExt;
use mempool_notifications::{CommittedTransaction, MempoolNotificationListener};
use move_deps::move_core_types::language_storage::TypeTag;
use rand::rngs::OsRng;
use rand::Rng;
use storage_service_types::responses::CompleteDataRange;

/// Creates a new data stream listener and notification sender pair
pub fn create_data_stream_listener() -> (mpsc::Sender<DataNotification>, DataStreamListener) {
    let (notification_sender, notification_receiver) = mpsc::channel(100);
    let data_stream_listener = DataStreamListener::new(create_random_u64(), notification_receiver);

    (notification_sender, data_stream_listener)
}

/// Creates a test epoch ending ledger info
pub fn create_epoch_ending_ledger_info() -> LedgerInfoWithSignatures {
    let ledger_info = LedgerInfo::genesis(HashValue::zero(), ValidatorSet::empty());
    LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty())
}

/// Creates a single test event
pub fn create_event(event_key: Option<EventKey>) -> ContractEvent {
    let event_key = event_key.unwrap_or_else(EventKey::random);
    ContractEvent::new(event_key, 0, TypeTag::Bool, bcs::to_bytes(&0).unwrap())
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
    LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty())
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
    LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty())
}

/// Returns an empty epoch state
pub fn create_empty_epoch_state() -> EpochState {
    EpochState::empty()
}

/// Returns an epoch state at the specified epoch
pub fn create_epoch_state(epoch: u64) -> EpochState {
    let mut epoch_state = create_empty_epoch_state();
    epoch_state.epoch = epoch;
    epoch_state
}

/// Returns a random u64
fn create_random_u64() -> u64 {
    let mut rng = OsRng;
    rng.gen()
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
        vec![create_event(None)],
        0,
        TransactionStatus::Keep(ExecutionStatus::Success),
    )
}

/// Verifies that mempool is notified about the committed transactions and
/// verifies that the event listener is notified about the committed
/// events (if it exists).
pub async fn verify_mempool_and_event_notification(
    event_listener: Option<&mut EventNotificationListener>,
    mempool_notification_listener: &mut MempoolNotificationListener,
    expected_transactions: Vec<Transaction>,
    expected_events: Vec<ContractEvent>,
) {
    // Verify mempool is notified and ack the notification
    let mempool_notification = mempool_notification_listener.select_next_some().await;
    let committed_transactions: Vec<CommittedTransaction> = expected_transactions
        .into_iter()
        .map(|txn| CommittedTransaction {
            sender: txn.as_signed_user_txn().unwrap().sender(),
            sequence_number: 0,
        })
        .collect();
    assert_eq!(mempool_notification.transactions, committed_transactions);
    let _ = mempool_notification_listener.ack_commit_notification(mempool_notification);

    // Verify the event listener is notified about the specified events
    if let Some(event_listener) = event_listener {
        let event_notification = event_listener.select_next_some().await;
        assert_eq!(event_notification.subscribed_events, expected_events);
    }
}
