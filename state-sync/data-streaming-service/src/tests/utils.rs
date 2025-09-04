// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{data_notification::DataNotification, data_stream::DataStreamListener, error::Error};
use velor_config::config::VelorDataClientConfig;
use velor_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use velor_data_client::{
    global_summary::{AdvertisedData, GlobalDataSummary, OptimalChunkSizes},
    interface::{
        VelorDataClientInterface, Response, ResponseCallback, ResponseContext, ResponseError,
        SubscriptionRequestMetadata,
    },
};
use velor_infallible::Mutex;
use velor_logger::Level;
use velor_storage_service_types::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, NewTransactionOutputsWithProofRequest,
        NewTransactionsOrOutputsWithProofRequest, NewTransactionsWithProofRequest,
        StateValuesWithProofRequest, SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        SubscriptionStreamMetadata, TransactionOutputsWithProofRequest,
        TransactionsOrOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{CompleteDataRange, TransactionOrOutputListWithProofV2},
    Epoch,
};
use velor_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    chain_id::ChainId,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::SparseMerkleRangeProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        RawTransaction, Script, SignedTransaction, Transaction, TransactionAuxiliaryData,
        TransactionListWithProof, TransactionListWithProofV2, TransactionOutput,
        TransactionOutputListWithProof, TransactionOutputListWithProofV2, TransactionPayload,
        TransactionStatus, Version,
    },
    write_set::WriteSet,
};
use async_trait::async_trait;
use futures::StreamExt;
use rand::{rngs::OsRng, Rng};
use std::{
    cmp::min,
    collections::{BTreeMap, HashMap},
    ops::DerefMut,
    sync::Arc,
    time::Duration,
};
use tokio::time::timeout;
// TODO(joshlind): provide a better way to mock the data client.
// Especially around verifying timeouts!

/// Generic test constants
pub const MAX_NOTIFICATION_TIMEOUT_SECS: u64 = 10;
pub const MAX_RESPONSE_ID: u64 = 100000;
pub const TOTAL_NUM_STATE_VALUES: u64 = 2000;

/// Test constants for advertised data
pub const MIN_ADVERTISED_EPOCH_END: u64 = 100;
pub const MAX_ADVERTISED_EPOCH_END: u64 = 150;
pub const MIN_ADVERTISED_STATES: u64 = 9500;
pub const MAX_ADVERTISED_STATES: u64 = 10000;
pub const MIN_ADVERTISED_TRANSACTION: u64 = 1000;
pub const MAX_ADVERTISED_TRANSACTION: u64 = 10000;
pub const MIN_ADVERTISED_TRANSACTION_OUTPUT: u64 = 1000;
pub const MAX_ADVERTISED_TRANSACTION_OUTPUT: u64 = 10000;

/// Test constants for data beyond the highest advertised
pub const MAX_REAL_EPOCH_END: u64 = MAX_ADVERTISED_EPOCH_END + 2;
pub const MAX_REAL_TRANSACTION: u64 = MAX_ADVERTISED_TRANSACTION + 10;
pub const MAX_REAL_TRANSACTION_OUTPUT: u64 = MAX_REAL_TRANSACTION;

/// A simple mock of the Velor Data Client
#[derive(Clone, Debug)]
pub struct MockVelorDataClient {
    pub velor_data_client_config: VelorDataClientConfig,
    pub advertised_epoch_ending_ledger_infos: BTreeMap<Epoch, LedgerInfoWithSignatures>,
    pub advertised_synced_ledger_infos: Vec<LedgerInfoWithSignatures>,
    pub data_beyond_highest_advertised: bool, // If true, data exists beyond the highest advertised
    pub data_request_counter: Arc<Mutex<HashMap<DataRequest, u64>>>, // Tracks the number of times the same data request was made
    pub highest_epoch_ending_ledger_infos: BTreeMap<Epoch, LedgerInfoWithSignatures>,
    pub limit_chunk_sizes: bool, // If true, responses will be truncated to emulate chunk and network limits
    pub skip_emulate_network_latencies: bool, // If true, skips network latency emulation
    pub skip_timeout_verification: bool, // If true, skips timeout verification for incoming requests
}

