// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    optimistic_fetch::OptimisticFetchRequest,
    storage::StorageReader,
    subscription::SubscriptionStreamRequests,
    tests::mock::{MockClient, MockDatabaseReader},
    StorageServiceServer,
};
use aptos_config::{
    config::StorageServiceConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use aptos_logger::Level;
use aptos_network::protocols::network::RpcError;
use aptos_storage_service_notifications::{
    StorageServiceNotificationSender, StorageServiceNotifier,
};
use aptos_storage_service_types::{
    requests::{
        DataRequest, StateValuesWithProofRequest, StorageServiceRequest,
        SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        SubscriptionStreamMetadata, TransactionsWithProofRequest,
    },
    responses::{
        CompleteDataRange, DataResponse, StorageServerSummary, StorageServiceResponse,
        TransactionDataResponseType, TransactionDataWithProofResponse,
    },
    Epoch, StorageServiceError,
};
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    chain_id::ChainId,
    contract_event::ContractEvent,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::ValidatorSet,
    proof::{TransactionAccumulatorRangeProof, TransactionInfoListWithProof},
    transaction::{
        ExecutionStatus, PersistedAuxiliaryInfo, RawTransaction, Script, SignedTransaction,
        Transaction, TransactionAuxiliaryData, TransactionInfo, TransactionListWithAuxiliaryInfos,
        TransactionListWithProof, TransactionListWithProofV2, TransactionOutput,
        TransactionOutputListWithAuxiliaryInfos, TransactionOutputListWithProof,
        TransactionOutputListWithProofV2, TransactionPayload, TransactionStatus,
    },
    validator_verifier::ValidatorVerifier,
    write_set::WriteSet,
};
use arc_swap::ArcSwap;
use bytes::Bytes;
use claims::assert_none;
use dashmap::DashMap;
use futures::channel::oneshot::Receiver;
use mockall::{
    predicate::{always, eq},
    Sequence,
};
use rand::{prelude::SliceRandom, rngs::OsRng, Rng};
use serde::Serialize;
use std::{collections::HashMap, future::Future, sync::Arc, time::Duration};
use tokio::time::timeout;

// Useful test constants
const MAX_WAIT_TIME_SECS: u64 = 60;

/// Advances the given timer by the amount of time it takes to refresh storage
pub async fn advance_storage_refresh_time(mock_time: &MockTimeService) {
    let default_storage_config = StorageServiceConfig::default();
    let cache_update_freq_ms = default_storage_config.storage_summary_refresh_interval_ms;
    mock_time.advance_ms_async(cache_update_freq_ms).await;
}

/// Creates and returns a list of data chunks that respect an epoch change
/// version (i.e., no single chunk crosses the epoch boundary). Each chunk
/// is of the form (start_version, end_version), inclusive. The list contains
/// the specified number of chunks and start at the given version.
pub fn create_data_chunks_with_epoch_boundary(
    chunk_size: u64,
    num_chunks_to_create: u64,
    start_version: u64,
    epoch_change_version: u64,
) -> Vec<(u64, u64)> {
    (0..num_chunks_to_create)
        .map(|i| {
            let chunk_start_version = start_version + (i * chunk_size) + 1;
            let chunk_end_version = chunk_start_version + chunk_size - 1;
            if chunk_end_version < epoch_change_version {
                (chunk_start_version, chunk_end_version) // The chunk is before the epoch change
            } else if chunk_start_version < epoch_change_version
                && epoch_change_version < chunk_end_version
            {
                (chunk_start_version, epoch_change_version) // The chunk would cross the epoch boundary
            } else {
                let chunk_shift_amount =
                    (chunk_start_version - epoch_change_version - 1) % chunk_size;
                (
                    chunk_start_version - chunk_shift_amount,
                    chunk_end_version - chunk_shift_amount,
                ) // The chunk is after the epoch change (shift it left)
            }
        })
        .collect()
}

/// Creates a test epoch ending ledger info
pub fn create_epoch_ending_ledger_info(epoch: u64, version: u64) -> LedgerInfoWithSignatures {
    // Create a new epoch state
    let verifier = ValidatorVerifier::from(&ValidatorSet::empty());
    let next_epoch_state = EpochState {
        epoch,
        verifier: verifier.into(),
    };

    // Create a mock ledger info with signatures
    let ledger_info = LedgerInfo::new(
        BlockInfo::new(
            epoch,
            0,
            HashValue::zero(),
            HashValue::zero(),
            version,
            0,
            Some(next_epoch_state),
        ),
        HashValue::zero(),
    );
    LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty())
}

/// Creates a test transaction output list with proof with the given sizes
pub fn create_output_list_using_sizes(
    start_version: u64,
    num_outputs: u64,
    min_bytes_per_output: u64,
    use_request_v2: bool,
) -> TransactionOutputListWithProofV2 {
    // Create a test transaction list that enforces the given size requirements
    let (transaction_list_with_proof, persisted_auxiliary_info) =
        create_transaction_list_using_sizes(
            start_version,
            num_outputs,
            min_bytes_per_output,
            false,
            use_request_v2,
        )
        .into_parts();

    // Create a test transaction and output list
    let transactions_and_outputs = transaction_list_with_proof
        .transactions
        .into_iter()
        .map(|txn| (txn, create_test_transaction_output()))
        .collect();

    TransactionOutputListWithProofV2::new(TransactionOutputListWithAuxiliaryInfos::new(
        TransactionOutputListWithProof::new(
            transactions_and_outputs,
            Some(start_version),
            transaction_list_with_proof.proof,
        ),
        persisted_auxiliary_info,
    ))
}

/// Creates a test transaction output list with proof
pub fn create_output_list_with_proof(
    start_version: u64,
    end_version: u64,
    proof_version: u64,
    use_request_v2: bool,
) -> TransactionOutputListWithProofV2 {
    let (transaction_list_with_proof, persisted_auxiliary_info) =
        create_transaction_list_with_proof(
            start_version,
            end_version,
            proof_version,
            false,
            use_request_v2,
        )
        .into_parts();
    let transactions_and_outputs = transaction_list_with_proof
        .transactions
        .iter()
        .map(|txn| (txn.clone(), create_test_transaction_output()))
        .collect();

    TransactionOutputListWithProofV2::new(TransactionOutputListWithAuxiliaryInfos::new(
        TransactionOutputListWithProof::new(
            transactions_and_outputs,
            Some(start_version),
            transaction_list_with_proof.proof,
        ),
        persisted_auxiliary_info,
    ))
}

