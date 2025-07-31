// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::AptosDataClient,
    interface::{AptosDataClientInterface, SubscriptionRequestMetadata},
    priority::PeerPriority,
    tests::{mock::MockNetwork, utils},
};
use aptos_config::{config::AptosDataClientConfig, network_id::NetworkId};
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use aptos_storage_service_types::{
    requests::{DataRequest, TransactionData, TransactionDataRequestType, TransactionOrOutputData},
    responses::{
        DataResponse, NewTransactionDataWithProofResponse, StorageServiceResponse,
        TransactionDataResponseType, TransactionDataWithProofResponse,
    },
};
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    chain_id::ChainId,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::{
        ExecutionStatus, PersistedAuxiliaryInfo, RawTransaction, Script, SignedTransaction,
        Transaction, TransactionAuxiliaryData, TransactionInfo, TransactionListWithAuxiliaryInfos,
        TransactionListWithProof, TransactionListWithProofV2, TransactionOutput,
        TransactionOutputListWithAuxiliaryInfos, TransactionOutputListWithProof,
        TransactionOutputListWithProofV2, TransactionPayload, TransactionStatus,
    },
    write_set::WriteSet,
};
use claims::assert_matches;

#[tokio::test]
async fn test_get_transactions() {
    // Test both v1 and v2 requests
    for use_request_v2 in [false, true] {
        // Create the data client with a connected peer
        let (mut mock_network, client, network_id) = create_client_with_peer(use_request_v2);

        // Spawn a handler for the peer to respond to the request
        let num_transactions = 10;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Verify the network request
                    match network_request.storage_service_request.data_request {
                        DataRequest::GetTransactionsWithProof(_) => {
                            assert!(!use_request_v2)
                        },
                        DataRequest::GetTransactionDataWithProof(request) => {
                            assert!(use_request_v2);
                            assert_matches!(
                                request.transaction_data_request_type,
                                TransactionDataRequestType::TransactionData(TransactionData {
                                    include_events: true,
                                })
                            );
                        },
                        _ => panic!(
                            "Unexpected data request type: {:?}",
                            network_request.storage_service_request.data_request
                        ),
                    }

                    // Create the storage service response
                    let data_response = if use_request_v2 {
                        let transaction_list_with_proof =
                            Some(create_transaction_list_with_proof_v2(num_transactions));
                        DataResponse::TransactionDataWithProof(TransactionDataWithProofResponse {
                            transaction_data_response_type:
                                TransactionDataResponseType::TransactionData,
                            transaction_list_with_proof,
                            transaction_output_list_with_proof: None,
                        })
                    } else {
                        let transaction_list_with_proof =
                            create_transaction_list_with_proof(num_transactions);
                        DataResponse::TransactionsWithProof(transaction_list_with_proof)
                    };
                    let storage_service_response =
                        StorageServiceResponse::new(data_response, true).unwrap();

                    // Send the response
                    network_request
                        .response_sender
                        .send(Ok(storage_service_response));
                });
            }
        });

        // Send the request and wait for the response
        let response = client
            .get_transactions_with_proof(0, 0, 0, true, 0)
            .await
            .unwrap();

        // Verify the response
        let transaction_list_with_proof_v2 = response.payload;
        verify_response_data(
            Some(transaction_list_with_proof_v2),
            None,
            use_request_v2,
            num_transactions,
        );
    }
}