impl MockVelorDataClient {
    pub fn new(
        velor_data_client_config: VelorDataClientConfig,
        data_beyond_highest_advertised: bool,
        limit_chunk_sizes: bool,
        skip_emulate_network_latencies: bool,
        skip_timeout_verification: bool,
    ) -> Self {
        // Create the advertised data
        let advertised_epoch_ending_ledger_infos = create_epoch_ending_ledger_infos(
            MIN_ADVERTISED_EPOCH_END,
            MIN_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_EPOCH_END,
            MAX_ADVERTISED_TRANSACTION,
        );
        let advertised_synced_ledger_infos = create_synced_ledger_infos(
            MIN_ADVERTISED_EPOCH_END,
            MIN_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_EPOCH_END,
            MAX_ADVERTISED_TRANSACTION,
            &advertised_epoch_ending_ledger_infos,
        );

        // Create the highest data
        let highest_epoch_ending_ledger_infos = create_epoch_ending_ledger_infos(
            MAX_ADVERTISED_EPOCH_END + 1,
            MAX_ADVERTISED_TRANSACTION + 1,
            MAX_REAL_EPOCH_END,
            MAX_REAL_TRANSACTION,
        );

        // Create the data request counter
        let data_request_counter = Arc::new(Mutex::new(HashMap::new()));

        Self {
            velor_data_client_config,
            advertised_epoch_ending_ledger_infos,
            advertised_synced_ledger_infos,
            data_beyond_highest_advertised,
            data_request_counter,
            highest_epoch_ending_ledger_infos,
            limit_chunk_sizes,
            skip_emulate_network_latencies,
            skip_timeout_verification,
        }
    }

    /// Clones the mock data client without timeout verification
    fn clone_without_timeout_verification(&self) -> Self {
        // Clone the mock data client and skip timeout verification
        let mut velor_data_client = self.clone();
        velor_data_client.skip_timeout_verification = true;

        velor_data_client
    }

    /// Emulates network latencies by sleeping for 10 -> 50 ms
    async fn emulate_network_latencies(&self) {
        if !self.skip_emulate_network_latencies {
            tokio::time::sleep(Duration::from_millis(create_range_random_u64(10, 50))).await;
        }
    }

    /// Emulates a timeout by sleeping for a long time and returning a timeout error
    async fn emulate_network_request_timeout(&self) -> velor_data_client::error::Error {
        // Sleep for a while
        tokio::time::sleep(Duration::from_secs(MAX_NOTIFICATION_TIMEOUT_SECS)).await;

        // Return a timeout error
        velor_data_client::error::Error::TimeoutWaitingForResponse("RPC timed out!".into())
    }

    /// Calculates the last index for the given start and end indices (with
    /// respect to a chunk size limit).
    fn calculate_last_index(&self, start_index: u64, end_index: u64) -> u64 {
        if self.limit_chunk_sizes {
            // Limit the chunk size by a random factor
            let chunk_reduction_factor = create_range_random_u64(2, 9);

            // Calculate the number of items to request
            let num_items_requested = (end_index - start_index) + 1;
            let num_reduced_items_requested = num_items_requested / chunk_reduction_factor;
            if num_reduced_items_requested <= 1 {
                start_index // Limit the chunk to a single item
            } else {
                start_index + num_reduced_items_requested - 1 // Limit the chunk by the reduction factor
            }
        } else {
            end_index // No need to limit the chunk
        }
    }

    /// Returns the known epoch for the given version using the
    /// highest epoch ending ledger infos.
    fn find_known_epoch_for_version(&self, known_version: u64) -> u64 {
        // Find the epoch for the given version using the highest epoch ending ledger infos
        for (epoch, ledger_info) in self.highest_epoch_ending_ledger_infos.iter() {
            let epoch_ending_ledger_version = ledger_info.ledger_info().version();
            if epoch_ending_ledger_version > known_version {
                return *epoch;
            }
        }

        // Otherwise, return the max epoch + 1
        MAX_REAL_EPOCH_END + 1
    }

