// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    optimistic_fetch::OptimisticFetchRequest,
    storage::StorageReader,
    tests::mock::{MockClient, MockDatabaseReader},
    StorageServiceServer,
};
use aptos_config::{
    config::StorageServiceConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use aptos_infallible::Mutex;
use aptos_storage_service_types::{
    requests::{
        DataRequest, StateValuesWithProofRequest, StorageServiceRequest,
        TransactionsWithProofRequest,
    },
    responses::{CompleteDataRange, DataResponse, StorageServerSummary, StorageServiceResponse},
    Epoch, StorageServiceError,
};
use aptos_time_service::MockTimeService;
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    chain_id::ChainId,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::ValidatorSet,
    transaction::{
        ExecutionStatus, RawTransaction, Script, SignedTransaction, Transaction,
        TransactionListWithProof, TransactionOutput, TransactionOutputListWithProof,
        TransactionPayload, TransactionStatus,
    },
    validator_verifier::ValidatorVerifier,
    write_set::WriteSet,
};
use mockall::predicate::eq;
use rand::Rng;
use std::{collections::HashMap, sync::Arc, time::Duration};

/// Advances the given timer by the amount of time it takes to refresh storage
pub async fn advance_storage_refresh_time(mock_time: &MockTimeService) {
    let default_storage_config = StorageServiceConfig::default();
    let cache_update_freq_ms = default_storage_config.storage_summary_refresh_interval_ms;
    mock_time.advance_ms_async(cache_update_freq_ms).await;
}

/// Creates a test epoch ending ledger info
pub fn create_epoch_ending_ledger_info(epoch: u64, version: u64) -> LedgerInfoWithSignatures {
    // Create a new epoch state
    let verifier = ValidatorVerifier::from(&ValidatorSet::empty());
    let next_epoch_state = EpochState { epoch, verifier };

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
) -> TransactionOutputListWithProof {
    // Create a test transaction list that enforces the given size requirements
    let transaction_list_with_proof = create_transaction_list_using_sizes(
        start_version,
        num_outputs,
        min_bytes_per_output,
        false,
    );

    // Create a test transaction and output list
    let transactions_and_outputs = transaction_list_with_proof
        .transactions
        .iter()
        .map(|txn| (txn.clone(), create_test_transaction_output()))
        .collect();

    TransactionOutputListWithProof::new(
        transactions_and_outputs,
        Some(start_version),
        transaction_list_with_proof.proof,
    )
}

/// Creates a test transaction output list with proof
pub fn create_output_list_with_proof(
    start_version: u64,
    end_version: u64,
    proof_version: u64,
) -> TransactionOutputListWithProof {
    let transaction_list_with_proof =
        create_transaction_list_with_proof(start_version, end_version, proof_version, false);
    let transactions_and_outputs = transaction_list_with_proof
        .transactions
        .iter()
        .map(|txn| (txn.clone(), create_test_transaction_output()))
        .collect();

    TransactionOutputListWithProof::new(
        transactions_and_outputs,
        Some(start_version),
        transaction_list_with_proof.proof,
    )
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

/// Creates a test transaction output
fn create_test_transaction_output() -> TransactionOutput {
    TransactionOutput::new(
        WriteSet::default(),
        vec![],
        0,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(None)),
    )
}

/// Creates a test transaction list with proof with the given sizes
pub fn create_transaction_list_using_sizes(
    start_version: u64,
    num_transactions: u64,
    min_bytes_per_transaction: u64,
    include_events: bool,
) -> TransactionListWithProof {
    // Generate random bytes of the given size
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..min_bytes_per_transaction)
        .map(|_| rng.gen::<u8>())
        .collect();

    // Include events if required
    let events = if include_events { Some(vec![]) } else { None };

    // Create the requested transactions
    let mut transactions = vec![];
    for sequence_number in start_version..=start_version + num_transactions - 1 {
        transactions.push(create_test_transaction(
            sequence_number,
            random_bytes.clone(),
        ));
    }

    // Create a transaction list with an empty proof
    let mut transaction_list_with_proof = TransactionListWithProof::new_empty();
    transaction_list_with_proof.first_transaction_version = Some(start_version);
    transaction_list_with_proof.events = events;
    transaction_list_with_proof.transactions = transactions;

    transaction_list_with_proof
}

/// Creates a test transaction output list with proof
pub fn create_transaction_list_with_proof(
    start_version: u64,
    end_version: u64,
    _proof_version: u64,
    include_events: bool,
) -> TransactionListWithProof {
    // Include events if required
    let events = if include_events { Some(vec![]) } else { None };

    // Create the requested transactions
    let mut transactions = vec![];
    for sequence_number in start_version..=end_version {
        transactions.push(create_test_transaction(sequence_number, vec![]));
    }

    // Create a transaction list with an empty proof
    let mut transaction_list_with_proof = TransactionListWithProof::new_empty();
    transaction_list_with_proof.first_transaction_version = Some(start_version);
    transaction_list_with_proof.events = events;
    transaction_list_with_proof.transactions = transactions;

    transaction_list_with_proof
}