#[tokio::test]
async fn test_get_transaction_outputs() {
    // Test both v1 and v2 requests
    for use_request_v2 in [false, true] {
        // Create the data client with a connected peer
        let (mut mock_network, client, network_id) = create_client_with_peer(use_request_v2);

        // Spawn a handler for the peer to respond to the request
        let num_outputs = 15;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Verify the network request
                    match network_request.storage_service_request.data_request {
                        DataRequest::GetTransactionOutputsWithProof(_) => {
                            assert!(!use_request_v2)
                        },
                        DataRequest::GetTransactionDataWithProof(request) => {
                            assert!(use_request_v2);
                            assert_matches!(
                                request.transaction_data_request_type,
                                TransactionDataRequestType::TransactionOutputData
                            );
                        },
                        _ => panic!(
                            "Unexpected data request type: {:?}",
                            network_request.storage_service_request.data_request
                        ),
                    }

                    // Create the storage service response
                    let data_response = if use_request_v2 {
                        let transaction_output_list_with_proof =
                            Some(create_transaction_output_list_with_proof_v2(num_outputs));
                        DataResponse::TransactionDataWithProof(TransactionDataWithProofResponse {
                            transaction_data_response_type:
                                TransactionDataResponseType::TransactionOutputData,
                            transaction_list_with_proof: None,
                            transaction_output_list_with_proof,
                        })
                    } else {
                        let output_list_with_proof =
                            create_transaction_output_list_with_proof(num_outputs);
                        DataResponse::TransactionOutputsWithProof(output_list_with_proof)
                    };
                    let storage_service_response =
                        StorageServiceResponse::new(data_response, true).unwrap();

                    // Send the response
                    network_request
                        .response_sender
                        .send(Ok(storage_service_response));
                });
            }
        });

        // Send the request and wait for the response
        let response = client
            .get_transaction_outputs_with_proof(0, 0, 0, 0)
            .await
            .unwrap();

        // Verify the response
        let output_list_with_proof_v2 = response.payload;
        verify_response_data(
            None,
            Some(output_list_with_proof_v2),
            use_request_v2,
            num_outputs,
        );
    }
}

#[tokio::test]
async fn test_get_transactions_or_outputs() {
    // Test both v1 and v2 requests
    for use_request_v2 in [false, true] {
        // Create the data client with a connected peer
        let (mut mock_network, client, network_id) = create_client_with_peer(use_request_v2);

        // Spawn a handler for the peer to respond to the request
        let num_transactions_or_outputs = 20;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Verify the network request
                    match network_request.storage_service_request.data_request {
                        DataRequest::GetTransactionsOrOutputsWithProof(_) => {
                            assert!(!use_request_v2)
                        },
                        DataRequest::GetTransactionDataWithProof(request) => {
                            assert!(use_request_v2);
                            assert_matches!(
                                request.transaction_data_request_type,
                                TransactionDataRequestType::TransactionOrOutputData(
                                    TransactionOrOutputData {
                                        include_events: false,
                                    }
                                )
                            );
                        },
                        _ => panic!(
                            "Unexpected data request type: {:?}",
                            network_request.storage_service_request.data_request
                        ),
                    }

                    // Create the storage service response
                    let data_response = if use_request_v2 {
                        let transaction_list_with_proof = Some(
                            create_transaction_list_with_proof_v2(num_transactions_or_outputs),
                        );
                        DataResponse::TransactionDataWithProof(TransactionDataWithProofResponse {
                            transaction_data_response_type:
                                TransactionDataResponseType::TransactionData,
                            transaction_list_with_proof,
                            transaction_output_list_with_proof: None,
                        })
                    } else {
                        let transaction_list_with_proof = Some(create_transaction_list_with_proof(
                            num_transactions_or_outputs,
                        ));
                        DataResponse::TransactionsOrOutputsWithProof((
                            transaction_list_with_proof,
                            None,
                        ))
                    };
                    let storage_service_response =
                        StorageServiceResponse::new(data_response, true).unwrap();

                    // Send the response
                    network_request
                        .response_sender
                        .send(Ok(storage_service_response));
                });
            }
        });

        // Send the request and wait for the response
        let response = client
            .get_transactions_or_outputs_with_proof(0, 0, 0, false, 0)
            .await
            .unwrap();

        // Verify the response
        let (transaction_list_with_proof_v2, output_list_with_proof_v2) = response.payload;
        verify_response_data(
            transaction_list_with_proof_v2,
            output_list_with_proof_v2,
            use_request_v2,
            num_transactions_or_outputs,
        );
    }
}