    /// Verifies the request timeout value for the given request type
    fn verify_request_timeout_value(
        &self,
        request_timeout_ms: u64,
        is_optimistic_fetch_request: bool,
        is_subscription_request: bool,
        data_request: DataRequest,
    ) {
        if !self.skip_timeout_verification {
            // Verify the given timeout for the request
            let expected_timeout = if is_optimistic_fetch_request {
                self.velor_data_client_config.optimistic_fetch_timeout_ms
            } else if is_subscription_request {
                self.velor_data_client_config
                    .subscription_response_timeout_ms
            } else {
                let min_timeout = self.velor_data_client_config.response_timeout_ms;
                let max_timeout = self.velor_data_client_config.max_response_timeout_ms;

                // Check how many times the given request has been made
                // and update the request counter.
                let mut data_request_counter_lock = self.data_request_counter.lock();
                let num_times_requested =
                    *data_request_counter_lock.get(&data_request).unwrap_or(&0);
                let _ = data_request_counter_lock
                    .deref_mut()
                    .insert(data_request, num_times_requested + 1);
                drop(data_request_counter_lock);

                // Calculate the expected timeout based on exponential backoff
                min(
                    max_timeout,
                    min_timeout * (u32::pow(2, num_times_requested as u32) as u64),
                )
            };

            // Verify the request timeouts match
            assert_eq!(request_timeout_ms, expected_timeout);
        }
    }
}

#[async_trait]
impl VelorDataClientInterface for MockVelorDataClient {
    fn get_global_data_summary(&self) -> GlobalDataSummary {
        // Create a random set of optimal chunk sizes to emulate changing environments
        let optimal_chunk_sizes = OptimalChunkSizes {
            state_chunk_size: create_range_random_u64(10, 200),
            epoch_chunk_size: create_non_zero_random_u64(10),
            transaction_chunk_size: create_range_random_u64(20, 1000),
            transaction_output_chunk_size: create_range_random_u64(20, 1000),
        };

        // Create a global data summary with a fixed set of data
        let advertised_data = AdvertisedData {
            states: vec![
                CompleteDataRange::new(MIN_ADVERTISED_STATES, MAX_ADVERTISED_STATES).unwrap(),
            ],
            epoch_ending_ledger_infos: vec![CompleteDataRange::new(
                MIN_ADVERTISED_EPOCH_END,
                MAX_ADVERTISED_EPOCH_END,
            )
            .unwrap()],
            synced_ledger_infos: self.advertised_synced_ledger_infos.clone(),
            transactions: vec![CompleteDataRange::new(
                MIN_ADVERTISED_TRANSACTION,
                MAX_ADVERTISED_TRANSACTION,
            )
            .unwrap()],
            transaction_outputs: vec![CompleteDataRange::new(
                MIN_ADVERTISED_TRANSACTION_OUTPUT,
                MAX_ADVERTISED_TRANSACTION_OUTPUT,
            )
            .unwrap()],
        };
        GlobalDataSummary {
            advertised_data,
            optimal_chunk_sizes,
        }
    }

