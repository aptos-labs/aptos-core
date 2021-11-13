// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{network::StorageServiceNetworkEvents, StorageReader, StorageServiceServer};
use anyhow::Result;
use channel::diem_channel;
use claim::{assert_matches, assert_none, assert_some};
use diem_config::config::StorageServiceConfig;
use diem_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use diem_logger::Level;
use diem_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    chain_id::ChainId,
    contract_event::ContractEvent,
    epoch_change::EpochChangeProof,
    event::EventKey,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::TransactionInfoListWithProof,
    protocol_spec::DpnProto,
    transaction::{
        default_protocol::{TransactionListWithProof, TransactionOutputListWithProof},
        RawTransaction, Script, SignedTransaction, Transaction, TransactionOutput,
        TransactionPayload, TransactionStatus, Version,
    },
    write_set::WriteSet,
    PeerId,
};
use futures::channel::oneshot;
use move_core_types::language_storage::TypeTag;
use network::{
    peer_manager::PeerManagerNotification,
    protocols::{
        network::NewNetworkEvents, rpc::InboundRpcRequest, wire::handshake::v1::ProtocolId,
    },
};
use std::{collections::BTreeMap, sync::Arc};
use storage_interface::DbReader;
use storage_service_types::{
    AccountStatesChunkWithProofRequest, CompleteDataRange, DataSummary,
    EpochEndingLedgerInfoRequest, ProtocolMetadata, ServerProtocolVersion, StorageServerSummary,
    StorageServiceError, StorageServiceMessage, StorageServiceRequest, StorageServiceResponse,
    TransactionOutputsWithProofRequest, TransactionsWithProofRequest,
};

// TODO(joshlind): Expand these test cases to better test storage interaction
// and functionality. This will likely require a better mock db abstraction.

#[tokio::test]
async fn test_get_server_protocol_version() {
    let (mut mock_client, service) = MockClient::new();
    tokio::spawn(service.start());

    // Process a request to fetch the protocol version
    let request = StorageServiceRequest::GetServerProtocolVersion;
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response = StorageServiceResponse::ServerProtocolVersion(ServerProtocolVersion {
        protocol_version: 1,
    });
    assert_eq!(response, expected_response);
}

#[tokio::test]
async fn test_get_account_states_chunk_with_proof() {
    let (mut mock_client, service) = MockClient::new();
    tokio::spawn(service.start());

    // Create a request to fetch an account states chunk with a proof
    let request =
        StorageServiceRequest::GetAccountStatesChunkWithProof(AccountStatesChunkWithProofRequest {
            version: 0,
            start_account_index: 0,
            end_account_index: 0,
        });

    // Process the request
    let error = mock_client.send_request(request).await.unwrap_err();

    // Verify the response is correct (the API call is currently unsupported)
    assert_matches!(error, StorageServiceError::InternalError(_));
}

#[tokio::test]
async fn test_get_number_of_accounts_at_version() {
    let (mut mock_client, service) = MockClient::new();
    tokio::spawn(service.start());

    // Create a request to fetch the number of accounts at the specified version
    let request = StorageServiceRequest::GetNumberOfAccountsAtVersion(10);

    // Process the request
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response = StorageServiceResponse::NumberOfAccountsAtVersion(1000);
    assert_eq!(response, expected_response);
}

#[tokio::test]
async fn test_get_storage_server_summary() {
    let (mut mock_client, service) = MockClient::new();
    tokio::spawn(service.start());

    // Process a request to fetch the storage summary
    let request = StorageServiceRequest::GetStorageServerSummary;
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    let highest_version = 100;
    let highest_epoch = 10;
    let expected_server_summary = StorageServerSummary {
        protocol_metadata: ProtocolMetadata {
            max_epoch_chunk_size: 100,
            max_transaction_chunk_size: 1000,
            max_transaction_output_chunk_size: 1000,
            max_account_states_chunk_size: 1000,
        },
        data_summary: DataSummary {
            synced_ledger_info: Some(create_test_ledger_info_with_sigs(
                highest_epoch,
                highest_version,
            )),
            epoch_ending_ledger_infos: Some(CompleteDataRange::from_genesis(highest_epoch - 1)),
            transactions: Some(CompleteDataRange::new(10, highest_version).unwrap()),
            transaction_outputs: Some(CompleteDataRange::new(20, highest_version).unwrap()),
            account_states: Some(CompleteDataRange::new(51, highest_version).unwrap()),
        },
    };
    assert_eq!(
        response,
        StorageServiceResponse::StorageServerSummary(expected_server_summary)
    );
}

