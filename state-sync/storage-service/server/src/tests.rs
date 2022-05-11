// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{network::StorageServiceNetworkEvents, StorageReader, StorageServiceServer};
use anyhow::{format_err, Result};
use aptos_config::config::StorageServiceConfig;
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use aptos_logger::Level;
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    chain_id::ChainId,
    contract_event::{ContractEvent, EventByVersionWithProof, EventWithProof},
    epoch_change::EpochChangeProof,
    event::EventKey,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::{
        AccumulatorConsistencyProof, SparseMerkleProof, SparseMerkleRangeProof,
        TransactionAccumulatorSummary, TransactionInfoListWithProof,
    },
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof, StateValueWithProof},
    },
    transaction::{
        AccountTransactionsWithProof, RawTransaction, Script, SignedTransaction, Transaction,
        TransactionInfo, TransactionListWithProof, TransactionOutputListWithProof,
        TransactionPayload, TransactionWithProof, Version,
    },
    PeerId,
};
use channel::aptos_channel;
use claim::assert_matches;
use futures::channel::oneshot;
use mockall::{
    mock,
    predicate::{always, eq},
    Sequence,
};
use network::{
    peer_manager::PeerManagerNotification,
    protocols::{
        network::NewNetworkEvents, rpc::InboundRpcRequest, wire::handshake::v1::ProtocolId,
    },
};
use std::{collections::BTreeMap, sync::Arc};
use storage_interface::{DbReader, Order, StartupInfo, TreeState};
use storage_service_types::{
    AccountStatesChunkWithProofRequest, CompleteDataRange, DataSummary, Epoch,
    EpochEndingLedgerInfoRequest, ProtocolMetadata, ServerProtocolVersion, StorageServerSummary,
    StorageServiceError, StorageServiceMessage, StorageServiceRequest, StorageServiceResponse,
    TransactionOutputsWithProofRequest, TransactionsWithProofRequest,
};

/// Various test constants for storage
const PROTOCOL_VERSION: u64 = 1;

#[tokio::test]
async fn test_cachable_requests_eviction() {
    // Create test data
    let max_lru_cache_size = StorageServiceConfig::default().max_lru_cache_size;
    let version = 101;
    let start_account_index = 100;
    let end_account_index = 199;
    let state_value_chunk_with_proof = StateValueChunkWithProof {
        first_index: start_account_index,
        last_index: end_account_index,
        first_key: HashValue::random(),
        last_key: HashValue::random(),
        raw_values: vec![],
        proof: SparseMerkleRangeProof::new(vec![]),
        root_hash: HashValue::random(),
    };

    // Create the mock db reader
    let mut db_reader = create_mock_db_reader();
    let mut expectation_sequence = Sequence::new();
    db_reader
        .expect_get_state_leaf_count()
        .times(max_lru_cache_size as usize)
        .with(always())
        .returning(move |_| Ok(165));
    for _ in 0..2 {
        let state_value_chunk_with_proof_clone = state_value_chunk_with_proof.clone();
        db_reader
            .expect_get_state_value_chunk_with_proof()
            .times(1)
            .with(
                eq(version),
                eq(start_account_index as usize),
                eq((end_account_index - start_account_index + 1) as usize),
            )
            .return_once(move |_, _, _| Ok(state_value_chunk_with_proof_clone))
            .in_sequence(&mut expectation_sequence);
    }

    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Process a request to fetch an account states chunk. This should cache and serve the response.
    for _ in 0..2 {
        let request = StorageServiceRequest::GetAccountStatesChunkWithProof(
            AccountStatesChunkWithProofRequest {
                version,
                start_account_index,
                end_account_index,
            },
        );
        let _ = mock_client.send_request(request).await.unwrap();
    }

    // Process enough requests to evict the previously cached response
    for version in 0..max_lru_cache_size {
        let request = StorageServiceRequest::GetNumberOfAccountsAtVersion(version);
        let _ = mock_client.send_request(request).await.unwrap();
    }

    // Process a request to fetch the account states chunk again. This requires refetching the data.
    let request =
        StorageServiceRequest::GetAccountStatesChunkWithProof(AccountStatesChunkWithProofRequest {
            version,
            start_account_index,
            end_account_index,
        });
    let _ = mock_client.send_request(request).await.unwrap();
}