/// Creates and returns a list of persisted auxiliary infos (if request v2 is enabled)
pub fn create_persisted_auxiliary_infos(
    start_version: u64,
    end_version: u64,
    use_request_v2: bool,
) -> Vec<PersistedAuxiliaryInfo> {
    let mut persisted_auxiliary_infos = vec![];
    for i in start_version..=end_version {
        let persisted_auxiliary_info = if use_request_v2 {
            PersistedAuxiliaryInfo::V1 {
                transaction_index: i as u32,
            }
        } else {
            PersistedAuxiliaryInfo::None
        };
        persisted_auxiliary_infos.push(persisted_auxiliary_info);
    }

    // Return the list of auxiliary infos
    persisted_auxiliary_infos
}

/// Creates a vector of entries from first_index to last_index (inclusive)
/// and shuffles the entries randomly.
pub fn create_shuffled_vector(first_index: u64, last_index: u64) -> Vec<u64> {
    let mut vector: Vec<u64> = (first_index..=last_index).collect();
    vector.shuffle(&mut rand::thread_rng());
    vector
}

/// Creates and returns a storage service config
pub fn create_storage_config(
    enable_transaction_data_v2: bool,
    enable_size_and_time_aware_chunking: bool,
) -> StorageServiceConfig {
    StorageServiceConfig {
        enable_transaction_data_v2,
        enable_size_and_time_aware_chunking,
        ..Default::default()
    }
}

/// Creates a test ledger info with signatures
pub fn create_test_ledger_info_with_sigs(epoch: u64, version: u64) -> LedgerInfoWithSignatures {
    // Create a mock ledger info with signatures
    let ledger_info = LedgerInfo::new(
        BlockInfo::new(
            epoch,
            0,
            HashValue::zero(),
            HashValue::zero(),
            version,
            0,
            None,
        ),
        HashValue::zero(),
    );
    LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty())
}

/// Creates a test user transaction
fn create_test_transaction(sequence_number: u64, code_bytes: Vec<u8>) -> Transaction {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();

    let transaction_payload = TransactionPayload::Script(Script::new(code_bytes, vec![], vec![]));
    let raw_transaction = RawTransaction::new(
        AccountAddress::random(),
        sequence_number,
        transaction_payload,
        0,
        0,
        0,
        ChainId::new(10),
    );
    let signed_transaction = SignedTransaction::new(
        raw_transaction.clone(),
        public_key,
        private_key.sign(&raw_transaction).unwrap(),
    );

    Transaction::UserTransaction(signed_transaction)
}

/// Creates a test transaction list with proof with the given sizes
pub fn create_transaction_list_using_sizes(
    start_version: u64,
    num_transactions: u64,
    min_bytes_per_transaction: u64,
    include_events: bool,
    use_request_v2: bool,
) -> TransactionListWithProofV2 {
    // Generate random bytes of the given size
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..min_bytes_per_transaction)
        .map(|_| rng.r#gen::<u8>())
        .collect();

    // Create the transaction data
    let mut transactions = vec![];
    let mut transaction_infos = vec![];
    let mut events = vec![];
    let end_version = start_version + num_transactions - 1;
    for sequence_number in start_version..=end_version {
        // Create a transaction info
        let transaction_info = TransactionInfo::new(
            HashValue::zero(),
            HashValue::zero(),
            HashValue::zero(),
            None,
            0,
            ExecutionStatus::Success,
            None,
        );

        // Save the transaction data
        transactions.push(create_test_transaction(
            sequence_number,
            random_bytes.clone(),
        ));
        transaction_infos.push(transaction_info);
        events.push(vec![]);
    }

    // Create a transaction list with proof
    let events = if include_events { Some(events) } else { None };
    let transaction_info_list_with_proof = TransactionInfoListWithProof::new(
        TransactionAccumulatorRangeProof::new_empty(),
        transaction_infos,
    );
    let transaction_list_with_proof = TransactionListWithProof::new(
        transactions,
        events,
        Some(start_version),
        transaction_info_list_with_proof,
    );

    // Create the auxiliary infos
    let auxiliary_infos =
        create_persisted_auxiliary_infos(start_version, end_version, use_request_v2);

    TransactionListWithProofV2::new(TransactionListWithAuxiliaryInfos::new(
        transaction_list_with_proof,
        auxiliary_infos,
    ))
}

/// Creates a test transaction output list with proof
pub fn create_transaction_list_with_proof(
    start_version: u64,
    end_version: u64,
    _proof_version: u64,
    include_events: bool,
    use_request_v2: bool,
) -> TransactionListWithProofV2 {
    // Create the transaction data
    let mut transactions = vec![];
    let mut transaction_infos = vec![];
    let mut events = vec![];
    for sequence_number in start_version..=end_version {
        // Create a transaction info
        let transaction_info = TransactionInfo::new(
            HashValue::zero(),
            HashValue::zero(),
            HashValue::zero(),
            None,
            0,
            ExecutionStatus::Success,
            None,
        );

        // Save the transaction data
        transactions.push(create_test_transaction(sequence_number, vec![]));
        transaction_infos.push(transaction_info);
        events.push(vec![]);
    }

    // Create a transaction list with proof
    let events = if include_events { Some(events) } else { None };
    let transaction_info_list_with_proof = TransactionInfoListWithProof::new(
        TransactionAccumulatorRangeProof::new_empty(),
        transaction_infos,
    );
    let transaction_list_with_proof = TransactionListWithProof::new(
        transactions,
        events,
        Some(start_version),
        transaction_info_list_with_proof,
    );

    // Create the auxiliary infos
    let auxiliary_infos =
        create_persisted_auxiliary_infos(start_version, end_version, use_request_v2);

    TransactionListWithProofV2::new(TransactionListWithAuxiliaryInfos::new(
        transaction_list_with_proof,
        auxiliary_infos,
    ))
}

/// Creates a test transaction output
fn create_test_transaction_output() -> TransactionOutput {
    // Create the event list
    let mut events = vec![];
    for _ in 0..10 {
        let event =
            ContractEvent::new_v2_with_type_tag_str("0x1::dkg::DKGStartEvent", b"dkg_2".to_vec());
        events.push(event);
    }

    // Create the transaction output
    TransactionOutput::new(
        WriteSet::default(),
        events,
        0,
        TransactionStatus::Keep(ExecutionStatus::Success),
        TransactionAuxiliaryData::default(),
    )
}