#[tokio::test]
async fn test_get_transactions_with_proof_events() {
    let (mut mock_client, service) = MockClient::new();
    tokio::spawn(service.start());

    // Create a request to fetch transactions with a proof
    let start_version = 0;
    let end_version = 10;
    let request = StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
        proof_version: 100,
        start_version,
        end_version,
        include_events: true,
    });

    // Process the request
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    match response {
        StorageServiceResponse::TransactionsWithProof(transactions_with_proof) => {
            assert_eq!(
                transactions_with_proof.transactions.len() as u64,
                end_version - start_version + 1,
            );
            assert_eq!(
                transactions_with_proof.first_transaction_version,
                Some(start_version)
            );
            assert_some!(transactions_with_proof.events);
        }
        _ => panic!("Expected transactions with proof but got: {:?}", response),
    };
}

#[tokio::test]
async fn test_get_transactions_with_proof_no_events() {
    let (mut mock_client, service) = MockClient::new();
    tokio::spawn(service.start());

    // Create a request to fetch transactions with a proof (excluding events)
    let start_version = 10;
    let end_version = 30;
    let request = StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
        proof_version: 1000,
        start_version,
        end_version,
        include_events: false,
    });

    // Process the request
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    match response {
        StorageServiceResponse::TransactionsWithProof(transactions_with_proof) => {
            assert_eq!(
                transactions_with_proof.transactions.len() as u64,
                end_version - start_version + 1,
            );
            assert_eq!(
                transactions_with_proof.first_transaction_version,
                Some(start_version)
            );
            assert_none!(transactions_with_proof.events);
        }
        _ => panic!("Expected transactions with proof but got: {:?}", response),
    };
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof() {
    let (mut mock_client, service) = MockClient::new();
    tokio::spawn(service.start());

    // Create a request to fetch transaction outputs with a proof
    let start_version = 5;
    let end_version = 47;
    let request =
        StorageServiceRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            proof_version: 1000,
            start_version,
            end_version,
        });

    // Process the request
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    match response {
        StorageServiceResponse::TransactionOutputsWithProof(outputs_with_proof) => {
            assert_eq!(
                outputs_with_proof.transactions_and_outputs.len() as u64,
                end_version - start_version + 1,
            );
            assert_eq!(
                outputs_with_proof.first_transaction_output_version,
                Some(start_version)
            );
        }
        _ => panic!("Expected outputs with proof but got: {:?}", response),
    };
}

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos() {
    let (mut mock_client, service) = MockClient::new();
    tokio::spawn(service.start());

    // Create a request to fetch transactions with a proof (excluding events)
    let start_epoch = 11;
    let expected_end_epoch = 21;
    let request = StorageServiceRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
        start_epoch,
        expected_end_epoch,
    });

    // Process the request
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    match response {
        StorageServiceResponse::EpochEndingLedgerInfos(epoch_change_proof) => {
            assert_eq!(
                epoch_change_proof.ledger_info_with_sigs.len(),
                (expected_end_epoch - start_epoch + 1) as usize
            );
            assert_eq!(epoch_change_proof.more, false);

            for (i, epoch_ending_li) in epoch_change_proof.ledger_info_with_sigs.iter().enumerate()
            {
                assert_eq!(
                    epoch_ending_li.ledger_info().epoch(),
                    (i as u64) + start_epoch
                );
            }
        }
        _ => panic!("Expected epoch ending ledger infos but got: {:?}", response),
    };
}