#[tokio::test]
async fn test_cachable_requests_data_versions() {
    // Create test data
    let start_versions = [0, 76, 101, 230, 300, 454];
    let end_version = 454;
    let proof_version = end_version;
    let include_events = false;

    // Create the mock db reader
    let mut db_reader = create_mock_db_reader();
    let mut expectation_sequence = Sequence::new();
    let mut transaction_lists_with_proof = vec![];
    for start_version in start_versions {
        // Create and save test transaction lists
        let transaction_list_with_proof = create_transaction_list_with_proof(
            start_version,
            end_version,
            proof_version,
            include_events,
        );
        transaction_lists_with_proof.push(transaction_list_with_proof.clone());

        // Expect the data to be fetched from storage once
        db_reader
            .expect_get_transactions()
            .times(1)
            .with(
                eq(start_version),
                eq(end_version - start_version + 1),
                eq(proof_version),
                eq(include_events),
            )
            .return_once(move |_, _, _, _| Ok(transaction_list_with_proof))
            .in_sequence(&mut expectation_sequence);
    }

    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Repeatedly fetch the data and verify the responses
    for (i, start_version) in start_versions.iter().enumerate() {
        for _ in 0..10 {
            let request =
                StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    proof_version,
                    start_version: *start_version,
                    end_version,
                    include_events,
                });

            // Process the request
            let response = mock_client.send_request(request).await.unwrap();

            // Verify the response is correct
            match response {
                StorageServiceResponse::TransactionsWithProof(transactions_with_proof) => {
                    assert_eq!(transactions_with_proof, transaction_lists_with_proof[i])
                }
                _ => panic!("Expected transactions with proof but got: {:?}", response),
            };
        }
    }
}

#[tokio::test]
async fn test_get_server_protocol_version() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Process a request to fetch the protocol version
    let request = StorageServiceRequest::GetServerProtocolVersion;
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response = StorageServiceResponse::ServerProtocolVersion(ServerProtocolVersion {
        protocol_version: PROTOCOL_VERSION,
    });
    assert_eq!(response, expected_response);
}

#[tokio::test]
async fn test_get_account_states_with_proof() {
    // Test small and large chunk requests
    for chunk_size in [
        1,
        100,
        StorageServiceConfig::default().max_account_states_chunk_sizes,
    ] {
        // Create test data
        let version = 101;
        let start_account_index = 100;
        let end_account_index = start_account_index + chunk_size - 1;
        let state_value_chunk_with_proof = StateValueChunkWithProof {
            first_index: start_account_index,
            last_index: end_account_index,
            first_key: HashValue::random(),
            last_key: HashValue::random(),
            raw_values: vec![],
            proof: SparseMerkleRangeProof::new(vec![]),
            root_hash: HashValue::random(),
        };

        // Create the mock db reader
        let mut db_reader = create_mock_db_reader();
        let state_value_chunk_with_proof_clone = state_value_chunk_with_proof.clone();
        db_reader
            .expect_get_state_value_chunk_with_proof()
            .times(1)
            .with(
                eq(version),
                eq(start_account_index as usize),
                eq((end_account_index - start_account_index + 1) as usize),
            )
            .return_once(move |_, _, _| Ok(state_value_chunk_with_proof_clone));

        // Create the storage client and server
        let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
        tokio::spawn(service.start());

        // Process a request to fetch an account states chunk with a proof
        let request = StorageServiceRequest::GetAccountStatesChunkWithProof(
            AccountStatesChunkWithProofRequest {
                version,
                start_account_index,
                end_account_index,
            },
        );
        let response = mock_client.send_request(request).await.unwrap();

        // Verify the response is correct
        assert_eq!(
            response,
            StorageServiceResponse::AccountStatesChunkWithProof(state_value_chunk_with_proof)
        );
    }
}