#[tokio::test]
async fn test_get_new_transactions() {
    // Test both v1 and v2 requests
    for use_request_v2 in [false, true] {
        // Create the data client with a connected peer
        let (mut mock_network, client, network_id) = create_client_with_peer(use_request_v2);

        // Spawn a handler for the peer to respond to the request
        let num_transactions = 34;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Verify the network request
                    match network_request.storage_service_request.data_request {
                        DataRequest::GetNewTransactionsWithProof(_) => {
                            assert!(!use_request_v2)
                        },
                        DataRequest::GetNewTransactionDataWithProof(request) => {
                            assert!(use_request_v2);
                            assert_matches!(
                                request.transaction_data_request_type,
                                TransactionDataRequestType::TransactionData(TransactionData {
                                    include_events: true,
                                })
                            );
                        },
                        _ => panic!(
                            "Unexpected data request type: {:?}",
                            network_request.storage_service_request.data_request
                        ),
                    }

                    // Create the storage service response
                    let data_response = if use_request_v2 {
                        let transaction_list_with_proof =
                            Some(create_transaction_list_with_proof_v2(num_transactions));
                        DataResponse::NewTransactionDataWithProof(
                            NewTransactionDataWithProofResponse {
                                transaction_data_response_type:
                                    TransactionDataResponseType::TransactionData,
                                transaction_list_with_proof,
                                transaction_output_list_with_proof: None,
                                ledger_info_with_signatures: create_ledger_info(),
                            },
                        )
                    } else {
                        let transaction_list_with_proof =
                            create_transaction_list_with_proof(num_transactions);
                        DataResponse::NewTransactionsWithProof((
                            transaction_list_with_proof,
                            create_ledger_info(),
                        ))
                    };
                    let storage_service_response =
                        StorageServiceResponse::new(data_response, true).unwrap();

                    // Send the response
                    network_request
                        .response_sender
                        .send(Ok(storage_service_response));
                });
            }
        });

        // Send the request and wait for the response
        let response = client
            .get_new_transactions_with_proof(0, 0, true, 0)
            .await
            .unwrap();

        // Verify the response
        let (transaction_list_with_proof_v2, _) = response.payload;
        verify_response_data(
            Some(transaction_list_with_proof_v2),
            None,
            use_request_v2,
            num_transactions,
        );
    }
}

#[tokio::test]
async fn test_get_new_transaction_outputs() {
    // Test both v1 and v2 requests
    for use_request_v2 in [false, true] {
        // Create the data client with a connected peer
        let (mut mock_network, client, network_id) = create_client_with_peer(use_request_v2);

        // Spawn a handler for the peer to respond to the request
        let num_outputs = 42;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Verify the network request
                    match network_request.storage_service_request.data_request {
                        DataRequest::GetNewTransactionOutputsWithProof(_) => {
                            assert!(!use_request_v2)
                        },
                        DataRequest::GetNewTransactionDataWithProof(request) => {
                            assert!(use_request_v2);
                            assert_matches!(
                                request.transaction_data_request_type,
                                TransactionDataRequestType::TransactionOutputData
                            );
                        },
                        _ => panic!(
                            "Unexpected data request type: {:?}",
                            network_request.storage_service_request.data_request
                        ),
                    }

                    // Create the storage service response
                    let data_response = if use_request_v2 {
                        let transaction_output_list_with_proof =
                            Some(create_transaction_output_list_with_proof_v2(num_outputs));
                        DataResponse::NewTransactionDataWithProof(
                            NewTransactionDataWithProofResponse {
                                transaction_data_response_type:
                                    TransactionDataResponseType::TransactionOutputData,
                                transaction_list_with_proof: None,
                                transaction_output_list_with_proof,
                                ledger_info_with_signatures: create_ledger_info(),
                            },
                        )
                    } else {
                        let output_list_with_proof =
                            create_transaction_output_list_with_proof(num_outputs);
                        DataResponse::NewTransactionOutputsWithProof((
                            output_list_with_proof,
                            create_ledger_info(),
                        ))
                    };
                    let storage_service_response =
                        StorageServiceResponse::new(data_response, true).unwrap();

                    // Send the response
                    network_request
                        .response_sender
                        .send(Ok(storage_service_response));
                });
            }
        });

        // Send the request and wait for the response
        let response = client
            .get_new_transaction_outputs_with_proof(0, 0, 0)
            .await
            .unwrap();

        // Verify the response
        let (output_list_with_proof_v2, _) = response.payload;
        verify_response_data(
            None,
            Some(output_list_with_proof_v2),
            use_request_v2,
            num_outputs,
        );
    }
}