/// A wrapper around the inbound network interface/channel for easily sending
/// mock client requests to a [`StorageServiceServer`].
struct MockClient {
    peer_mgr_notifs_tx: diem_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>,
}

impl MockClient {
    fn new() -> (Self, StorageServiceServer<StorageReader>) {
        initialize_logger();
        let storage = StorageReader::new(Arc::new(MockDbReader));

        let queue_cfg = crate::network::network_endpoint_config()
            .inbound_queue
            .unwrap();
        let (peer_mgr_notifs_tx, peer_mgr_notifs_rx) = queue_cfg.build();
        let (_connection_notifs_tx, connection_notifs_rx) = queue_cfg.build();
        let network_requests =
            StorageServiceNetworkEvents::new(peer_mgr_notifs_rx, connection_notifs_rx);

        let executor = tokio::runtime::Handle::current();
        let storage_server = StorageServiceServer::new(
            StorageServiceConfig::default(),
            executor,
            storage,
            network_requests,
        );

        (Self { peer_mgr_notifs_tx }, storage_server)
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

fn create_test_event(sequence_number: u64) -> ContractEvent {
    ContractEvent::new(
        EventKey::new_from_address(&AccountAddress::random(), 0),
        sequence_number,
        TypeTag::Bool,
        bcs::to_bytes(&0).unwrap(),
    )
}

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
        "".into(),
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

fn create_test_output() -> TransactionOutput {
    TransactionOutput::new(WriteSet::default(), vec![], 0, TransactionStatus::Retry)
}

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

/// This is a mock implementation of the `DbReader` trait.
#[derive(Clone)]
struct MockDbReader;

impl DbReader<DpnProto> for MockDbReader {
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<EpochChangeProof> {
        let mut ledger_info_with_sigs = vec![];
        for epoch in start_epoch..end_epoch + 1 {
            ledger_info_with_sigs.push(create_test_ledger_info_with_sigs(epoch, 0));
        }

        Ok(EpochChangeProof {
            ledger_info_with_sigs,
            more: false,
        })
    }

    fn get_transactions(
        &self,
        start_version: Version,
        batch_size: u64,
        _ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionListWithProof> {
        // Create mock events
        let events = if fetch_events {
            let mut events = vec![];
            for i in 0..batch_size {
                events.push(vec![create_test_event(i)]);
            }
            Some(events)
        } else {
            None
        };

        // Create mock transactions
        let mut transactions = vec![];
        for i in 0..batch_size {
            transactions.push(create_test_transaction(i))
        }

        Ok(TransactionListWithProof {
            transactions,
            events,
            first_transaction_version: Some(start_version),
            proof: TransactionInfoListWithProof::new_empty(),
        })
    }

    fn get_transaction_outputs(
        &self,
        start_version: Version,
        limit: u64,
        _ledger_version: Version,
    ) -> Result<TransactionOutputListWithProof> {
        // Create mock transactions and outputs
        let mut transactions_and_outputs = vec![];
        for i in 0..limit {
            let transaction = create_test_transaction(i);
            let output = create_test_output();
            transactions_and_outputs.push((transaction, output))
        }

        Ok(TransactionOutputListWithProof {
            transactions_and_outputs,
            first_transaction_output_version: Some(start_version),
            proof: TransactionInfoListWithProof::new_empty(),
        })
    }

    fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures> {
        Ok(create_test_ledger_info_with_sigs(10, 100))
    }

    fn get_first_write_set_version(&self) -> Result<Option<Version>> {
        Ok(Some(20))
    }

    fn get_first_txn_version(&self) -> Result<Option<Version>> {
        Ok(Some(10))
    }

    fn get_state_prune_window(&self) -> Option<usize> {
        Some(50)
    }

    fn get_account_count(&self, _version: Version) -> Result<usize> {
        Ok(1000)
    }
}

/// Initializes the Diem logger for tests
pub fn initialize_logger() {
    diem_logger::DiemLogger::builder()
        .is_async(false)
        .level(Level::Debug)
        .build();
}