#[tokio::test]
async fn test_get_account_states_with_proof_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Test invalid ranges and chunks that are too large
    let max_account_chunk_size = StorageServiceConfig::default().max_account_states_chunk_sizes;
    let start_account_index = 100;
    for end_account_index in [99, start_account_index + max_account_chunk_size] {
        let request = StorageServiceRequest::GetAccountStatesChunkWithProof(
            AccountStatesChunkWithProofRequest {
                version: 0,
                start_account_index,
                end_account_index,
            },
        );

        // Process and verify the response
        let response = mock_client.send_request(request).await.unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

#[tokio::test]
async fn test_get_number_of_accounts_at_version() {
    // Create test data
    let version = 101;
    let number_of_accounts: u64 = 560;

    // Create the mock db reader
    let mut db_reader = create_mock_db_reader();
    db_reader
        .expect_get_state_leaf_count()
        .times(1)
        .with(eq(version))
        .returning(move |_| Ok(number_of_accounts as usize));

    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Process a request to fetch the number of accounts at a version
    let request = StorageServiceRequest::GetNumberOfAccountsAtVersion(version);
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    assert_eq!(
        response,
        StorageServiceResponse::NumberOfAccountsAtVersion(number_of_accounts)
    );
}

#[tokio::test]
async fn test_get_number_of_accounts_at_version_invalid() {
    // Create test data
    let version = 1;

    // Create the mock db reader
    let mut db_reader = create_mock_db_reader();
    db_reader
        .expect_get_state_leaf_count()
        .times(1)
        .with(eq(version))
        .returning(move |_| Err(format_err!("Version does not exist!")));

    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Process a request to fetch the number of accounts at a version
    let request = StorageServiceRequest::GetNumberOfAccountsAtVersion(version);
    let response = mock_client.send_request(request).await.unwrap_err();

    // Verify the response is correct
    assert_matches!(response, StorageServiceError::InternalError(_));
}

#[tokio::test]
async fn test_get_storage_server_summary() {
    // Create test data
    let highest_version = 506;
    let highest_epoch = 30;
    let lowest_version = 101;
    let state_prune_window = 50;
    let highest_ledger_info = create_test_ledger_info_with_sigs(highest_epoch, highest_version);

    // Create the mock db reader
    let mut db_reader = create_mock_db_reader();
    let highest_ledger_info_clone = highest_ledger_info.clone();
    db_reader
        .expect_get_latest_ledger_info()
        .times(1)
        .return_once(move || Ok(highest_ledger_info_clone));
    db_reader
        .expect_get_first_txn_version()
        .times(1)
        .return_once(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_first_write_set_version()
        .times(1)
        .return_once(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_state_prune_window()
        .times(1)
        .return_once(move || Ok(Some(state_prune_window)));

    // Create the storage client and server
    let (mut mock_client, service, mock_time) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Fetch the storage summary and verify we get a default summary response
    let request = StorageServiceRequest::GetStorageServerSummary;
    let response = mock_client.send_request(request).await.unwrap();
    let default_response =
        StorageServiceResponse::StorageServerSummary(StorageServerSummary::default());
    assert_eq!(response, default_response);

    // Elapse enough time to force a cache update
    let default_storage_config = StorageServiceConfig::default();
    let cache_update_freq_ms = default_storage_config.storage_summary_refresh_interval_ms;
    mock_time.advance_ms_async(cache_update_freq_ms).await;

    // Process another request to fetch the storage summary
    let request = StorageServiceRequest::GetStorageServerSummary;
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct (after the cache update)
    let expected_server_summary = StorageServerSummary {
        protocol_metadata: ProtocolMetadata {
            max_epoch_chunk_size: default_storage_config.max_epoch_chunk_size,
            max_transaction_chunk_size: default_storage_config.max_transaction_chunk_size,
            max_transaction_output_chunk_size: default_storage_config
                .max_transaction_output_chunk_size,
            max_account_states_chunk_size: default_storage_config.max_account_states_chunk_sizes,
        },
        data_summary: DataSummary {
            synced_ledger_info: Some(highest_ledger_info),
            epoch_ending_ledger_infos: Some(CompleteDataRange::from_genesis(highest_epoch - 1)),
            transactions: Some(CompleteDataRange::new(lowest_version, highest_version).unwrap()),
            transaction_outputs: Some(
                CompleteDataRange::new(lowest_version, highest_version).unwrap(),
            ),
            account_states: Some(
                CompleteDataRange::new(
                    highest_version - state_prune_window as u64 + 1,
                    highest_version,
                )
                .unwrap(),
            ),
        },
    };
    assert_eq!(
        response,
        StorageServiceResponse::StorageServerSummary(expected_server_summary)
    );
}

#[tokio::test]
async fn test_get_transactions_with_proof() {
    // Test small and large chunk requests
    for chunk_size in [
        1,
        100,
        StorageServiceConfig::default().max_transaction_chunk_size,
    ] {
        // Test event inclusion
        for include_events in [true, false] {
            // Create test data
            let start_version = 0;
            let end_version = start_version + chunk_size - 1;
            let proof_version = end_version;
            let transaction_list_with_proof = create_transaction_list_with_proof(
                start_version,
                end_version,
                proof_version,
                include_events,
            );

            // Create the mock db reader
            let mut db_reader = create_mock_db_reader();
            let transaction_list_with_proof_clone = transaction_list_with_proof.clone();
            db_reader
                .expect_get_transactions()
                .times(1)
                .with(
                    eq(start_version),
                    eq(end_version - start_version + 1),
                    eq(proof_version),
                    eq(include_events),
                )
                .return_once(move |_, _, _, _| Ok(transaction_list_with_proof_clone));

            // Create the storage client and server
            let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
            tokio::spawn(service.start());

            // Create a request to fetch transactions with a proof
            let request =
                StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    proof_version,
                    start_version,
                    end_version,
                    include_events,
                });

            // Process the request
            let response = mock_client.send_request(request).await.unwrap();

            // Verify the response is correct
            match response {
                StorageServiceResponse::TransactionsWithProof(transactions_with_proof) => {
                    assert_eq!(transactions_with_proof, transaction_list_with_proof)
                }
                _ => panic!("Expected transactions with proof but got: {:?}", response),
            };
        }
    }
}

#[tokio::test]
async fn test_get_transactions_with_proof_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Test invalid ranges and chunks that are too large
    let max_transaction_chunk_size = StorageServiceConfig::default().max_transaction_chunk_size;
    let start_version = 1000;
    for end_version in [1, start_version + max_transaction_chunk_size] {
        let request =
            StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                proof_version: start_version,
                start_version,
                end_version,
                include_events: true,
            });

        // Process and verify the response
        let response = mock_client.send_request(request).await.unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof() {
    // Test small and large chunk requests
    for chunk_size in [
        1,
        100,
        StorageServiceConfig::default().max_transaction_output_chunk_size,
    ] {
        // Create test data
        let start_version = 0;
        let end_version = start_version + chunk_size - 1;
        let proof_version = end_version;
        let output_list_with_proof =
            create_output_list_with_proof(start_version, end_version, proof_version);

        // Create the mock db reader
        let mut db_reader = create_mock_db_reader();
        let output_list_with_proof_clone = output_list_with_proof.clone();
        db_reader
            .expect_get_transaction_outputs()
            .times(1)
            .with(
                eq(start_version),
                eq(end_version - start_version + 1),
                eq(proof_version),
            )
            .return_once(move |_, _, _| Ok(output_list_with_proof_clone));

        // Create the storage client and server
        let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
        tokio::spawn(service.start());

        // Create a request to fetch transactions outputs with a proof
        let request = StorageServiceRequest::GetTransactionOutputsWithProof(
            TransactionOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
            },
        );

        // Process the request
        let response = mock_client.send_request(request).await.unwrap();

        // Verify the response is correct
        match response {
            StorageServiceResponse::TransactionOutputsWithProof(outputs_with_proof) => {
                assert_eq!(outputs_with_proof, output_list_with_proof)
            }
            _ => panic!(
                "Expected transaction outputs with proof but got: {:?}",
                response
            ),
        };
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Test invalid ranges and chunks that are too large
    let max_output_chunk_size = StorageServiceConfig::default().max_transaction_output_chunk_size;
    let start_version = 1000;
    for end_version in [1, start_version + max_output_chunk_size] {
        let request = StorageServiceRequest::GetTransactionOutputsWithProof(
            TransactionOutputsWithProofRequest {
                proof_version: end_version,
                start_version,
                end_version,
            },
        );

        // Process and verify the response
        let response = mock_client.send_request(request).await.unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos() {
    // Test small and large chunk requests
    for chunk_size in [1, 100, StorageServiceConfig::default().max_epoch_chunk_size] {
        // Create test data
        let start_epoch = 11;
        let expected_end_epoch = start_epoch + chunk_size - 1;
        let epoch_change_proof = EpochChangeProof {
            ledger_info_with_sigs: create_epoch_ending_ledger_infos(
                start_epoch,
                expected_end_epoch,
            ),
            more: false,
        };

        // Create the mock db reader
        let mut db_reader = create_mock_db_reader();
        let epoch_change_proof_clone = epoch_change_proof.clone();
        db_reader
            .expect_get_epoch_ending_ledger_infos()
            .times(1)
            .with(eq(start_epoch), eq(expected_end_epoch + 1))
            .return_once(move |_, _| Ok(epoch_change_proof_clone));

        // Create the storage client and server
        let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
        tokio::spawn(service.start());

        // Create a request to fetch epoch ending ledger infos
        let request =
            StorageServiceRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
                start_epoch,
                expected_end_epoch,
            });

        // Process the request
        let response = mock_client.send_request(request).await.unwrap();

        // Verify the response is correct
        match response {
            StorageServiceResponse::EpochEndingLedgerInfos(response_epoch_change_proof) => {
                assert_eq!(response_epoch_change_proof, epoch_change_proof)
            }
            _ => panic!("Expected epoch ending ledger infos but got: {:?}", response),
        };
    }
}

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Test invalid ranges and chunks that are too large
    let max_epoch_chunk_size = StorageServiceConfig::default().max_epoch_chunk_size;
    let start_epoch = 11;
    for expected_end_epoch in [10, start_epoch + max_epoch_chunk_size] {
        let request =
            StorageServiceRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
                start_epoch,
                expected_end_epoch,
            });

        // Process and verify the response
        let response = mock_client.send_request(request).await.unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