    async fn get_state_values_with_proof(
        &self,
        version: Version,
        start_index: u64,
        end_index: u64,
        request_timeout_ms: u64,
    ) -> Result<Response<StateValueChunkWithProof>, velor_data_client::error::Error> {
        // Verify the request timeout
        let data_request = DataRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
            version,
            start_index,
            end_index,
        });
        self.verify_request_timeout_value(request_timeout_ms, false, false, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Calculate the last index based on if we should limit the chunk size
        let end_index = self.calculate_last_index(start_index, end_index);

        // Create state keys and values according to the given indices
        let mut state_keys_and_values = vec![];
        for _ in start_index..=end_index {
            state_keys_and_values.push((
                StateKey::raw(HashValue::random().as_ref()),
                StateValue::from(vec![]),
            ));
        }

        // Create a state value chunk with proof
        let state_value_chunk_with_proof = StateValueChunkWithProof {
            first_index: start_index,
            last_index: end_index,
            first_key: HashValue::random(),
            last_key: HashValue::random(),
            raw_values: state_keys_and_values,
            proof: SparseMerkleRangeProof::new(vec![]),
            root_hash: HashValue::zero(),
        };

        // Create and send a data client response
        Ok(create_data_client_response(state_value_chunk_with_proof))
    }

    async fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: Epoch,
        end_epoch: Epoch,
        request_timeout_ms: u64,
    ) -> Result<Response<Vec<LedgerInfoWithSignatures>>, velor_data_client::error::Error> {
        // Verify the request timeout
        let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
            start_epoch,
            expected_end_epoch: end_epoch,
        });
        self.verify_request_timeout_value(request_timeout_ms, false, false, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Calculate the last epoch based on if we should limit the chunk size
        let end_epoch = self.calculate_last_index(start_epoch, end_epoch);

        // Fetch the epoch ending ledger infos according to the requested epochs
        let mut epoch_ending_ledger_infos = vec![];
        for epoch in start_epoch..=end_epoch {
            let ledger_info = if epoch <= MAX_ADVERTISED_EPOCH_END {
                self.advertised_epoch_ending_ledger_infos
                    .get(&epoch)
                    .unwrap()
            } else {
                self.highest_epoch_ending_ledger_infos.get(&epoch).unwrap()
            };
            epoch_ending_ledger_infos.push(ledger_info.clone());
        }

        // Create and send a data client response
        Ok(create_data_client_response(epoch_ending_ledger_infos))
    }

    async fn get_new_transaction_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        request_timeout_ms: u64,
    ) -> Result<
        Response<(TransactionOutputListWithProofV2, LedgerInfoWithSignatures)>,
        velor_data_client::error::Error,
    > {
        // Verify the request timeout
        let data_request =
            DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
                known_version,
                known_epoch,
            });
        self.verify_request_timeout_value(request_timeout_ms, true, false, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Attempt to fetch and serve the new data
        if self.data_beyond_highest_advertised && known_version < MAX_REAL_TRANSACTION_OUTPUT {
            // Create a mock data client without timeout verification (to handle the internal requests)
            let velor_data_client = self.clone_without_timeout_verification();

            // Determine the target ledger info
            let target_ledger_info =
                determine_target_ledger_info(known_epoch, request_timeout_ms, &velor_data_client)
                    .await;

            // Fetch the new transaction outputs
            let outputs_with_proof = velor_data_client
                .get_transaction_outputs_with_proof(
                    known_version + 1,
                    known_version + 1,
                    known_version + 1,
                    self.velor_data_client_config.response_timeout_ms,
                )
                .await
                .unwrap()
                .payload;

            // Create and send a data client response
            return Ok(create_data_client_response((
                outputs_with_proof,
                target_ledger_info,
            )));
        }

        // Otherwise, emulate a network request timeout
        Err(self.emulate_network_request_timeout().await)
    }

    async fn get_new_transactions_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> Result<
        Response<(TransactionListWithProofV2, LedgerInfoWithSignatures)>,
        velor_data_client::error::Error,
    > {
        // Verify the request timeout
        let data_request =
            DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
                known_version,
                known_epoch,
                include_events,
            });
        self.verify_request_timeout_value(request_timeout_ms, true, false, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Attempt to fetch and serve the new data
        if self.data_beyond_highest_advertised && known_version < MAX_REAL_TRANSACTION {
            // Create a mock data client without timeout verification (to handle the internal requests)
            let velor_data_client = self.clone_without_timeout_verification();

            // Determine the target ledger info
            let target_ledger_info =
                determine_target_ledger_info(known_epoch, request_timeout_ms, &velor_data_client)
                    .await;

            // Fetch the new transactions
            let transactions_with_proof = velor_data_client
                .get_transactions_with_proof(
                    known_version + 1,
                    known_version + 1,
                    known_version + 1,
                    include_events,
                    self.velor_data_client_config.response_timeout_ms,
                )
                .await
                .unwrap()
                .payload;

            // Create and send a data client response
            return Ok(create_data_client_response((
                transactions_with_proof,
                target_ledger_info,
            )));
        }

        // Otherwise, emulate a network request timeout
        Err(self.emulate_network_request_timeout().await)
    }

    async fn get_new_transactions_or_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> velor_data_client::error::Result<
        Response<(TransactionOrOutputListWithProofV2, LedgerInfoWithSignatures)>,
    > {
        // Verify the request timeout
        let data_request = DataRequest::GetNewTransactionsOrOutputsWithProof(
            NewTransactionsOrOutputsWithProofRequest {
                known_version,
                known_epoch,
                include_events,
                max_num_output_reductions: 3,
            },
        );
        self.verify_request_timeout_value(request_timeout_ms, true, false, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Create a mock data client without timeout verification (to handle the internal requests)
        let velor_data_client = self.clone_without_timeout_verification();

        // Get the new transactions or outputs response
        let response = if return_transactions_instead_of_outputs() {
            let (transactions, ledger_info) = velor_data_client
                .get_new_transactions_with_proof(
                    known_version,
                    known_epoch,
                    include_events,
                    request_timeout_ms,
                )
                .await?
                .payload;
            ((Some(transactions), None), ledger_info)
        } else {
            let (outputs, ledger_info) = velor_data_client
                .get_new_transaction_outputs_with_proof(
                    known_version,
                    known_epoch,
                    request_timeout_ms,
                )
                .await?
                .payload;
            ((None, Some(outputs)), ledger_info)
        };

        // Create and send a data client response
        Ok(create_data_client_response(response))
    }

    async fn get_number_of_states(
        &self,
        version: Version,
        request_timeout_ms: u64,
    ) -> Result<Response<u64>, velor_data_client::error::Error> {
        // Verify the request timeout
        let data_request = DataRequest::GetNumberOfStatesAtVersion(version);
        self.verify_request_timeout_value(request_timeout_ms, false, false, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Create and send a data client response
        Ok(create_data_client_response(TOTAL_NUM_STATE_VALUES))
    }

    async fn get_transaction_outputs_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        request_timeout_ms: u64,
    ) -> Result<Response<TransactionOutputListWithProofV2>, velor_data_client::error::Error> {
        // Verify the request timeout
        let data_request =
            DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
            });
        self.verify_request_timeout_value(request_timeout_ms, false, false, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Calculate the last version based on if we should limit the chunk size
        let end_version = self.calculate_last_index(start_version, end_version);

        // Create the output list with proof
        let output_list_with_proof = create_output_list_with_proof(start_version, end_version);

        // Create and send a data client response
        Ok(create_data_client_response(output_list_with_proof))
    }

    async fn get_transactions_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> Result<Response<TransactionListWithProofV2>, velor_data_client::error::Error> {
        // Verify the request timeout
        let data_request = DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
            proof_version,
            start_version,
            end_version,
            include_events,
        });
        self.verify_request_timeout_value(request_timeout_ms, false, false, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Calculate the last version based on if we should limit the chunk size
        let end_version = self.calculate_last_index(start_version, end_version);

        // Create the transaction list with proof
        let transaction_list_with_proof =
            create_transaction_list_with_proof(start_version, end_version, include_events);

        // Create and send a data client response
        Ok(create_data_client_response(transaction_list_with_proof))
    }

    async fn get_transactions_or_outputs_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> velor_data_client::error::Result<Response<TransactionOrOutputListWithProofV2>> {
        // Verify the request timeout
        let data_request =
            DataRequest::GetTransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
                include_events,
                max_num_output_reductions: 3,
            });
        self.verify_request_timeout_value(request_timeout_ms, false, false, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Create a mock data client without timeout verification (to handle the internal requests)
        let velor_data_client = self.clone_without_timeout_verification();

        // Get the transactions or outputs response
        let transactions_or_outputs = if return_transactions_instead_of_outputs() {
            let transactions_with_proof = velor_data_client
                .get_transactions_with_proof(
                    proof_version,
                    start_version,
                    end_version,
                    include_events,
                    request_timeout_ms,
                )
                .await?;
            (Some(transactions_with_proof.payload), None)
        } else {
            let outputs_with_proof = velor_data_client
                .get_transaction_outputs_with_proof(
                    proof_version,
                    start_version,
                    end_version,
                    request_timeout_ms,
                )
                .await?;
            (None, Some(outputs_with_proof.payload))
        };

        // Create and send a data client response
        Ok(create_data_client_response(transactions_or_outputs))
    }

    async fn subscribe_to_transaction_outputs_with_proof(
        &self,
        subscription_request_metadata: SubscriptionRequestMetadata,
        request_timeout_ms: u64,
    ) -> velor_data_client::error::Result<
        Response<(TransactionOutputListWithProofV2, LedgerInfoWithSignatures)>,
    > {
        // Extract the known version, known epoch and the subscription stream index
        let known_version_at_stream_start =
            subscription_request_metadata.known_version_at_stream_start;
        let known_epoch_at_stream_start = subscription_request_metadata.known_epoch_at_stream_start;
        let subscription_stream_index = subscription_request_metadata.subscription_stream_index;

        // Verify the request timeout
        let data_request = DataRequest::SubscribeTransactionOutputsWithProof(
            SubscribeTransactionOutputsWithProofRequest {
                subscription_stream_metadata: SubscriptionStreamMetadata {
                    known_version_at_stream_start,
                    known_epoch_at_stream_start,
                    subscription_stream_id: subscription_request_metadata.subscription_stream_id,
                },
                subscription_stream_index,
            },
        );
        self.verify_request_timeout_value(request_timeout_ms, false, true, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Update the known version and epoch
        let known_version = known_version_at_stream_start + subscription_stream_index;

        // Attempt to fetch and serve the new data
        if self.data_beyond_highest_advertised && known_version < MAX_REAL_TRANSACTION_OUTPUT {
            // Create a mock data client without timeout verification (to handle the internal requests)
            let velor_data_client = self.clone_without_timeout_verification();

            // Determine the target ledger info
            let known_epoch = self.find_known_epoch_for_version(known_version);
            let target_ledger_info =
                determine_target_ledger_info(known_epoch, request_timeout_ms, &velor_data_client)
                    .await;

            // Fetch the new transaction outputs
            let outputs_with_proof = velor_data_client
                .get_transaction_outputs_with_proof(
                    known_version + 1,
                    known_version + 1,
                    known_version + 1,
                    self.velor_data_client_config.response_timeout_ms,
                )
                .await
                .unwrap()
                .payload;

            // Create and send a data client response
            return Ok(create_data_client_response((
                outputs_with_proof,
                target_ledger_info,
            )));
        }

        // Otherwise, emulate a network request timeout
        Err(self.emulate_network_request_timeout().await)
    }

    async fn subscribe_to_transactions_with_proof(
        &self,
        subscription_request_metadata: SubscriptionRequestMetadata,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> velor_data_client::error::Result<
        Response<(TransactionListWithProofV2, LedgerInfoWithSignatures)>,
    > {
        // Extract the known version, known epoch and the subscription stream index
        let known_version_at_stream_start =
            subscription_request_metadata.known_version_at_stream_start;
        let known_epoch_at_stream_start = subscription_request_metadata.known_epoch_at_stream_start;
        let subscription_stream_index = subscription_request_metadata.subscription_stream_index;

        // Verify the request timeout
        let data_request =
            DataRequest::SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest {
                subscription_stream_metadata: SubscriptionStreamMetadata {
                    known_version_at_stream_start,
                    known_epoch_at_stream_start,
                    subscription_stream_id: subscription_request_metadata.subscription_stream_id,
                },
                subscription_stream_index,
                include_events,
            });
        self.verify_request_timeout_value(request_timeout_ms, false, true, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Update the known version and epoch
        let known_version = known_version_at_stream_start + subscription_stream_index;

        // Attempt to fetch and serve the new data
        if self.data_beyond_highest_advertised && known_version < MAX_REAL_TRANSACTION_OUTPUT {
            // Create a mock data client without timeout verification (to handle the internal requests)
            let velor_data_client = self.clone_without_timeout_verification();

            // Determine the target ledger info
            let known_epoch = self.find_known_epoch_for_version(known_version);
            let target_ledger_info =
                determine_target_ledger_info(known_epoch, request_timeout_ms, &velor_data_client)
                    .await;

            // Fetch the new transaction outputs
            let outputs_with_proof = velor_data_client
                .get_transactions_with_proof(
                    known_version + 1,
                    known_version + 1,
                    known_version + 1,
                    include_events,
                    self.velor_data_client_config.response_timeout_ms,
                )
                .await
                .unwrap()
                .payload;

            // Create and send a data client response
            return Ok(create_data_client_response((
                outputs_with_proof,
                target_ledger_info,
            )));
        }

        // Otherwise, emulate a network request timeout
        Err(self.emulate_network_request_timeout().await)
    }

    async fn subscribe_to_transactions_or_outputs_with_proof(
        &self,
        subscription_request_metadata: SubscriptionRequestMetadata,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> velor_data_client::error::Result<
        Response<(TransactionOrOutputListWithProofV2, LedgerInfoWithSignatures)>,
    > {
        // Extract the known version, known epoch and the subscription stream index
        let known_version_at_stream_start =
            subscription_request_metadata.known_version_at_stream_start;
        let known_epoch_at_stream_start = subscription_request_metadata.known_epoch_at_stream_start;
        let subscription_stream_index = subscription_request_metadata.subscription_stream_index;

        // Verify the request timeout
        let data_request = DataRequest::SubscribeTransactionsOrOutputsWithProof(
            SubscribeTransactionsOrOutputsWithProofRequest {
                subscription_stream_metadata: SubscriptionStreamMetadata {
                    known_version_at_stream_start,
                    known_epoch_at_stream_start,
                    subscription_stream_id: subscription_request_metadata.subscription_stream_id,
                },
                subscription_stream_index,
                include_events,
                max_num_output_reductions: 3,
            },
        );
        self.verify_request_timeout_value(request_timeout_ms, false, true, data_request);

        // Emulate network latencies
        self.emulate_network_latencies().await;

        // Create a mock data client without timeout verification (to handle the internal requests)
        let velor_data_client = self.clone_without_timeout_verification();

        // Send the new transactions or outputs response
        let response = if return_transactions_instead_of_outputs() {
            let (transactions, ledger_info) = velor_data_client
                .subscribe_to_transactions_with_proof(
                    subscription_request_metadata,
                    include_events,
                    request_timeout_ms,
                )
                .await?
                .payload;
            ((Some(transactions), None), ledger_info)
        } else {
            let (outputs, ledger_info) = velor_data_client
                .subscribe_to_transaction_outputs_with_proof(
                    subscription_request_metadata,
                    request_timeout_ms,
                )
                .await?
                .payload;
            ((None, Some(outputs)), ledger_info)
        };

        // Create and send a data client response
        Ok(create_data_client_response(response))
    }
}

#[derive(Debug)]
pub struct NoopResponseCallback;

impl ResponseCallback for NoopResponseCallback {
    fn notify_bad_response(&self, _error: ResponseError) {
        // TODO(philiphayes): do something here
    }
}

/// Creates a data client response using a specified payload and random id
pub fn create_data_client_response<T>(payload: T) -> Response<T> {
    let id = create_random_u64(MAX_RESPONSE_ID);
    let context = ResponseContext::new(id, Box::new(NoopResponseCallback));
    Response::new(context, payload)
}

/// Creates a ledger info with the given version and epoch. If `epoch_ending`
/// is true, makes the ledger info an epoch ending ledger info.
pub fn create_ledger_info(
    version: Version,
    epoch: Epoch,
    epoch_ending: bool,
) -> LedgerInfoWithSignatures {
    let next_epoch_state = if epoch_ending {
        let mut epoch_state = EpochState::empty();
        epoch_state.epoch = epoch + 1;
        Some(epoch_state)
    } else {
        None
    };

    let block_info = BlockInfo::new(
        epoch,
        0,
        HashValue::zero(),
        HashValue::zero(),
        version,
        0,
        next_epoch_state,
    );
    LedgerInfoWithSignatures::new(
        LedgerInfo::new(block_info, HashValue::zero()),
        AggregateSignature::empty(),
    )
}

/// Creates epoch ending ledger infos for the given epoch and version range
fn create_epoch_ending_ledger_infos(
    start_epoch: Epoch,
    start_version: Version,
    end_epoch: Epoch,
    end_version: Version,
) -> BTreeMap<Epoch, LedgerInfoWithSignatures> {
    let mut current_epoch = start_epoch;
    let mut current_version = start_version;

    // Populate the epoch ending ledger infos using random intervals
    let max_num_versions_in_epoch = (end_version - start_version) / ((end_epoch + 1) - start_epoch);
    let mut epoch_ending_ledger_infos = BTreeMap::new();
    while current_epoch < end_epoch + 1 {
        let num_versions_in_epoch = create_non_zero_random_u64(max_num_versions_in_epoch);
        current_version += num_versions_in_epoch;

        if epoch_ending_ledger_infos
            .insert(
                current_epoch,
                create_ledger_info(current_version, current_epoch, true),
            )
            .is_some()
        {
            panic!("Duplicate epoch ending ledger info found! This should not occur!",);
        }
        current_epoch += 1;
    }

    epoch_ending_ledger_infos
}

/// Creates a set of synced ledger infos given the versions and epochs range
fn create_synced_ledger_infos(
    start_epoch: Epoch,
    start_version: Version,
    end_epoch: Epoch,
    end_version: Version,
    epoch_ending_ledger_infos: &BTreeMap<Epoch, LedgerInfoWithSignatures>,
) -> Vec<LedgerInfoWithSignatures> {
    let mut current_epoch = start_epoch;
    let mut current_version = start_version;

    // Populate the synced ledger infos
    let mut synced_ledger_infos = vec![];
    while current_version < end_version && current_epoch < end_epoch {
        let random_num_versions = create_non_zero_random_u64(10);
        current_version += random_num_versions;

        let end_of_epoch_version = epoch_ending_ledger_infos
            .get(&current_epoch)
            .unwrap()
            .ledger_info()
            .version();
        if current_version > end_of_epoch_version {
            current_epoch += 1;
        }

        let end_of_epoch = end_of_epoch_version == current_version;
        synced_ledger_infos.push(create_ledger_info(
            current_version,
            current_epoch,
            end_of_epoch,
        ));
    }

    // Manually insert a synced ledger info at the last transaction and highest
    // epoch to ensure we can sync right up to the end.
    synced_ledger_infos.push(create_ledger_info(end_version, end_epoch + 1, false));

    synced_ledger_infos
}

/// Creates a simple test transaction
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

/// Creates an empty transaction output
fn create_transaction_output() -> TransactionOutput {
    TransactionOutput::new(
        WriteSet::default(),
        vec![],
        0,
        TransactionStatus::Retry,
        TransactionAuxiliaryData::default(),
    )
}

/// Returns a random u64 with a value between 0 and `max_value` - 1 (inclusive).
pub fn create_random_u64(max_value: u64) -> u64 {
    create_range_random_u64(0, max_value)
}

/// Returns a random (but non-zero) u64 with a value between 1 and `max_value` - 1 (inclusive).
fn create_non_zero_random_u64(max_value: u64) -> u64 {
    create_range_random_u64(1, max_value)
}

/// Returns a random u64 within the range, [min, max)
fn create_range_random_u64(min_value: u64, max_value: u64) -> u64 {
    let mut rng = OsRng;
    rng.gen_range(min_value, max_value)
}

/// Determines the target ledger info for the given known epoch
async fn determine_target_ledger_info(
    known_epoch: Epoch,
    request_timeout_ms: u64,
    velor_data_client: &MockVelorDataClient,
) -> LedgerInfoWithSignatures {
    if known_epoch <= MAX_REAL_EPOCH_END {
        // Fetch the epoch ending ledger info
        velor_data_client
            .get_epoch_ending_ledger_infos(known_epoch, known_epoch, request_timeout_ms)
            .await
            .unwrap()
            .payload[0]
            .clone()
    } else {
        // Return a synced ledger info at the last version and highest epoch
        create_ledger_info(MAX_REAL_TRANSACTION, MAX_REAL_EPOCH_END + 1, false)
    }
}

/// Initializes the Velor logger for tests
pub fn initialize_logger() {
    velor_logger::Logger::builder()
        .is_async(false)
        .level(Level::Info)
        .build();
}

/// Returns a data notification from the given stream listener
pub async fn get_data_notification(
    stream_listener: &mut DataStreamListener,
) -> Result<DataNotification, Error> {
    if let Ok(data_notification) = timeout(
        Duration::from_secs(MAX_NOTIFICATION_TIMEOUT_SECS),
        stream_listener.select_next_some(),
    )
    .await
    {
        Ok(data_notification)
    } else {
        Err(Error::UnexpectedErrorEncountered(
            "Timed out waiting for a data notification!".into(),
        ))
    }
}

/// Creates a transaction list with proof for testing
pub fn create_transaction_list_with_proof(
    start_version: u64,
    end_version: u64,
    include_events: bool,
) -> TransactionListWithProofV2 {
    // Include events if required
    let events = if include_events { Some(vec![]) } else { None };

    // Create the requested transactions
    let mut transactions = vec![];
    for _ in start_version..=end_version {
        transactions.push(create_transaction());
    }

    // Create a transaction list with an empty proof
    let mut transaction_list_with_proof = TransactionListWithProof::new_empty();
    transaction_list_with_proof.first_transaction_version = Some(start_version);
    transaction_list_with_proof.events = events;
    transaction_list_with_proof.transactions = transactions;

    TransactionListWithProofV2::new_from_v1(transaction_list_with_proof)
}

/// Creates an output list with proof for testing
pub fn create_output_list_with_proof(
    start_version: u64,
    end_version: u64,
) -> TransactionOutputListWithProofV2 {
    // Create a transaction list with proof
    let transaction_list_with_proof =
        create_transaction_list_with_proof(start_version, end_version, false);

    // Create a transaction output list with proof
    let transactions_and_outputs = transaction_list_with_proof
        .get_transaction_list_with_proof()
        .transactions
        .iter()
        .map(|txn| (txn.clone(), create_transaction_output()))
        .collect();
    let output_list_with_proof = TransactionOutputListWithProof::new(
        transactions_and_outputs,
        Some(start_version),
        transaction_list_with_proof
            .get_transaction_list_with_proof()
            .proof
            .clone(),
    );

    TransactionOutputListWithProofV2::new_from_v1(output_list_with_proof)
}

/// Returns true iff the server should return transactions
/// instead of outputs.
///
/// Note: there's a 50-50 chance of this occurring.
fn return_transactions_instead_of_outputs() -> bool {
    (create_random_u64(u64::MAX) % 2) == 0
}