#[tokio::test]
async fn test_get_new_transactions_or_outputs() {
    // Test both v1 and v2 requests
    for use_request_v2 in [false, true] {
        // Create the data client with a connected peer
        let (mut mock_network, client, network_id) = create_client_with_peer(use_request_v2);

        // Spawn a handler for the peer to respond to the request
        let num_transactions_or_outputs = 50;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Verify the network request
                    match network_request.storage_service_request.data_request {
                        DataRequest::GetNewTransactionsOrOutputsWithProof(_) => {
                            assert!(!use_request_v2)
                        },
                        DataRequest::GetNewTransactionDataWithProof(request) => {
                            assert!(use_request_v2);
                            assert_matches!(
                                request.transaction_data_request_type,
                                TransactionDataRequestType::TransactionOrOutputData(
                                    TransactionOrOutputData {
                                        include_events: false,
                                    }
                                )
                            );
                        },
                        _ => panic!(
                            "Unexpected data request type: {:?}",
                            network_request.storage_service_request.data_request
                        ),
                    }

                    // Create the storage service response
                    let data_response = if use_request_v2 {
                        let transaction_list_with_proof = Some(
                            create_transaction_list_with_proof_v2(num_transactions_or_outputs),
                        );
                        DataResponse::NewTransactionDataWithProof(
                            NewTransactionDataWithProofResponse {
                                transaction_data_response_type:
                                    TransactionDataResponseType::TransactionData,
                                transaction_list_with_proof,
                                transaction_output_list_with_proof: None,
                                ledger_info_with_signatures: create_ledger_info(),
                            },
                        )
                    } else {
                        let transaction_list_with_proof =
                            create_transaction_list_with_proof(num_transactions_or_outputs);
                        let transaction_or_output_list_with_proof =
                            (Some(transaction_list_with_proof), None);
                        DataResponse::NewTransactionsOrOutputsWithProof((
                            transaction_or_output_list_with_proof,
                            create_ledger_info(),
                        ))
                    };
                    let storage_service_response =
                        StorageServiceResponse::new(data_response, true).unwrap();

                    // Send the response
                    network_request
                        .response_sender
                        .send(Ok(storage_service_response));
                });
            }
        });

        // Send the request and wait for the response
        let response = client
            .get_new_transactions_or_outputs_with_proof(0, 0, false, 0)
            .await
            .unwrap();

        // Verify the response
        let ((transaction_list_with_proof, output_list_with_proof), _) = response.payload;
        verify_response_data(
            transaction_list_with_proof,
            output_list_with_proof,
            use_request_v2,
            num_transactions_or_outputs,
        );
    }
}

#[tokio::test]
async fn test_subscribe_transactions() {
    // Test both v1 and v2 requests
    for use_request_v2 in [false, true] {
        // Create the data client with a connected peer
        let (mut mock_network, client, network_id) = create_client_with_peer(use_request_v2);

        // Spawn a handler for the peer to respond to the request
        let num_transactions = 5;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Verify the network request
                    match network_request.storage_service_request.data_request {
                        DataRequest::SubscribeTransactionsWithProof(_) => {
                            assert!(!use_request_v2)
                        },
                        DataRequest::SubscribeTransactionDataWithProof(request) => {
                            assert!(use_request_v2);
                            assert_matches!(
                                request.transaction_data_request_type,
                                TransactionDataRequestType::TransactionData(TransactionData {
                                    include_events: true,
                                })
                            );
                        },
                        _ => panic!(
                            "Unexpected data request type: {:?}",
                            network_request.storage_service_request.data_request
                        ),
                    }

                    // Create the storage service response
                    let data_response = if use_request_v2 {
                        let transaction_list_with_proof =
                            Some(create_transaction_list_with_proof_v2(num_transactions));
                        DataResponse::NewTransactionDataWithProof(
                            NewTransactionDataWithProofResponse {
                                transaction_data_response_type:
                                    TransactionDataResponseType::TransactionData,
                                transaction_list_with_proof,
                                transaction_output_list_with_proof: None,
                                ledger_info_with_signatures: create_ledger_info(),
                            },
                        )
                    } else {
                        let transaction_list_with_proof =
                            create_transaction_list_with_proof(num_transactions);
                        DataResponse::NewTransactionsWithProof((
                            transaction_list_with_proof,
                            create_ledger_info(),
                        ))
                    };
                    let storage_service_response =
                        StorageServiceResponse::new(data_response, true).unwrap();

                    // Send the response
                    network_request
                        .response_sender
                        .send(Ok(storage_service_response));
                });
            }
        });

        // Send the request and wait for the response
        let subscription_request_metadata = create_subscription_request_metadata();
        let response = client
            .subscribe_to_transactions_with_proof(subscription_request_metadata, true, 0)
            .await
            .unwrap();

        // Verify the response
        let (transaction_list_with_proof_v2, _) = response.payload;
        verify_response_data(
            Some(transaction_list_with_proof_v2),
            None,
            use_request_v2,
            num_transactions,
        );
    }
}