/// A wrapper around the inbound network interface/channel for easily sending
/// mock client requests to a [`StorageServiceServer`].
struct MockClient {
    peer_mgr_notifs_tx: aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>,
}

impl MockClient {
    fn new(
        db_reader: Option<MockDatabaseReader>,
    ) -> (Self, StorageServiceServer<StorageReader>, MockTimeService) {
        initialize_logger();
        let storage_config = StorageServiceConfig::default();
        let storage = StorageReader::new(
            storage_config,
            Arc::new(db_reader.unwrap_or_else(create_mock_db_reader)),
        );

        let queue_cfg = crate::network::network_endpoint_config(storage_config)
            .inbound_queue
            .unwrap();
        let (peer_mgr_notifs_tx, peer_mgr_notifs_rx) = queue_cfg.build();
        let (_connection_notifs_tx, connection_notifs_rx) = queue_cfg.build();
        let network_requests =
            StorageServiceNetworkEvents::new(peer_mgr_notifs_rx, connection_notifs_rx);

        let executor = tokio::runtime::Handle::current();
        let mock_time_service = TimeService::mock();
        let storage_server = StorageServiceServer::new(
            StorageServiceConfig::default(),
            executor,
            storage,
            mock_time_service.clone(),
            network_requests,
        );

        let mock_client = Self { peer_mgr_notifs_tx };
        (mock_client, storage_server, mock_time_service.into_mock())
    }