/// Creates a new storage service config with the limit
/// configured to be the size of an output list or transaction
/// list (depending on if `fallback_to_transactions` is set).
pub fn configure_network_chunk_limit(
    fallback_to_transactions: bool,
    output_list_with_proof: &TransactionOutputListWithProof,
    transaction_list_with_proof: &TransactionListWithProof,
) -> StorageServiceConfig {
    let max_network_chunk_bytes = if fallback_to_transactions {
        // Network limit is only big enough for the transaction list
        bcs::to_bytes(&transaction_list_with_proof).unwrap().len() as u64 + 1
    } else {
        // Network limit is big enough for the output list
        bcs::to_bytes(&output_list_with_proof).unwrap().len() as u64 + 1
    };
    StorageServiceConfig {
        max_network_chunk_bytes,
        ..Default::default()
    }
}

/// Sets an expectation on the given mock db for a call to fetch an epoch change proof
pub fn expect_get_epoch_ending_ledger_infos(
    mock_db: &mut MockDatabaseReader,
    start_epoch: u64,
    expected_end_epoch: u64,
    epoch_change_proof: EpochChangeProof,
) {
    mock_db
        .expect_get_epoch_ending_ledger_infos()
        .times(1)
        .with(eq(start_epoch), eq(expected_end_epoch))
        .returning(move |_, _| Ok(epoch_change_proof.clone()));
}

/// Sets an expectation on the given mock db for a call to fetch transactions
pub fn expect_get_transactions(
    mock_db: &mut MockDatabaseReader,
    start_version: u64,
    num_items: u64,
    proof_version: u64,
    include_events: bool,
    transaction_list: TransactionListWithProof,
) {
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
}

/// Sets an expectation on the given mock db for a call to fetch transaction outputs
pub fn expect_get_transaction_outputs(
    mock_db: &mut MockDatabaseReader,
    start_version: u64,
    num_items: u64,
    proof_version: u64,
    output_list: TransactionOutputListWithProof,
) {
    mock_db
        .expect_get_transaction_outputs()
        .times(1)
        .with(eq(start_version), eq(num_items), eq(proof_version))
        .returning(move |_, _, _| Ok(output_list.clone()));
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

/// Sends a number of states request and processes the response
pub async fn get_number_of_states(
    mock_client: &mut MockClient,
    version: u64,
    use_compression: bool,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = DataRequest::GetNumberOfStatesAtVersion(version);
    send_storage_request(mock_client, use_compression, data_request).await
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
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
        proof_version,
        start_version,
        end_version,
        include_events,
    });
    send_storage_request(mock_client, use_compression, data_request).await
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

/// Updates the storage server summary with the specified data
pub fn update_storage_server_summary(
    storage_server: &mut StorageServiceServer<StorageReader>,
    highest_synced_version: u64,
    highest_synced_epoch: Epoch,
) {
    // Create a storage server summary
    let mut storage_server_summary = StorageServerSummary::default();

    // Set the highest synced ledger info
    let mut data_summary = &mut storage_server_summary.data_summary;
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
    *storage_server.cached_storage_server_summary.write() = storage_server_summary;
}

/// Waits until the storage summary has refreshed for the first time
pub async fn wait_for_storage_to_refresh(
    mock_client: &mut MockClient,
    mock_time: &MockTimeService,
) {
    let storage_request = StorageServiceRequest::new(DataRequest::GetStorageServerSummary, true);
    while mock_client
        .process_request(storage_request.clone())
        .await
        .unwrap()
        == StorageServiceResponse::new(
            DataResponse::StorageServerSummary(StorageServerSummary::default()),
            true,
        )
        .unwrap()
    {
        advance_storage_refresh_time(mock_time).await;
    }
}

/// Advances enough time that the optimistic fetch service is able to refresh
pub async fn wait_for_optimistic_fetch_service_to_refresh(
    mock_client: &mut MockClient,
    mock_time: &MockTimeService,
) {
    // Elapse enough time to force storage to be updated
    wait_for_storage_to_refresh(mock_client, mock_time).await;

    // Elapse enough time to force the optimistic fetch thread to work
    advance_storage_refresh_time(mock_time).await;
}

/// Waits for the specified number of optimistic fetches to be active
pub async fn wait_for_active_optimistic_fetches(
    active_optimistic_fetches: Arc<Mutex<HashMap<PeerNetworkId, OptimisticFetchRequest>>>,
    expected_num_active_fetches: usize,
) {
    loop {
        let num_active_fetches = active_optimistic_fetches.lock().len();
        if num_active_fetches == expected_num_active_fetches {
            return; // We found the expected number of active fetches
        }

        // Sleep for a while
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
