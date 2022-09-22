// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{data_notification::DataNotification, data_stream::DataStreamListener, error::Error};
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use aptos_data_client::{
    AdvertisedData, AptosDataClient, GlobalDataSummary, OptimalChunkSizes, Response,
    ResponseCallback, ResponseContext, ResponseError,
};
use aptos_logger::Level;
use aptos_types::aggregate_signature::AggregateSignature;
use aptos_types::{
    account_address::AccountAddress,
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
        RawTransaction, Script, SignedTransaction, Transaction, TransactionListWithProof,
        TransactionOutput, TransactionOutputListWithProof, TransactionPayload, TransactionStatus,
        Version,
    },
    write_set::WriteSet,
};
use async_trait::async_trait;
use futures::StreamExt;
use rand::{rngs::OsRng, Rng};
use std::{collections::HashMap, thread, time::Duration};
use storage_service_types::responses::CompleteDataRange;
use storage_service_types::Epoch;
use tokio::time::timeout;

/// The number of state values held at any version
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
pub const MAX_REAL_EPOCH_END: u64 = MAX_ADVERTISED_EPOCH_END + 5;
pub const MAX_REAL_TRANSACTION: u64 = MAX_ADVERTISED_TRANSACTION + 5000;
pub const MAX_REAL_TRANSACTION_OUTPUT: u64 = MAX_REAL_TRANSACTION;
pub const MAX_RESPONSE_ID: u64 = 100000;

/// Test timeout constant
pub const MAX_NOTIFICATION_TIMEOUT_SECS: u64 = 40;

/// A simple mock of the Aptos Data Client
#[derive(Clone, Debug)]
pub struct MockAptosDataClient {
    pub advertised_epoch_ending_ledger_infos: HashMap<Epoch, LedgerInfoWithSignatures>,
    pub advertised_synced_ledger_infos: Vec<LedgerInfoWithSignatures>,
    pub data_beyond_highest_advertised: bool, // If true, data exists beyond the highest advertised
    pub highest_epoch_ending_ledger_infos: HashMap<Epoch, LedgerInfoWithSignatures>,
    pub limit_chunk_sizes: bool, // If true, responses will be truncated to emulate chunk and network limits
    pub skip_emulate_network_latencies: bool, // If true, skips network latency emulation
}

impl MockAptosDataClient {
    pub fn new(
        data_beyond_highest_advertised: bool,
        limit_chunk_sizes: bool,
        skip_emulate_network_latencies: bool,
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

        Self {
            advertised_epoch_ending_ledger_infos,
            advertised_synced_ledger_infos,
            data_beyond_highest_advertised,
            highest_epoch_ending_ledger_infos,
            limit_chunk_sizes,
            skip_emulate_network_latencies,
        }
    }

    fn emulate_network_latencies(&self) {
        if self.skip_emulate_network_latencies {
            return;
        }

        // Sleep for 10 - 50 ms to emulate variance
        thread::sleep(Duration::from_millis(create_range_random_u64(10, 50)));
    }

    fn emulate_subscription_expiration(&self) -> aptos_data_client::Error {
        thread::sleep(Duration::from_secs(MAX_NOTIFICATION_TIMEOUT_SECS));
        aptos_data_client::Error::TimeoutWaitingForResponse("RPC timed out!".into())
    }