    async fn send_request(
        &mut self,
        request: StorageServiceRequest,
    ) -> Result<StorageServiceResponse, StorageServiceError> {
        // craft the inbound Rpc notification
        let peer_id = PeerId::ZERO;
        let protocol_id = ProtocolId::StorageServiceRpc;
        let data = protocol_id
            .to_bytes(&StorageServiceMessage::Request(request))
            .unwrap();
        let (res_tx, res_rx) = oneshot::channel();
        let inbound_rpc = InboundRpcRequest {
            protocol_id,
            data: data.into(),
            res_tx,
        };
        let notif = PeerManagerNotification::RecvRpc(peer_id, inbound_rpc);

        // push it up to the storage service
        self.peer_mgr_notifs_tx
            .push((peer_id, protocol_id), notif)
            .unwrap();

        // wait for the response and deserialize
        let response = res_rx.await.unwrap().unwrap();
        let response = protocol_id
            .from_bytes::<StorageServiceMessage>(&response)
            .unwrap();
        match response {
            StorageServiceMessage::Response(response) => response,
            _ => panic!("Unexpected response message: {:?}", response),
        }
    }
}

/// Creates a test epoch change proof
fn create_epoch_ending_ledger_infos(
    start_epoch: Epoch,
    end_epoch: Epoch,
) -> Vec<LedgerInfoWithSignatures> {
    let mut ledger_info_with_sigs = vec![];
    for epoch in start_epoch..end_epoch {
        ledger_info_with_sigs.push(create_test_ledger_info_with_sigs(epoch, 0));
    }
    ledger_info_with_sigs
}