/// Creates a new storage service config with the limit
/// configured to be the size of an output list or transaction
/// list (depending on if `fallback_to_transactions` is set).
pub fn configure_network_chunk_limit(
    fallback_to_transactions: bool,
    output_list_with_proof: &TransactionOutputListWithProofV2,
    transaction_list_with_proof: &TransactionListWithProofV2,
    enable_transaction_data_v2: bool,
    enable_size_and_time_aware_chunking: bool,
) -> StorageServiceConfig {
    // Determine the max network chunk bytes
    let max_network_chunk_bytes = if fallback_to_transactions {
        if enable_size_and_time_aware_chunking {
            let (transaction_list, persisted_auxiliary_infos) =
                transaction_list_with_proof.clone().into_parts();

            // Calculate the serialized size of each component
            let mut serialized_size = 0;
            for transaction in transaction_list.transactions {
                serialized_size += get_num_serialized_bytes(&transaction) + 1;
            }
            for transaction_info in transaction_list.proof.transaction_infos {
                serialized_size += get_num_serialized_bytes(&transaction_info) + 1;
            }
            if let Some(events) = transaction_list.events {
                for events in events {
                    serialized_size += get_num_serialized_bytes(&events) + 1;
                }
            }
            for auxiliary_info in persisted_auxiliary_infos {
                serialized_size += get_num_serialized_bytes(&auxiliary_info) + 1;
            }
            serialized_size + 1
        } else {
            let response = TransactionDataWithProofResponse {
                transaction_data_response_type: TransactionDataResponseType::TransactionData,
                transaction_list_with_proof: Some(transaction_list_with_proof.clone()),
                transaction_output_list_with_proof: None,
            };
            bcs::serialized_size(&response).unwrap() as u64 + 1
        }
    } else if enable_size_and_time_aware_chunking {
        let (output_list, persisted_auxiliary_infos) = output_list_with_proof.clone().into_parts();

        // Calculate the serialized size of each component
        let mut serialized_size = 0;
        for (transaction, output) in output_list.transactions_and_outputs {
            serialized_size += get_num_serialized_bytes(&transaction) + 1;
            serialized_size += get_num_serialized_bytes(&output) + 1;
        }
        for transaction_info in output_list.proof.transaction_infos {
            serialized_size += get_num_serialized_bytes(&transaction_info) + 1;
        }
        for auxiliary_info in persisted_auxiliary_infos {
            serialized_size += get_num_serialized_bytes(&auxiliary_info) + 1;
        }
        serialized_size + 1
    } else {
        let response = TransactionDataWithProofResponse {
            transaction_data_response_type: TransactionDataResponseType::TransactionOutputData,
            transaction_list_with_proof: None,
            transaction_output_list_with_proof: Some(output_list_with_proof.clone()),
        };
        bcs::serialized_size(&response).unwrap() as u64 + 1
    };

    // Create the storage service config
    StorageServiceConfig {
        max_network_chunk_bytes,
        enable_transaction_data_v2,
        enable_size_and_time_aware_chunking,
        max_network_chunk_bytes_v2: max_network_chunk_bytes, // Use the same limit for v2
        ..Default::default()
    }
}

/// Serializes the given data and returns the number of serialized bytes
fn get_num_serialized_bytes<T: ?Sized + Serialize>(data: &T) -> u64 {
    bcs::serialized_size(data).unwrap() as u64
}

/// Advances the mock time service by the specified number of milliseconds
pub async fn elapse_time(time_ms: u64, time_service: &TimeService) {
    time_service
        .clone()
        .into_mock()
        .advance_async(Duration::from_millis(time_ms))
        .await;
}

/// Sets an expectation on the given mock db for a call to fetch an epoch change proof
pub fn expect_get_epoch_ending_ledger_infos(
    mock_db: &mut MockDatabaseReader,
    start_epoch: u64,
    expected_end_epoch: u64,
    epoch_change_proof: EpochChangeProof,
    use_size_and_time_aware_chunking: bool,
) {
    // If size and time-aware chunking are disabled, expect the legacy implementation
    if !use_size_and_time_aware_chunking {
        mock_db
            .expect_get_epoch_ending_ledger_infos()
            .times(1)
            .with(eq(start_epoch), eq(expected_end_epoch))
            .returning(move |_, _| Ok(epoch_change_proof.clone()));
        return;
    }

    // Expect a call to get an epoch ending iterator
    let epoch_ending_iterator = epoch_change_proof.ledger_info_with_sigs.into_iter().map(Ok);
    mock_db
        .expect_get_epoch_ending_ledger_info_iterator()
        .times(1)
        .with(eq(start_epoch), eq(expected_end_epoch))
        .returning(move |_, _| Ok(Box::new(epoch_ending_iterator.clone())));
}