#[tokio::test]
async fn test_subscribe_transaction_outputs() {
    // Test both v1 and v2 requests
    for use_request_v2 in [false, true] {
        // Create the data client with a connected peer
        let (mut mock_network, client, network_id) = create_client_with_peer(use_request_v2);

        // Spawn a handler for the peer to respond to the request
        let num_outputs = 7;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Verify the network request
                    match network_request.storage_service_request.data_request {
                        DataRequest::SubscribeTransactionOutputsWithProof(_) => {
                            assert!(!use_request_v2)
                        },
                        DataRequest::SubscribeTransactionDataWithProof(request) => {
                            assert!(use_request_v2);
                            assert_matches!(
                                request.transaction_data_request_type,
                                TransactionDataRequestType::TransactionOutputData
                            );
                        },
                        _ => panic!(
                            "Unexpected data request type: {:?}",
                            network_request.storage_service_request.data_request
                        ),
                    }

                    // Create the storage service response
                    let data_response = if use_request_v2 {
                        let transaction_output_list_with_proof =
                            Some(create_transaction_output_list_with_proof_v2(num_outputs));
                        DataResponse::NewTransactionDataWithProof(
                            NewTransactionDataWithProofResponse {
                                transaction_data_response_type:
                                    TransactionDataResponseType::TransactionOutputData,
                                transaction_list_with_proof: None,
                                transaction_output_list_with_proof,
                                ledger_info_with_signatures: create_ledger_info(),
                            },
                        )
                    } else {
                        let output_list_with_proof =
                            create_transaction_output_list_with_proof(num_outputs);
                        DataResponse::NewTransactionOutputsWithProof((
                            output_list_with_proof,
                            create_ledger_info(),
                        ))
                    };
                    let storage_service_response =
                        StorageServiceResponse::new(data_response, true).unwrap();

                    // Send the response
                    network_request
                        .response_sender
                        .send(Ok(storage_service_response));
                });
            }
        });

        // Send the request and wait for the response
        let subscription_request_metadata = create_subscription_request_metadata();
        let response = client
            .subscribe_to_transaction_outputs_with_proof(subscription_request_metadata, 0)
            .await
            .unwrap();

        // Verify the response
        let (output_list_with_proof_v2, _) = response.payload;
        verify_response_data(
            None,
            Some(output_list_with_proof_v2),
            use_request_v2,
            num_outputs,
        );
    }
}

#[tokio::test]
async fn test_subscribe_transactions_or_outputs() {
    // Test both v1 and v2 requests
    for use_request_v2 in [false, true] {
        // Create the data client with a connected peer
        let (mut mock_network, client, network_id) = create_client_with_peer(use_request_v2);

        // Spawn a handler for the peer to respond to the request
        let num_transactions_or_outputs = 12;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Verify the network request
                    match network_request.storage_service_request.data_request {
                        DataRequest::SubscribeTransactionsOrOutputsWithProof(_) => {
                            assert!(!use_request_v2)
                        },
                        DataRequest::SubscribeTransactionDataWithProof(request) => {
                            assert!(use_request_v2);
                            assert_matches!(
                                request.transaction_data_request_type,
                                TransactionDataRequestType::TransactionOrOutputData(
                                    TransactionOrOutputData {
                                        include_events: true,
                                    }
                                )
                            );
                        },
                        _ => panic!(
                            "Unexpected data request type: {:?}",
                            network_request.storage_service_request.data_request
                        ),
                    }

                    // Create the storage service response
                    let data_response = if use_request_v2 {
                        let transaction_output_list_with_proof =
                            Some(create_transaction_output_list_with_proof_v2(
                                num_transactions_or_outputs,
                            ));
                        DataResponse::NewTransactionDataWithProof(
                            NewTransactionDataWithProofResponse {
                                transaction_data_response_type:
                                    TransactionDataResponseType::TransactionData,
                                transaction_list_with_proof: None,
                                transaction_output_list_with_proof,
                                ledger_info_with_signatures: create_ledger_info(),
                            },
                        )
                    } else {
                        let output_list_with_proof =
                            create_transaction_output_list_with_proof(num_transactions_or_outputs);
                        let transaction_or_output_list_with_proof =
                            (None, Some(output_list_with_proof));
                        DataResponse::NewTransactionsOrOutputsWithProof((
                            transaction_or_output_list_with_proof,
                            create_ledger_info(),
                        ))
                    };
                    let storage_service_response =
                        StorageServiceResponse::new(data_response, true).unwrap();

                    // Send the response
                    network_request
                        .response_sender
                        .send(Ok(storage_service_response));
                });
            }
        });

        // Send the request and wait for the response
        let subscription_request_metadata = create_subscription_request_metadata();
        let response = client
            .subscribe_to_transactions_or_outputs_with_proof(subscription_request_metadata, true, 0)
            .await
            .unwrap();

        // Verify the response
        let ((transaction_list_with_proof, output_list_with_proof), _) = response.payload;
        verify_response_data(
            transaction_list_with_proof,
            output_list_with_proof,
            use_request_v2,
            num_transactions_or_outputs,
        );
    }
}