/// Creates a test transaction output list with proof
fn create_output_list_with_proof(
    start_version: u64,
    _end_version: u64,
    _proof_version: u64,
) -> TransactionOutputListWithProof {
    TransactionOutputListWithProof::new(
        vec![],
        Some(start_version),
        TransactionInfoListWithProof::new_empty(),
    )
}

/// Creates a test ledger info with signatures
fn create_test_ledger_info_with_sigs(epoch: u64, version: u64) -> LedgerInfoWithSignatures {
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
    LedgerInfoWithSignatures::new(ledger_info, BTreeMap::new())
}

/// Creates a test user transaction
fn create_test_transaction(sequence_number: u64) -> Transaction {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();

    let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
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
        private_key.sign(&raw_transaction),
    );

    Transaction::UserTransaction(signed_transaction)
}

/// Creates a test transaction output list with proof
fn create_transaction_list_with_proof(
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
        transactions.push(create_test_transaction(sequence_number));
    }

    // Create a transaction list with an empty proof
    let mut transaction_list_with_proof = TransactionListWithProof::new_empty();
    transaction_list_with_proof.first_transaction_version = Some(start_version);
    transaction_list_with_proof.events = events;
    transaction_list_with_proof.transactions = transactions;

    transaction_list_with_proof
}

/// Initializes the Aptos logger for tests
pub fn initialize_logger() {
    aptos_logger::Logger::builder()
        .is_async(false)
        .level(Level::Debug)
        .build();
}

/// Creates a mock database reader
pub fn create_mock_db_reader() -> MockDatabaseReader {
    MockDatabaseReader::new()
}