/// Sets an expectation on the given mock db for a call to fetch transaction outputs
pub fn expect_get_transaction_outputs(
    mock_db: &mut MockDatabaseReader,
    start_version: u64,
    num_items: u64,
    proof_version: u64,
    output_list: TransactionOutputListWithProofV2,
    use_size_and_time_aware_chunking: bool,
) {
    // If size and time-aware chunking are disabled, expect the legacy implementation
    if !use_size_and_time_aware_chunking {
        mock_db
            .expect_get_transaction_outputs()
            .times(1)
            .with(eq(start_version), eq(num_items), eq(proof_version))
            .returning(move |_, _, _| Ok(output_list.clone()));
        return;
    }

    // Expect a call to get a transaction iterator
    let (output_list_with_proof, persisted_auxiliary_infos) = output_list.into_parts();
    let mut expectation_sequence = Sequence::new();
    let transaction_iterator = output_list_with_proof
        .clone()
        .transactions_and_outputs
        .into_iter()
        .map(|(transaction, _)| Ok(transaction));
    mock_db
        .expect_get_transaction_iterator()
        .times(1)
        .with(eq(start_version), eq(num_items))
        .returning(move |_, _| Ok(Box::new(transaction_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get a transaction info iterator
    let transaction_info_iterator = output_list_with_proof
        .clone()
        .proof
        .transaction_infos
        .into_iter()
        .map(Ok);
    mock_db
        .expect_get_transaction_info_iterator()
        .times(1)
        .with(eq(start_version), eq(num_items))
        .returning(move |_, _| Ok(Box::new(transaction_info_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get a write set iterator
    let write_set_iterator = output_list_with_proof
        .clone()
        .transactions_and_outputs
        .into_iter()
        .map(|(_, output)| Ok(output.write_set().clone()));
    mock_db
        .expect_get_write_set_iterator()
        .times(1)
        .with(eq(start_version), eq(num_items))
        .returning(move |_, _| Ok(Box::new(write_set_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get an event iterator
    let events_iterator = output_list_with_proof
        .clone()
        .transactions_and_outputs
        .into_iter()
        .map(|(_, output)| Ok(output.events().to_vec()));
    mock_db
        .expect_get_events_iterator()
        .times(1)
        .with(eq(start_version), eq(num_items))
        .returning(move |_, _| Ok(Box::new(events_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get an auxiliary data iterator
    let auxiliary_data_iterator = output_list_with_proof
        .clone()
        .transactions_and_outputs
        .into_iter()
        .map(|(_, output)| Ok(output.auxiliary_data().clone()));
    mock_db
        .expect_get_auxiliary_data_iterator()
        .times(1)
        .with(eq(start_version), eq(num_items))
        .returning(move |_, _| Ok(Box::new(auxiliary_data_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get a persisted auxiliary info iterator
    let persisted_auxiliary_info_iterator = persisted_auxiliary_infos.clone().into_iter().map(Ok);
    mock_db
        .expect_get_persisted_auxiliary_info_iterator()
        .times(1)
        .with(eq(start_version), eq(num_items as usize))
        .returning(move |_, _| Ok(Box::new(persisted_auxiliary_info_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get the transaction accumulator range proof
    let accumulator_range_proof = output_list_with_proof
        .proof
        .ledger_info_to_transaction_infos_proof;
    mock_db
        .expect_get_transaction_accumulator_range_proof()
        .times(1)
        .with(eq(start_version), always(), eq(proof_version))
        .returning(move |_, _, _| Ok(accumulator_range_proof.clone()))
        .in_sequence(&mut expectation_sequence);
}

/// Sets an expectation on the given mock db for a call to fetch transactions
pub fn expect_get_transactions(
    mock_db: &mut MockDatabaseReader,
    start_version: u64,
    num_items: u64,
    proof_version: u64,
    include_events: bool,
    transaction_list: TransactionListWithProofV2,
    use_size_and_time_aware_chunking: bool,
) {
    // If size and time-aware chunking are disabled, expect the legacy implementation
    if !use_size_and_time_aware_chunking {
        mock_db
            .expect_get_transactions()
            .times(1)
            .with(
                eq(start_version),
                eq(num_items),
                eq(proof_version),
                eq(include_events),
            )
            .returning(move |_, _, _, _| Ok(transaction_list.clone()));
        return;
    }

    // Expect a call to get a transaction iterator
    let (transaction_list_with_proof, persisted_auxiliary_infos) = transaction_list.into_parts();
    let mut expectation_sequence = Sequence::new();
    let transaction_iterator = transaction_list_with_proof
        .clone()
        .transactions
        .into_iter()
        .map(Ok);
    mock_db
        .expect_get_transaction_iterator()
        .times(1)
        .with(eq(start_version), eq(num_items))
        .returning(move |_, _| Ok(Box::new(transaction_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get a transaction info iterator
    let transaction_info_iterator = transaction_list_with_proof
        .clone()
        .proof
        .transaction_infos
        .into_iter()
        .map(Ok);
    mock_db
        .expect_get_transaction_info_iterator()
        .times(1)
        .with(eq(start_version), eq(num_items))
        .returning(move |_, _| Ok(Box::new(transaction_info_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get an event iterator (only if events are requested)
    if include_events {
        let events_iterator = transaction_list_with_proof
            .clone()
            .events
            .unwrap()
            .into_iter()
            .map(Ok);
        mock_db
            .expect_get_events_iterator()
            .times(1)
            .with(eq(start_version), eq(num_items))
            .returning(move |_, _| Ok(Box::new(events_iterator.clone())))
            .in_sequence(&mut expectation_sequence);
    }

    // Expect a call to get a persisted auxiliary info iterator
    let persisted_auxiliary_info_iterator = persisted_auxiliary_infos.clone().into_iter().map(Ok);
    mock_db
        .expect_get_persisted_auxiliary_info_iterator()
        .times(1)
        .with(eq(start_version), eq(num_items as usize))
        .returning(move |_, _| Ok(Box::new(persisted_auxiliary_info_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get the transaction accumulator range proof
    let accumulator_range_proof = transaction_list_with_proof
        .proof
        .ledger_info_to_transaction_infos_proof;
    mock_db
        .expect_get_transaction_accumulator_range_proof()
        .times(1)
        .with(eq(start_version), always(), eq(proof_version))
        .returning(move |_, _, _| Ok(accumulator_range_proof.clone()))
        .in_sequence(&mut expectation_sequence);
}

/// Extracts the peer and network ids from an optional peer network id
pub fn extract_peer_and_network_id(
    peer_network_id: Option<PeerNetworkId>,
) -> (Option<AccountAddress>, Option<NetworkId>) {
    if let Some(peer_network_id) = peer_network_id {
        (
            Some(peer_network_id.peer_id()),
            Some(peer_network_id.network_id()),
        )
    } else {
        (None, None)
    }
}

/// This function forces a cache update notification to be sent
/// to the optimistic fetch and subscription handlers.
///
/// This can be done in two ways: (i) a state sync notification
/// is sent to the storage service, invoking the handlers; or (ii)
/// enough time elapses that the handlers execute manually.
pub async fn force_cache_update_notification(
    mock_client: &mut MockClient,
    mock_time: &MockTimeService,
    storage_service_notifier: &StorageServiceNotifier,
    always_advance_time: bool,
    wait_for_storage_cache_update: bool,
) {
    // Generate a random number and if the number is even, send
    // a state sync notification. Otherwise, advance enough time
    // to refresh the storage cache manually.
    let random_number: u8 = OsRng.r#gen();
    if always_advance_time || random_number % 2 != 0 {
        // Advance the storage refresh time manually
        advance_storage_refresh_time(mock_time).await;
    } else {
        // Send a state sync notification with the highest synced version
        storage_service_notifier
            .notify_new_commit(random_number as u64)
            .await
            .unwrap();
    }

    // Wait for the storage server to refresh the cached summary
    if wait_for_storage_cache_update {
        wait_for_cached_summary_update(
            mock_client,
            mock_time,
            StorageServerSummary::default(),
            true,
        )
        .await;
    }
}

/// This function forces the optimistic fetch handler to work
pub async fn force_optimistic_fetch_handler_to_run(
    mock_client: &mut MockClient,
    mock_time: &MockTimeService,
    storage_service_notifier: &StorageServiceNotifier,
) {
    force_cache_update_notification(
        mock_client,
        mock_time,
        storage_service_notifier,
        false,
        true,
    )
    .await;
}

/// This function forces the subscription handler to work
pub async fn force_subscription_handler_to_run(
    mock_client: &mut MockClient,
    mock_time: &MockTimeService,
    storage_service_notifier: &StorageServiceNotifier,
) {
    force_cache_update_notification(
        mock_client,
        mock_time,
        storage_service_notifier,
        false,
        true,
    )
    .await;
}

/// Sends a number of states request and processes the response
pub async fn get_number_of_states(
    mock_client: &mut MockClient,
    version: u64,
    use_compression: bool,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = DataRequest::GetNumberOfStatesAtVersion(version);
    send_storage_request(mock_client, use_compression, data_request).await
}

/// Generates and returns a random number (u64)
pub fn get_random_u64() -> u64 {
    OsRng.r#gen()
}

/// Sends a state values with proof request and processes the response
pub async fn get_state_values_with_proof(
    mock_client: &mut MockClient,
    version: u64,
    start_index: u64,
    end_index: u64,
    use_compression: bool,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = DataRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
        version,
        start_index,
        end_index,
    });
    send_storage_request(mock_client, use_compression, data_request).await
}

/// Sends a transactions with proof request and processes the response
pub async fn get_transactions_with_proof(
    mock_client: &mut MockClient,
    start_version: u64,
    end_version: u64,
    proof_version: u64,
    include_events: bool,
    use_compression: bool,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = if use_request_v2 {
        DataRequest::get_transaction_data_with_proof(
            proof_version,
            start_version,
            end_version,
            include_events,
            max_response_bytes_v2,
        )
    } else {
        DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
            proof_version,
            start_version,
            end_version,
            include_events,
        })
    };
    send_storage_request(mock_client, use_compression, data_request).await
}

/// Initializes the Aptos logger for tests
pub fn initialize_logger() {
    aptos_logger::Logger::builder()
        .is_async(false)
        .level(Level::Debug)
        .build();
}

/// Sends a batch of transaction output requests and
/// returns the response receivers for each request.
pub async fn send_output_subscription_request_batch(
    mock_client: &mut MockClient,
    peer_network_id: PeerNetworkId,
    first_stream_request_index: u64,
    last_stream_request_index: u64,
    stream_id: u64,
    peer_version: u64,
    peer_epoch: u64,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> HashMap<u64, Receiver<Result<Bytes, RpcError>>> {
    // Shuffle the stream request indices to emulate out of order requests
    let stream_request_indices =
        create_shuffled_vector(first_stream_request_index, last_stream_request_index);

    // Send the requests and gather the response receivers
    let mut response_receivers = HashMap::new();
    for stream_request_index in stream_request_indices {
        // Send the transaction output subscription request
        let response_receiver = subscribe_to_transaction_outputs_for_peer(
            mock_client,
            peer_version,
            peer_epoch,
            stream_id,
            stream_request_index,
            Some(peer_network_id),
            use_request_v2,
            max_response_bytes_v2,
        )
        .await;

        // Save the response receiver
        response_receivers.insert(stream_request_index, response_receiver);
    }

    response_receivers
}

/// Sends the given storage request to the given client
pub async fn send_storage_request(
    mock_client: &mut MockClient,
    use_compression: bool,
    data_request: DataRequest,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let storage_request = StorageServiceRequest::new(data_request, use_compression);
    mock_client.process_request(storage_request).await
}

/// Creates and sends a request to subscribe to new transactions or outputs
pub async fn subscribe_to_transactions_or_outputs(
    mock_client: &mut MockClient,
    known_version: u64,
    known_epoch: u64,
    include_events: bool,
    max_num_output_reductions: u64,
    stream_id: u64,
    stream_index: u64,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Receiver<Result<bytes::Bytes, aptos_network::protocols::network::RpcError>> {
    subscribe_to_transactions_or_outputs_for_peer(
        mock_client,
        known_version,
        known_epoch,
        include_events,
        max_num_output_reductions,
        stream_id,
        stream_index,
        None,
        use_request_v2,
        max_response_bytes_v2,
    )
    .await
}

/// Creates and sends a request to subscribe to new transactions or outputs for the specified peer
pub async fn subscribe_to_transactions_or_outputs_for_peer(
    mock_client: &mut MockClient,
    known_version_at_stream_start: u64,
    known_epoch_at_stream_start: u64,
    include_events: bool,
    max_num_output_reductions: u64,
    subscription_stream_id: u64,
    subscription_stream_index: u64,
    peer_network_id: Option<PeerNetworkId>,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Receiver<Result<Bytes, RpcError>> {
    // Create the data request
    let subscription_stream_metadata = SubscriptionStreamMetadata {
        known_version_at_stream_start,
        known_epoch_at_stream_start,
        subscription_stream_id,
    };
    let data_request = if use_request_v2 {
        DataRequest::subscribe_transaction_or_output_data_with_proof(
            subscription_stream_metadata,
            subscription_stream_index,
            include_events,
            max_response_bytes_v2,
        )
    } else {
        DataRequest::SubscribeTransactionsOrOutputsWithProof(
            SubscribeTransactionsOrOutputsWithProofRequest {
                subscription_stream_metadata,
                include_events,
                max_num_output_reductions,
                subscription_stream_index,
            },
        )
    };
    let storage_request = StorageServiceRequest::new(data_request, true);

    // Send the request
    let (peer_id, network_id) = extract_peer_and_network_id(peer_network_id);
    mock_client
        .send_request(storage_request, peer_id, network_id)
        .await
}

/// Creates and sends a request to subscribe to new transaction outputs
pub async fn subscribe_to_transaction_outputs(
    mock_client: &mut MockClient,
    known_version: u64,
    known_epoch: u64,
    stream_id: u64,
    stream_index: u64,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Receiver<Result<Bytes, RpcError>> {
    subscribe_to_transaction_outputs_for_peer(
        mock_client,
        known_version,
        known_epoch,
        stream_id,
        stream_index,
        None,
        use_request_v2,
        max_response_bytes_v2,
    )
    .await
}

/// Creates and sends a request to subscribe to new transaction outputs for the specified peer
pub async fn subscribe_to_transaction_outputs_for_peer(
    mock_client: &mut MockClient,
    known_version_at_stream_start: u64,
    known_epoch_at_stream_start: u64,
    subscription_stream_id: u64,
    subscription_stream_index: u64,
    peer_network_id: Option<PeerNetworkId>,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Receiver<Result<Bytes, RpcError>> {
    // Create the data request
    let subscription_stream_metadata = SubscriptionStreamMetadata {
        known_version_at_stream_start,
        known_epoch_at_stream_start,
        subscription_stream_id,
    };
    let data_request = if use_request_v2 {
        DataRequest::subscribe_transaction_output_data_with_proof(
            subscription_stream_metadata,
            subscription_stream_index,
            max_response_bytes_v2,
        )
    } else {
        DataRequest::SubscribeTransactionOutputsWithProof(
            SubscribeTransactionOutputsWithProofRequest {
                subscription_stream_metadata,
                subscription_stream_index,
            },
        )
    };
    let storage_request = StorageServiceRequest::new(data_request, true);

    // Send the request
    let (peer_id, network_id) = extract_peer_and_network_id(peer_network_id);
    mock_client
        .send_request(storage_request, peer_id, network_id)
        .await
}

/// Creates and sends a request to subscribe to new transactions
pub async fn subscribe_to_transactions(
    mock_client: &mut MockClient,
    known_version: u64,
    known_epoch: u64,
    include_events: bool,
    stream_id: u64,
    stream_index: u64,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Receiver<Result<Bytes, RpcError>> {
    subscribe_to_transactions_for_peer(
        mock_client,
        known_version,
        known_epoch,
        include_events,
        stream_id,
        stream_index,
        None,
        use_request_v2,
        max_response_bytes_v2,
    )
    .await
}

/// Creates and sends a request to subscribe to new transactions for the specified peer
pub async fn subscribe_to_transactions_for_peer(
    mock_client: &mut MockClient,
    known_version_at_stream_start: u64,
    known_epoch_at_stream_start: u64,
    include_events: bool,
    subscription_stream_id: u64,
    subscription_stream_index: u64,
    peer_network_id: Option<PeerNetworkId>,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Receiver<Result<Bytes, RpcError>> {
    // Create the data request
    let subscription_stream_metadata = SubscriptionStreamMetadata {
        known_version_at_stream_start,
        known_epoch_at_stream_start,
        subscription_stream_id,
    };
    let data_request = if use_request_v2 {
        DataRequest::subscribe_transaction_data_with_proof(
            subscription_stream_metadata,
            subscription_stream_index,
            include_events,
            max_response_bytes_v2,
        )
    } else {
        DataRequest::SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest {
            subscription_stream_metadata,
            include_events,
            subscription_stream_index,
        })
    };
    let storage_request = StorageServiceRequest::new(data_request, true);

    // Send the request
    let (peer_id, network_id) = extract_peer_and_network_id(peer_network_id);
    mock_client
        .send_request(storage_request, peer_id, network_id)
        .await
}

/// Updates the storage server summary with the specified data
pub fn update_storage_server_summary(
    storage_server: &mut StorageServiceServer<StorageReader>,
    highest_synced_version: u64,
    highest_synced_epoch: Epoch,
) {
    // Create a storage server summary
    let mut storage_server_summary = StorageServerSummary::default();

    // Set the highest synced ledger info
    let data_summary = &mut storage_server_summary.data_summary;
    data_summary.synced_ledger_info = Some(create_epoch_ending_ledger_info(
        highest_synced_epoch,
        highest_synced_version,
    ));

    // Set the epoch ending ledger info range
    let data_range = CompleteDataRange::new(0, highest_synced_epoch).unwrap();
    data_summary.epoch_ending_ledger_infos = Some(data_range);

    // Set the transaction and state ranges
    let data_range = CompleteDataRange::new(0, highest_synced_version).unwrap();
    data_summary.states = Some(data_range);
    data_summary.transactions = Some(data_range);
    data_summary.transaction_outputs = Some(data_range);

    // Update the storage server summary
    storage_server
        .cached_storage_server_summary
        .store(Arc::new(storage_server_summary));
}

/// Updates the storage server summary cache with new data
/// and returns the synced ledger info.
pub fn update_storage_summary_cache(
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    highest_synced_version: u64,
    highest_synced_epoch: u64,
) -> LedgerInfoWithSignatures {
    // Create the storage server summary and synced ledger info
    let mut storage_server_summary = StorageServerSummary::default();
    let highest_synced_ledger_info =
        create_test_ledger_info_with_sigs(highest_synced_epoch, highest_synced_version);

    // Update the epoch ending ledger infos and synced ledger info
    storage_server_summary
        .data_summary
        .epoch_ending_ledger_infos = Some(CompleteDataRange::new(0, highest_synced_epoch).unwrap());
    storage_server_summary.data_summary.synced_ledger_info =
        Some(highest_synced_ledger_info.clone());

    // Update the cached storage server summary
    cached_storage_server_summary.store(Arc::new(storage_server_summary));

    highest_synced_ledger_info
}

/// Verifies that the peer has an active subscription stream
/// and that the stream has the appropriate ID.
pub fn verify_active_stream_id_for_peer(
    active_subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    peer_network_id: PeerNetworkId,
    new_stream_id: u64,
) {
    // Get the subscription stream requests for the peer
    let subscription = active_subscriptions.get(&peer_network_id).unwrap();
    let subscription_stream_requests = subscription.value();

    // Verify the stream ID is correct
    assert_eq!(
        subscription_stream_requests.subscription_stream_id(),
        new_stream_id
    );
}

/// Verifies that a new transaction outputs with proof response is received
/// and that the response contains the correct data.
pub async fn verify_new_transaction_outputs_with_proof(
    mock_client: &mut MockClient,
    receiver: Receiver<Result<Bytes, RpcError>>,
    use_request_v2: bool,
    expected_output_list_with_proof: TransactionOutputListWithProofV2,
    expected_ledger_info: LedgerInfoWithSignatures,
) {
    // Get the data response
    let storage_service_response = mock_client.wait_for_response(receiver).await.unwrap();
    let data_response = storage_service_response.get_data_response().unwrap();

    // Verify the response type (v1 or v2)
    match &data_response {
        DataResponse::NewTransactionOutputsWithProof(_) => assert!(!use_request_v2),
        DataResponse::NewTransactionDataWithProof(_) => {
            assert!(use_request_v2)
        },
        _ => panic!(
            "Expected new transaction outputs with proof but got: {:?}",
            data_response
        ),
    }

    // Verify the response data
    match data_response {
        DataResponse::NewTransactionOutputsWithProof((outputs_with_proof, ledger_info)) => {
            assert_eq!(
                outputs_with_proof,
                expected_output_list_with_proof
                    .get_output_list_with_proof()
                    .clone()
            );
            assert_eq!(ledger_info, expected_ledger_info);
        },
        DataResponse::NewTransactionDataWithProof(new_transaction_data_with_proof_response) => {
            // Verify the data type
            assert_eq!(
                new_transaction_data_with_proof_response.transaction_data_response_type,
                TransactionDataResponseType::TransactionOutputData
            );

            // Verify the ledger info
            assert_eq!(
                new_transaction_data_with_proof_response.ledger_info_with_signatures,
                expected_ledger_info
            );

            // Verify the transactions
            assert!(new_transaction_data_with_proof_response
                .transaction_list_with_proof
                .is_none());

            assert_eq!(
                new_transaction_data_with_proof_response
                    .transaction_output_list_with_proof
                    .unwrap(),
                expected_output_list_with_proof
            );
        },
        _ => panic!(
            "Expected new transaction outputs with proof but got: {:?}",
            data_response
        ),
    }
}

/// Verifies that a new transactions with proof response is received
/// and that the response contains the correct data.
pub async fn verify_new_transactions_with_proof(
    mock_client: &mut MockClient,
    receiver: Receiver<Result<Bytes, RpcError>>,
    use_request_v2: bool,
    expected_transactions_with_proof: TransactionListWithProofV2,
    expected_ledger_info: LedgerInfoWithSignatures,
) {
    // Get the data response
    let storage_service_response = mock_client.wait_for_response(receiver).await.unwrap();
    let data_response = storage_service_response.get_data_response().unwrap();

    // Verify the response type (v1 or v2)
    match &data_response {
        DataResponse::NewTransactionsWithProof(_) => assert!(!use_request_v2),
        DataResponse::NewTransactionDataWithProof(_) => {
            assert!(use_request_v2)
        },
        _ => panic!(
            "Expected new transaction with proof but got: {:?}",
            data_response
        ),
    }

    // Verify the response data
    match data_response {
        DataResponse::NewTransactionsWithProof((transactions_with_proof, ledger_info)) => {
            assert_eq!(
                transactions_with_proof,
                expected_transactions_with_proof
                    .get_transaction_list_with_proof()
                    .clone()
            );
            assert_eq!(ledger_info, expected_ledger_info);
        },
        DataResponse::NewTransactionDataWithProof(new_transaction_data_with_proof_response) => {
            // Verify the data type
            assert_eq!(
                new_transaction_data_with_proof_response.transaction_data_response_type,
                TransactionDataResponseType::TransactionData
            );

            // Verify the ledger info
            assert_eq!(
                new_transaction_data_with_proof_response.ledger_info_with_signatures,
                expected_ledger_info
            );

            // Verify the outputs
            assert!(new_transaction_data_with_proof_response
                .transaction_output_list_with_proof
                .is_none());

            assert_eq!(
                new_transaction_data_with_proof_response
                    .transaction_list_with_proof
                    .unwrap(),
                expected_transactions_with_proof
            );
        },
        _ => panic!(
            "Expected new transaction with proof but got: {:?}",
            data_response
        ),
    }
}

/// Verifies that a new transactions or outputs with proof response is received
/// and that the response contains the correct data.
pub async fn verify_new_transactions_or_outputs_with_proof(
    mock_client: &mut MockClient,
    receiver: Receiver<Result<Bytes, RpcError>>,
    use_request_v2: bool,
    expected_transaction_list_with_proof: Option<TransactionListWithProofV2>,
    expected_output_list_with_proof: Option<TransactionOutputListWithProofV2>,
    expected_ledger_info: LedgerInfoWithSignatures,
) {
    // Get the data response
    let storage_service_response = mock_client.wait_for_response(receiver).await.unwrap();
    let data_response = storage_service_response.get_data_response().unwrap();

    // Verify the response type (v1 or v2)
    match &data_response {
        DataResponse::NewTransactionsOrOutputsWithProof(_) => assert!(!use_request_v2),
        DataResponse::NewTransactionDataWithProof(_) => {
            assert!(use_request_v2)
        },
        _ => panic!(
            "Expected new transactions or outputs with proof but got: {:?}",
            data_response
        ),
    }

    // Verify the response data
    match data_response {
        DataResponse::NewTransactionsOrOutputsWithProof((
            (transactions_with_proof, outputs_with_proof),
            ledger_info,
        )) => {
            // Verify the ledger info
            assert_eq!(ledger_info, expected_ledger_info);

            // Verify the transactions or outputs
            assert_eq!(
                transactions_with_proof,
                expected_transaction_list_with_proof
                    .map(|t| t.get_transaction_list_with_proof().clone())
            );
            assert_eq!(
                outputs_with_proof,
                expected_output_list_with_proof.map(|t| t.get_output_list_with_proof().clone())
            );
        },
        DataResponse::NewTransactionDataWithProof(new_transaction_data_with_proof_response) => {
            // Verify the ledger info
            assert_eq!(
                new_transaction_data_with_proof_response.ledger_info_with_signatures,
                expected_ledger_info
            );

            // Verify the transactions or outputs
            if let Some(transactions_with_proof_v2) =
                new_transaction_data_with_proof_response.transaction_list_with_proof
            {
                assert_eq!(
                    new_transaction_data_with_proof_response.transaction_data_response_type,
                    TransactionDataResponseType::TransactionData,
                );
                assert!(new_transaction_data_with_proof_response
                    .transaction_output_list_with_proof
                    .is_none());
                assert_eq!(
                    transactions_with_proof_v2,
                    expected_transaction_list_with_proof.unwrap()
                );
            } else if let Some(outputs_with_proof_v2) =
                new_transaction_data_with_proof_response.transaction_output_list_with_proof
            {
                assert_eq!(
                    new_transaction_data_with_proof_response.transaction_data_response_type,
                    TransactionDataResponseType::TransactionOutputData,
                );
                assert!(new_transaction_data_with_proof_response
                    .transaction_list_with_proof
                    .is_none());
                assert_eq!(
                    outputs_with_proof_v2,
                    expected_output_list_with_proof.unwrap()
                );
            } else {
                panic!("Expected either transactions or outputs with proof, but got neither!");
            }
        },
        _ => panic!(
            "Expected new transactions or outputs with proof but got: {:?}",
            data_response
        ),
    }
}

/// Verifies the response for a transaction with proof request
pub fn verify_transaction_with_proof_response(
    use_request_v2: bool,
    transaction_list_with_proof: TransactionListWithProofV2,
    response: StorageServiceResponse,
) {
    // Get the data response
    let data_response = response.get_data_response().unwrap();

    // Verify the response type (v1 or v2)
    match &data_response {
        DataResponse::TransactionsWithProof(_) => assert!(!use_request_v2),
        DataResponse::TransactionDataWithProof(_) => {
            assert!(use_request_v2)
        },
        _ => panic!(
            "Expected transactions with proof but got: {:?}",
            data_response
        ),
    }

    // Verify the response data
    match data_response {
        DataResponse::TransactionsWithProof(transactions_with_proof) => {
            assert_eq!(
                transactions_with_proof,
                transaction_list_with_proof
                    .get_transaction_list_with_proof()
                    .clone()
            )
        },
        DataResponse::TransactionDataWithProof(transaction_data_with_proof) => {
            // Verify the data type
            assert_eq!(
                transaction_data_with_proof.transaction_data_response_type,
                TransactionDataResponseType::TransactionData
            );

            // Verify the outputs
            assert!(transaction_data_with_proof
                .transaction_output_list_with_proof
                .is_none());

            assert_eq!(
                transaction_data_with_proof
                    .transaction_list_with_proof
                    .unwrap(),
                transaction_list_with_proof
            );
        },
        _ => panic!("Expected transactions with proof but got: {:?}", response),
    };
}

/// Verifies that no subscription responses have been received yet
pub fn verify_no_subscription_responses(
    response_receivers: &mut HashMap<u64, Receiver<Result<Bytes, RpcError>>>,
) {
    for response_receiver in response_receivers.values_mut() {
        assert_none!(response_receiver.try_recv().unwrap());
    }
}

/// Verifies that a response is received for a given stream request index
/// and that the response contains the correct data.
pub async fn verify_output_subscription_response(
    expected_output_lists_with_proofs: Vec<TransactionOutputListWithProofV2>,
    expected_target_ledger_info: LedgerInfoWithSignatures,
    mock_client: &mut MockClient,
    response_receivers: &mut HashMap<u64, Receiver<Result<Bytes, RpcError>>>,
    stream_request_index: u64,
    use_request_v2: bool,
) {
    let response_receiver = response_receivers.remove(&stream_request_index).unwrap();
    verify_new_transaction_outputs_with_proof(
        mock_client,
        response_receiver,
        use_request_v2,
        expected_output_lists_with_proofs[stream_request_index as usize].clone(),
        expected_target_ledger_info,
    )
    .await;
}

/// Verifies the state of an active subscription stream entry.
/// This is useful for manually testing internal logic.
pub fn verify_subscription_stream_entry(
    active_subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    peer_network_id: PeerNetworkId,
    num_requests_per_batch: u64,
    peer_known_version: u64,
    expected_epoch: u64,
    max_transaction_output_chunk_size: u64,
) {
    // Get the subscription stream for the specified peer
    let mut subscription = active_subscriptions.get_mut(&peer_network_id).unwrap();
    let subscription_stream_requests = subscription.value_mut();

    // Get the next index to serve on the stream
    let next_index_to_serve = subscription_stream_requests.get_next_index_to_serve();

    // Verify the highest known version and epoch in the stream
    let expected_version =
        peer_known_version + (max_transaction_output_chunk_size * next_index_to_serve);
    assert_eq!(
        subscription_stream_requests.get_highest_known_version_and_epoch(),
        (expected_version, expected_epoch)
    );

    // Verify the number of active requests
    let num_active_stream_requests = subscription_stream_requests
        .get_pending_subscription_requests()
        .len();
    assert_eq!(
        num_active_stream_requests as u64,
        num_requests_per_batch - (next_index_to_serve % num_requests_per_batch)
    );
}

/// Waits for the specified number of optimistic fetches to be active
pub async fn wait_for_active_optimistic_fetches(
    active_optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    expected_num_active_fetches: usize,
) {
    // Wait for the specified number of active fetches
    let check_active_fetches = async move {
        loop {
            // Check if we've found the expected number of active fetches
            let num_active_fetches = active_optimistic_fetches.len();
            if num_active_fetches == expected_num_active_fetches {
                return; // We found the expected number of active fetches
            }

            // Otherwise, sleep for a while
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        check_active_fetches,
        &format!(
            "Timed-out while waiting for {} active fetches!",
            expected_num_active_fetches
        ),
    )
    .await;
}

/// Waits for the specified number of active stream requests for
/// the given peer ID.
pub async fn wait_for_active_stream_requests(
    active_subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    peer_network_id: PeerNetworkId,
    expected_num_active_stream_requests: usize,
) {
    // Wait for the specified number of active stream requests
    let check_active_stream_requests = async move {
        loop {
            // Check if the number of active stream requests matches
            if let Some(mut subscription) = active_subscriptions.get_mut(&peer_network_id) {
                let num_active_stream_requests = subscription
                    .value_mut()
                    .get_pending_subscription_requests()
                    .len();
                if num_active_stream_requests == expected_num_active_stream_requests {
                    return; // We found the expected number of stream requests
                }
            }

            // Otherwise, sleep for a while
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        check_active_stream_requests,
        &format!(
            "Timed-out while waiting for {} active stream requests.",
            expected_num_active_stream_requests
        ),
    )
    .await;
}

/// Waits for the specified number of subscriptions to be active
pub async fn wait_for_active_subscriptions(
    active_subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    expected_num_active_subscriptions: usize,
) {
    // Wait for the specified number of active subscriptions
    let check_active_subscriptions = async move {
        loop {
            // Check if the number of active subscriptions matches
            if active_subscriptions.len() == expected_num_active_subscriptions {
                return; // We found the expected number of active subscriptions
            }

            // Otherwise, sleep for a while
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        check_active_subscriptions,
        &format!(
            "Timed-out while waiting for {} active subscriptions.",
            expected_num_active_subscriptions
        ),
    )
    .await;
}

/// Waits for the cached storage summary to update
async fn wait_for_cached_summary_update(
    mock_client: &mut MockClient,
    mock_time: &MockTimeService,
    old_storage_server_summary: StorageServerSummary,
    continue_advancing_time: bool,
) {
    // Create a storage summary request
    let storage_request = StorageServiceRequest::new(DataRequest::GetStorageServerSummary, true);

    // Wait for the storage summary to update
    let storage_summary_updated = async move {
        while mock_client
            .process_request(storage_request.clone())
            .await
            .unwrap()
            == StorageServiceResponse::new(
                DataResponse::StorageServerSummary(old_storage_server_summary.clone()),
                true,
            )
            .unwrap()
        {
            // Advance the storage refresh time
            if continue_advancing_time {
                advance_storage_refresh_time(mock_time).await;
            }

            // Sleep for a while
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        storage_summary_updated,
        "Timed-out while waiting for the cached storage summary to update!",
    )
    .await;
}

/// Spawns the given task with a timeout
pub async fn spawn_with_timeout(task: impl Future<Output = ()>, timeout_error_message: &str) {
    let timeout_duration = Duration::from_secs(MAX_WAIT_TIME_SECS);
    timeout(timeout_duration, task)
        .await
        .expect(timeout_error_message)
}