/// Creates a data client config with the specified request v2 flag
fn create_aptos_data_client(use_request_v2: bool) -> AptosDataClientConfig {
    AptosDataClientConfig {
        enable_transaction_data_v2: use_request_v2,
        ..Default::default()
    }
}

/// Creates a data client with a connected peer for testing
fn create_client_with_peer(use_request_v2: bool) -> (MockNetwork, AptosDataClient, NetworkId) {
    // Create a base config for a fullnode
    let base_config = utils::create_fullnode_base_config();

    // Create a data client config with request v2 set appropriately
    let data_client_config = create_aptos_data_client(use_request_v2);

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Add a peer to the network
    let (peer_network_id, network_id) =
        utils::add_peer_to_network(PeerPriority::HighPriority, &mut mock_network);

    // Advertise transaction data for the peer
    let storage_summary = utils::create_storage_summary(1000);
    client.update_peer_storage_summary(peer_network_id, storage_summary.clone());
    client.update_global_summary_cache().unwrap();
    (mock_network, client, network_id)
}

/// Creates a new ledger info
fn create_ledger_info() -> LedgerInfoWithSignatures {
    LedgerInfoWithSignatures::new(
        LedgerInfo::new(BlockInfo::random_with_epoch(10, 10), HashValue::random()),
        AggregateSignature::empty(),
    )
}

/// Creates a subscription request metadata with default values
fn create_subscription_request_metadata() -> SubscriptionRequestMetadata {
    SubscriptionRequestMetadata {
        known_version_at_stream_start: 0,
        known_epoch_at_stream_start: 0,
        subscription_stream_index: 0,
        subscription_stream_id: 0,
    }
}

/// Creates a test transaction
fn create_transaction() -> Transaction {
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
    let signature = private_key.sign(&raw_transaction).unwrap();
    let signed_transaction = SignedTransaction::new(raw_transaction, public_key, signature);

    Transaction::UserTransaction(signed_transaction)
}

/// Creates a test transaction info
fn create_transaction_info() -> TransactionInfo {
    TransactionInfo::new(
        HashValue::random(),
        HashValue::random(),
        HashValue::random(),
        Some(HashValue::random()),
        0,
        ExecutionStatus::Success,
        Some(HashValue::random()),
    )
}

/// Creates a transaction list with proof (with the specified number of transactions)
fn create_transaction_list_with_proof(num_transactions: usize) -> TransactionListWithProof {
    // Create the requested transactions
    let mut transactions = vec![];
    for _ in 0..num_transactions {
        transactions.push(create_transaction());
    }

    // Create the transaction infos
    let mut transaction_infos = vec![];
    for _ in 0..num_transactions {
        transaction_infos.push(create_transaction_info());
    }

    // Create the transaction list with proof
    let mut transaction_list_with_proof = TransactionListWithProof::new_empty();
    transaction_list_with_proof.transactions = transactions;
    transaction_list_with_proof.proof.transaction_infos = transaction_infos;

    transaction_list_with_proof
}

/// Creates a transaction list with proof v2 (with the specified number of transactions)
fn create_transaction_list_with_proof_v2(num_transactions: usize) -> TransactionListWithProofV2 {
    // Create the transaction list with proof
    let transaction_list_with_proof = create_transaction_list_with_proof(num_transactions);

    // Create the auxiliary infos
    let mut persisted_auxiliary_infos = vec![];
    for index in 0..num_transactions {
        persisted_auxiliary_infos.push(PersistedAuxiliaryInfo::V1 {
            transaction_index: index as u32,
        });
    }

    // Create the transaction list with proof v2
    let transaction_list_with_auxiliary_infos = TransactionListWithAuxiliaryInfos::new(
        transaction_list_with_proof,
        persisted_auxiliary_infos,
    );
    TransactionListWithProofV2::new(transaction_list_with_auxiliary_infos)
}