// This automatically creates a MockDatabaseReader.
// TODO(joshlind): if we frequently use these mocks, we should define a single
// mock test crate to be shared across the codebase.
mock! {
    pub DatabaseReader {}
    impl DbReader for DatabaseReader {
        fn get_epoch_ending_ledger_infos(
            &self,
            start_epoch: u64,
            end_epoch: u64,
        ) -> Result<EpochChangeProof>;

        fn get_transactions(
            &self,
            start_version: Version,
            batch_size: u64,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<TransactionListWithProof>;

        fn get_transaction_by_hash(
            &self,
            hash: HashValue,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<Option<TransactionWithProof>>;

        fn get_transaction_by_version(
            &self,
            version: Version,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<TransactionWithProof>;

        fn get_first_txn_version(&self) -> Result<Option<Version>>;

        fn get_first_write_set_version(&self) -> Result<Option<Version>>;

        fn get_transaction_outputs(
            &self,
            start_version: Version,
            limit: u64,
            ledger_version: Version,
        ) -> Result<TransactionOutputListWithProof>;

        fn get_events(
            &self,
            event_key: &EventKey,
            start: u64,
            order: Order,
            limit: u64,
        ) -> Result<Vec<(u64, ContractEvent)>>;

        fn get_events_with_proofs(
            &self,
            event_key: &EventKey,
            start: u64,
            order: Order,
            limit: u64,
            known_version: Option<u64>,
        ) -> Result<Vec<EventWithProof>>;

        fn get_block_timestamp(&self, version: u64) -> Result<u64>;

        fn get_event_by_version_with_proof(
            &self,
            event_key: &EventKey,
            event_version: u64,
            proof_version: u64,
        ) -> Result<EventByVersionWithProof>;

        fn get_last_version_before_timestamp(
            &self,
            _timestamp: u64,
            _ledger_version: Version,
        ) -> Result<Version>;

        fn get_latest_state_value(&self, state_key: StateKey) -> Result<Option<StateValue>>;

        fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>>;

        fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures>;

        fn get_latest_version_option(&self) -> Result<Option<Version>>;

        fn get_latest_version(&self) -> Result<Version>;

        fn get_latest_commit_metadata(&self) -> Result<(Version, u64)>;

        fn get_startup_info(&self) -> Result<Option<StartupInfo>>;

        fn get_account_transaction(
            &self,
            address: AccountAddress,
            seq_num: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<Option<TransactionWithProof>>;

        fn get_account_transactions(
            &self,
            address: AccountAddress,
            seq_num: u64,
            limit: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<AccountTransactionsWithProof>;

        fn get_state_proof_with_ledger_info(
            &self,
            known_version: u64,
            ledger_info: LedgerInfoWithSignatures,
        ) -> Result<StateProof>;

        fn get_state_proof(&self, known_version: u64) -> Result<StateProof>;

        fn get_state_value_with_proof(
            &self,
            state_key: StateKey,
            version: Version,
            ledger_version: Version,
        ) -> Result<StateValueWithProof>;

        fn get_state_value_with_proof_by_version(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<(Option<StateValue>, SparseMerkleProof<StateValue>)>;

        fn get_latest_tree_state(&self) -> Result<TreeState>;

        fn get_epoch_ending_ledger_info(&self, known_version: u64) -> Result<LedgerInfoWithSignatures>;

        fn get_latest_transaction_info_option(&self) -> Result<Option<(Version, TransactionInfo)>>;

        fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue>;

        fn get_accumulator_consistency_proof(
            &self,
            _client_known_version: Option<Version>,
            _ledger_version: Version,
        ) -> Result<AccumulatorConsistencyProof>;

        fn get_accumulator_summary(
            &self,
            ledger_version: Version,
        ) -> Result<TransactionAccumulatorSummary>;

        fn get_state_leaf_count(&self, version: Version) -> Result<usize>;

        fn get_state_value_chunk_with_proof(
            &self,
            version: Version,
            start_idx: usize,
            chunk_size: usize,
        ) -> Result<StateValueChunkWithProof>;

        fn get_state_prune_window(&self) -> Result<Option<usize>>;
    }
}