    fn calculate_last_index(&self, start_index: u64, end_index: u64) -> u64 {
        if self.limit_chunk_sizes {
            let num_items_requested = (end_index - start_index) + 1;
            let chunk_reduction_factor = create_range_random_u64(2, 9);
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
}

#[async_trait]
impl AptosDataClient for MockAptosDataClient {
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
        _version: Version,
        start_index: u64,
        end_index: u64,
    ) -> Result<Response<StateValueChunkWithProof>, aptos_data_client::Error> {
        self.emulate_network_latencies();

        // Calculate the last index based on if we should limit the chunk size
        let end_index = self.calculate_last_index(start_index, end_index);

        // Create state keys and values according to the given indices
        let mut state_keys_and_values = vec![];
        for _ in start_index..=end_index {
            state_keys_and_values.push((
                StateKey::Raw(HashValue::random().to_vec()),
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
        Ok(create_data_client_response(state_value_chunk_with_proof))
    }

    async fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: Epoch,
        end_epoch: Epoch,
    ) -> Result<Response<Vec<LedgerInfoWithSignatures>>, aptos_data_client::Error> {
        self.emulate_network_latencies();

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
        Ok(create_data_client_response(epoch_ending_ledger_infos))
    }

    async fn get_new_transaction_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
    ) -> Result<
        Response<(TransactionOutputListWithProof, LedgerInfoWithSignatures)>,
        aptos_data_client::Error,
    > {
        self.emulate_network_latencies();

        // Attempt to fetch the new data
        if self.data_beyond_highest_advertised && known_version < MAX_REAL_TRANSACTION_OUTPUT {
            let target_ledger_info = if known_epoch <= MAX_REAL_EPOCH_END {
                // Fetch the epoch ending ledger info
                self.get_epoch_ending_ledger_infos(known_epoch, known_epoch)
                    .await
                    .unwrap()
                    .payload[0]
                    .clone()
            } else {
                // Return a synced ledger info at the last version and highest epoch
                create_ledger_info(MAX_REAL_TRANSACTION_OUTPUT, MAX_REAL_EPOCH_END + 1, false)
            };

            // Fetch the new transaction outputs
            let target_ledger_version = target_ledger_info.ledger_info().version();
            let outputs_with_proof = self
                .get_transaction_outputs_with_proof(
                    target_ledger_version,
                    known_version + 1,
                    target_ledger_version,
                )
                .await
                .unwrap()
                .payload;

            // Return the new data
            Ok(create_data_client_response((
                outputs_with_proof,
                target_ledger_info,
            )))
        } else {
            Err(self.emulate_subscription_expiration())
        }
    }

    async fn get_new_transactions_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
    ) -> Result<
        Response<(TransactionListWithProof, LedgerInfoWithSignatures)>,
        aptos_data_client::Error,
    > {
        self.emulate_network_latencies();

        // Attempt to fetch the new data
        if self.data_beyond_highest_advertised && known_version < MAX_REAL_TRANSACTION {
            self.emulate_network_latencies();
            let target_ledger_info = if known_epoch <= MAX_REAL_EPOCH_END {
                // Fetch the epoch ending ledger info
                self.get_epoch_ending_ledger_infos(known_epoch, known_epoch)
                    .await
                    .unwrap()
                    .payload[0]
                    .clone()
            } else {
                // Return a synced ledger info at the last version and highest epoch
                create_ledger_info(MAX_REAL_TRANSACTION, MAX_REAL_EPOCH_END + 1, false)
            };

            // Fetch the new transactions
            let target_ledger_version = target_ledger_info.ledger_info().version();
            let transactions_with_proof = self
                .get_transactions_with_proof(
                    target_ledger_version,
                    known_version + 1,
                    target_ledger_version,
                    include_events,
                )
                .await
                .unwrap()
                .payload;

            // Return the new data
            Ok(create_data_client_response((
                transactions_with_proof,
                target_ledger_info,
            )))
        } else {
            Err(self.emulate_subscription_expiration())
        }
    }

    async fn get_number_of_states(
        &self,
        _version: Version,
    ) -> Result<Response<u64>, aptos_data_client::Error> {
        Ok(create_data_client_response(TOTAL_NUM_STATE_VALUES))
    }

    async fn get_transaction_outputs_with_proof(
        &self,
        _proof_version: Version,
        start_version: Version,
        end_version: Version,
    ) -> Result<Response<TransactionOutputListWithProof>, aptos_data_client::Error> {
        self.emulate_network_latencies();

        // Calculate the last version based on if we should limit the chunk size
        let end_version = self.calculate_last_index(start_version, end_version);

        // Create the requested transactions and transaction outputs
        let mut transactions_and_outputs = vec![];
        for _ in start_version..=end_version {
            transactions_and_outputs.push((create_transaction(), create_transaction_output()));
        }

        // Create a transaction output list with an empty proof
        let mut output_list_with_proof = TransactionOutputListWithProof::new_empty();
        output_list_with_proof.first_transaction_output_version = Some(start_version);
        output_list_with_proof.transactions_and_outputs = transactions_and_outputs;
        Ok(create_data_client_response(output_list_with_proof))
    }

    async fn get_transactions_with_proof(
        &self,
        _proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
    ) -> Result<Response<TransactionListWithProof>, aptos_data_client::Error> {
        self.emulate_network_latencies();

        // Calculate the last version based on if we should limit the chunk size
        let end_version = self.calculate_last_index(start_version, end_version);

        let transaction_list_with_proof =
            create_transaction_list_with_proof(start_version, end_version, include_events);

        // Return the transaction list with proofs
        Ok(create_data_client_response(transaction_list_with_proof))
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
    let response_callback = Box::new(NoopResponseCallback);
    let context = ResponseContext {
        id,
        response_callback,
    };
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
) -> HashMap<Epoch, LedgerInfoWithSignatures> {
    let mut current_epoch = start_epoch;
    let mut current_version = start_version;

    // Populate the epoch ending ledger infos using random intervals
    let max_num_versions_in_epoch = (end_version - start_version) / ((end_epoch + 1) - start_epoch);
    let mut epoch_ending_ledger_infos = HashMap::new();
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
    epoch_ending_ledger_infos: &HashMap<Epoch, LedgerInfoWithSignatures>,
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
    TransactionOutput::new(WriteSet::default(), vec![], 0, TransactionStatus::Retry)
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

/// Initializes the Aptos logger for tests
pub fn initialize_logger() {
    aptos_logger::Logger::builder()
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

pub fn create_transaction_list_with_proof(
    start_version: u64,
    end_version: u64,
    include_events: bool,
) -> TransactionListWithProof {
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

    transaction_list_with_proof
}