/// Creates a single test transaction output
fn create_transaction_output() -> TransactionOutput {
    TransactionOutput::new(
        WriteSet::default(),
        vec![],
        0,
        TransactionStatus::Keep(ExecutionStatus::Success),
        TransactionAuxiliaryData::default(),
    )
}

/// Creates a transaction output list with proof (with the specified number of outputs)
fn create_transaction_output_list_with_proof(num_outputs: usize) -> TransactionOutputListWithProof {
    // Create the transactions and outputs
    let transaction_list_with_proof = create_transaction_list_with_proof(num_outputs);
    let transactions_and_outputs = transaction_list_with_proof
        .transactions
        .iter()
        .map(|txn| (txn.clone(), create_transaction_output()))
        .collect();

    // Create the transaction infos
    let mut transaction_infos = vec![];
    for _ in 0..num_outputs {
        transaction_infos.push(create_transaction_info());
    }

    // Create the transaction output list with proof
    let mut output_list_with_proof = TransactionOutputListWithProof::new_empty();
    output_list_with_proof.transactions_and_outputs = transactions_and_outputs;
    output_list_with_proof.proof.transaction_infos = transaction_infos;

    output_list_with_proof
}

/// Creates a transaction output list with proof v2 (with the specified number of outputs)
fn create_transaction_output_list_with_proof_v2(
    num_outputs: usize,
) -> TransactionOutputListWithProofV2 {
    // Create the transaction output list with proof
    let transaction_output_list_with_proof = create_transaction_output_list_with_proof(num_outputs);

    // Create the auxiliary infos
    let mut persisted_auxiliary_infos = vec![];
    for index in 0..num_outputs {
        persisted_auxiliary_infos.push(PersistedAuxiliaryInfo::V1 {
            transaction_index: index as u32,
        });
    }

    // Create the transaction output list with proof v2
    let output_list_with_auxiliary_infos = TransactionOutputListWithAuxiliaryInfos::new(
        transaction_output_list_with_proof,
        persisted_auxiliary_infos,
    );
    TransactionOutputListWithProofV2::new(output_list_with_auxiliary_infos)
}

/// Verifies that the persisted auxiliary infos are populated correctly
fn verify_persisted_auxiliary_infos(
    use_request_v2: bool,
    persisted_auxiliary_infos: &[PersistedAuxiliaryInfo],
    expected_length: usize,
) {
    // Verify the length of the auxiliary infos
    assert_eq!(persisted_auxiliary_infos.len(), expected_length);

    // Verify the contents of the auxiliary infos
    for auxiliary_info in persisted_auxiliary_infos {
        if use_request_v2 {
            assert_matches!(auxiliary_info, PersistedAuxiliaryInfo::V1 { .. });
        } else {
            assert_eq!(auxiliary_info, &PersistedAuxiliaryInfo::None);
        }
    }
}

/// Verifies the response data for any given transaction or output list.
/// Also verifies the persisted auxiliary infos (if applicable).
fn verify_response_data(
    transaction_list_with_proof_v2: Option<TransactionListWithProofV2>,
    output_list_with_proof_v2: Option<TransactionOutputListWithProofV2>,
    use_request_v2: bool,
    expected_count: usize,
) {
    // Verify the transaction data
    if let Some(transaction_list_with_proof_v2) = transaction_list_with_proof_v2 {
        // Verify the number of transactions
        let transaction_list_with_proof =
            transaction_list_with_proof_v2.get_transaction_list_with_proof();
        let num_transactions = transaction_list_with_proof.get_num_transactions();
        assert_eq!(num_transactions, expected_count);

        // Verify the persisted auxiliary infos
        verify_persisted_auxiliary_infos(
            use_request_v2,
            transaction_list_with_proof_v2.get_persisted_auxiliary_infos(),
            expected_count,
        );
    }

    // Verify the output data
    if let Some(output_list_with_proof_v2) = output_list_with_proof_v2 {
        // Verify the number of outputs
        let output_list_with_proof = output_list_with_proof_v2.get_output_list_with_proof();
        let num_outputs = output_list_with_proof.get_num_outputs();
        assert_eq!(num_outputs, expected_count);

        // Verify the persisted auxiliary infos
        verify_persisted_auxiliary_infos(
            use_request_v2,
            output_list_with_proof_v2.get_persisted_auxiliary_infos(),
            expected_count,
        );
    }
}
