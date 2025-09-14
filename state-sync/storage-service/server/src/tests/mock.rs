// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics, network::StorageServiceNetworkEvents, storage::StorageReader, tests::utils,
    StorageServiceServer,
};
use anyhow::Result;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::{StateSyncConfig, StorageServiceConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_crypto::HashValue;
use aptos_network::{
    application::{interface::NetworkServiceEvents, storage::PeersAndMetadata},
    protocols::{
        network::{NetworkEvents, NewNetworkEvents, ReceivedMessage},
        wire::{
            handshake::v1::ProtocolId,
            messaging::v1::{NetworkMessage, RpcRequest},
        },
    },
};
use aptos_storage_interface::{DbReader, LedgerSummary, Order};
use aptos_storage_service_notifications::StorageServiceNotifier;
use aptos_storage_service_types::{
    requests::StorageServiceRequest, responses::StorageServiceResponse, StorageServiceError,
    StorageServiceMessage,
};
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{
    account_address::AccountAddress,
    contract_event::{ContractEvent, EventWithVersion},
    epoch_change::EpochChangeProof,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        AccumulatorConsistencyProof, SparseMerkleProof, TransactionAccumulatorRangeProof,
        TransactionAccumulatorSummary,
    },
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        AccountOrderedTransactionsWithProof, PersistedAuxiliaryInfo, Transaction,
        TransactionAuxiliaryData, TransactionInfo, TransactionListWithProofV2,
        TransactionOutputListWithProofV2, TransactionWithProof, Version,
    },
    write_set::WriteSet,
    PeerId,
};
use futures::channel::{oneshot, oneshot::Receiver};
use mockall::mock;
use rand::{rngs::OsRng, Rng};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::timeout;

// Useful test constants
const MAX_RESPONSE_TIMEOUT_SECS: u64 = 60;

/// A wrapper around the inbound network interface/channel for easily sending
/// mock client requests to a [`StorageServiceServer`].
pub struct MockClient {
    peer_manager_notifiers:
        HashMap<NetworkId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
}

impl MockClient {
    pub fn new(
        db_reader: Option<MockDatabaseReader>,
        storage_config: Option<StorageServiceConfig>,
    ) -> (
        Self,
        StorageServiceServer<StorageReader>,
        StorageServiceNotifier,
        MockTimeService,
        Arc<PeersAndMetadata>,
    ) {
        utils::initialize_logger();

        // Create the state sync config
        let mut state_sync_config = StateSyncConfig::default();
        let storage_service_config = storage_config.unwrap_or_default();
        state_sync_config.storage_service = storage_service_config;

        // Create the storage reader
        let mock_time_service = TimeService::mock();
        let storage_reader = StorageReader::new(
            storage_service_config,
            Arc::new(db_reader.unwrap_or_else(create_mock_db_reader)),
            mock_time_service.clone(),
        );

        // Setup the networks and the network events
        let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
        let mut network_and_events = HashMap::new();
        let mut peer_manager_notifiers = HashMap::new();
        for network_id in network_ids.clone() {
            let queue_cfg = aptos_channel::Config::new(
                storage_service_config.max_network_channel_size as usize,
            )
            .queue_style(QueueStyle::FIFO)
            .counters(&metrics::PENDING_STORAGE_SERVER_NETWORK_EVENTS);
            let (peer_manager_notifier, peer_manager_notification_receiver) = queue_cfg.build();

            let network_events = NetworkEvents::new(peer_manager_notification_receiver, None, true);
            network_and_events.insert(network_id, network_events);
            peer_manager_notifiers.insert(network_id, peer_manager_notifier);
        }
        let storage_service_network_events =
            StorageServiceNetworkEvents::new(NetworkServiceEvents::new(network_and_events));

        // Create the storage service notifier and listener
        let (storage_service_notifier, storage_service_listener) =
            aptos_storage_service_notifications::new_storage_service_notifier_listener_pair();

        // Create the storage service
        let peers_and_metadata = create_peers_and_metadata(network_ids);
        let executor = tokio::runtime::Handle::current();
        let storage_server = StorageServiceServer::new(
            state_sync_config,
            executor,
            storage_reader,
            mock_time_service.clone(),
            peers_and_metadata.clone(),
            storage_service_network_events,
            storage_service_listener,
        );

        // Return the client and service
        let mock_client = Self {
            peer_manager_notifiers,
        };
        (
            mock_client,
            storage_server,
            storage_service_notifier,
            mock_time_service.into_mock(),
            peers_and_metadata,
        )
    }

    /// Send the given storage request and wait for a response
    pub async fn process_request(
        &mut self,
        request: StorageServiceRequest,
    ) -> Result<StorageServiceResponse, StorageServiceError> {
        let receiver = self.send_request(request, None, None).await;
        self.wait_for_response(receiver).await
    }

    /// Send the specified storage request and return the receiver on which to
    /// expect a result.
    pub async fn send_request(
        &mut self,
        request: StorageServiceRequest,
        peer_id: Option<AccountAddress>,
        network_id: Option<NetworkId>,
    ) -> Receiver<Result<bytes::Bytes, aptos_network::protocols::network::RpcError>> {
        // Create the inbound rpc request
        let peer_id = peer_id.unwrap_or_else(PeerId::random);
        let network_id = network_id.unwrap_or_else(get_random_network_id);
        let protocol_id = ProtocolId::StorageServiceRpc;
        let data = protocol_id
            .to_bytes(&StorageServiceMessage::Request(request))
            .unwrap();
        let (res_tx, res_rx) = oneshot::channel();
        let notification = ReceivedMessage {
            message: NetworkMessage::RpcRequest(RpcRequest {
                protocol_id,
                request_id: 0,
                priority: 0,
                raw_request: data,
            }),
            sender: PeerNetworkId::new(network_id, peer_id),
            receive_timestamp_micros: 0,
            rpc_replier: Some(Arc::new(res_tx)),
        };

        // Push the request up to the storage service
        self.peer_manager_notifiers
            .get(&network_id)
            .unwrap()
            .push((peer_id, protocol_id), notification)
            .unwrap();

        res_rx
    }

    /// Helper method to wait for and deserialize a response on the specified receiver
    pub async fn wait_for_response(
        &mut self,
        receiver: Receiver<Result<bytes::Bytes, aptos_network::protocols::network::RpcError>>,
    ) -> Result<StorageServiceResponse, StorageServiceError> {
        if let Ok(response) =
            timeout(Duration::from_secs(MAX_RESPONSE_TIMEOUT_SECS), receiver).await
        {
            let response = ProtocolId::StorageServiceRpc
                .from_bytes::<StorageServiceMessage>(&response.unwrap().unwrap())
                .unwrap();
            match response {
                StorageServiceMessage::Response(response) => response,
                _ => panic!("Unexpected response message: {:?}", response),
            }
        } else {
            panic!("Timed out while waiting for a response from the storage service!")
        }
    }
}

/// Creates a peers and metadata struct for test purposes
pub fn create_peers_and_metadata(network_ids: Vec<NetworkId>) -> Arc<PeersAndMetadata> {
    PeersAndMetadata::new(&network_ids)
}

/// Returns a random network ID
fn get_random_network_id() -> NetworkId {
    let mut rng = OsRng;
    let random_number: u8 = rng.r#gen();
    match random_number % 3 {
        0 => NetworkId::Validator,
        1 => NetworkId::Vfn,
        2 => NetworkId::Public,
        num => panic!("This shouldn't be possible! Got num: {:?}", num),
    }
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
        ) -> aptos_storage_interface::Result<EpochChangeProof>;

        fn get_transactions(
            &self,
            start_version: Version,
            batch_size: u64,
            ledger_version: Version,
            fetch_events: bool,
        ) -> aptos_storage_interface::Result<TransactionListWithProofV2>;

        fn get_transaction_by_hash(
            &self,
            hash: HashValue,
            ledger_version: Version,
            fetch_events: bool,
        ) -> aptos_storage_interface::Result<Option<TransactionWithProof>>;

        fn get_transaction_by_version(
            &self,
            version: Version,
            ledger_version: Version,
            fetch_events: bool,
        ) -> aptos_storage_interface::Result<TransactionWithProof>;

        fn get_first_txn_version(&self) -> aptos_storage_interface::Result<Option<Version>>;

        fn get_first_write_set_version(&self) -> aptos_storage_interface::Result<Option<Version>>;

        fn get_transaction_outputs(
            &self,
            start_version: Version,
            limit: u64,
            ledger_version: Version,
        ) -> aptos_storage_interface::Result<TransactionOutputListWithProofV2>;

        fn get_events(
            &self,
            event_key: &EventKey,
            start: u64,
            order: Order,
            limit: u64,
            ledger_version: Version,
        ) -> aptos_storage_interface::Result<Vec<EventWithVersion>>;

        fn get_block_timestamp(&self, version: u64) -> aptos_storage_interface::Result<u64>;

        fn get_last_version_before_timestamp(
            &self,
            _timestamp: u64,
            _ledger_version: Version,
        ) -> aptos_storage_interface::Result<Version>;

        fn get_latest_ledger_info_option(&self) -> aptos_storage_interface::Result<Option<LedgerInfoWithSignatures>>;

        fn get_latest_ledger_info(&self) -> aptos_storage_interface::Result<LedgerInfoWithSignatures>;

        fn get_synced_version(&self) -> aptos_storage_interface::Result<Option<Version>>;

        fn get_latest_ledger_info_version(&self) -> aptos_storage_interface::Result<Version>;

        fn get_latest_commit_metadata(&self) -> aptos_storage_interface::Result<(Version, u64)>;

        fn get_account_ordered_transaction(
            &self,
            address: AccountAddress,
            seq_num: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> aptos_storage_interface::Result<Option<TransactionWithProof>>;

        fn get_account_ordered_transactions(
            &self,
            address: AccountAddress,
            seq_num: u64,
            limit: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> aptos_storage_interface::Result<AccountOrderedTransactionsWithProof>;

        fn get_state_proof_with_ledger_info(
            &self,
            known_version: u64,
            ledger_info: LedgerInfoWithSignatures,
        ) -> aptos_storage_interface::Result<StateProof>;

        fn get_state_proof(&self, known_version: u64) -> aptos_storage_interface::Result<StateProof>;

        fn get_state_value_with_proof_by_version(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> aptos_storage_interface::Result<(Option<StateValue>, SparseMerkleProof)>;

        fn get_pre_committed_ledger_summary(&self) -> aptos_storage_interface::Result<LedgerSummary>;

        fn get_epoch_ending_ledger_info(&self, known_version: u64) ->aptos_storage_interface::Result<LedgerInfoWithSignatures>;

        fn get_accumulator_root_hash(&self, _version: Version) -> aptos_storage_interface::Result<HashValue>;

        fn get_accumulator_consistency_proof(
            &self,
            _client_known_version: Option<Version>,
            _ledger_version: Version,
        ) -> aptos_storage_interface::Result<AccumulatorConsistencyProof>;

        fn get_accumulator_summary(
            &self,
            ledger_version: Version,
        ) -> aptos_storage_interface::Result<TransactionAccumulatorSummary>;

        fn get_state_item_count(&self, version: Version) -> aptos_storage_interface::Result<usize>;

        fn get_state_value_chunk_with_proof(
            &self,
            version: Version,
            start_idx: usize,
            chunk_size: usize,
        ) -> aptos_storage_interface::Result<StateValueChunkWithProof>;

        fn get_epoch_snapshot_prune_window(&self) -> aptos_storage_interface::Result<usize>;

        fn is_state_merkle_pruner_enabled(&self) -> aptos_storage_interface::Result<bool>;

        fn get_persisted_auxiliary_info_iterator(
            &self,
            start_version: Version,
            num_persisted_auxiliary_info: usize,
        ) -> aptos_storage_interface::Result<Box<dyn Iterator<Item = aptos_storage_interface::Result<PersistedAuxiliaryInfo>>>>;

        fn get_epoch_ending_ledger_info_iterator(
            &self,
            start_epoch: u64,
            end_epoch: u64,
        ) -> aptos_storage_interface::Result<Box<dyn Iterator<Item = aptos_storage_interface::Result<LedgerInfoWithSignatures>>>>;

        fn get_transaction_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> aptos_storage_interface::Result<Box<dyn Iterator<Item = aptos_storage_interface::Result<Transaction>>>>;

        fn get_transaction_info_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> aptos_storage_interface::Result<Box<dyn Iterator<Item = aptos_storage_interface::Result<TransactionInfo>>>>;

        fn get_events_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> aptos_storage_interface::Result<Box<dyn Iterator<Item = aptos_storage_interface::Result<Vec<ContractEvent>>>>>;

        fn get_write_set_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> aptos_storage_interface::Result<Box<dyn Iterator<Item = aptos_storage_interface::Result<WriteSet>>>>;

        fn get_auxiliary_data_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> aptos_storage_interface::Result<Box<dyn Iterator<Item = aptos_storage_interface::Result<TransactionAuxiliaryData>>>>;

        fn get_transaction_accumulator_range_proof(
            &self,
            start_version: Version,
            limit: u64,
            ledger_version: Version,
        ) -> aptos_storage_interface::Result<TransactionAccumulatorRangeProof>;

        fn get_state_value_chunk_iter(
            &self,
            version: Version,
            first_index: usize,
            chunk_size: usize,
        ) -> aptos_storage_interface::Result<Box<dyn Iterator<Item = aptos_storage_interface::Result<(StateKey, StateValue)>>>>;

        fn get_state_value_chunk_proof(
            &self,
            version: Version,
            first_index: usize,
            state_key_values: Vec<(StateKey, StateValue)>,
        ) -> aptos_storage_interface::Result<StateValueChunkWithProof>;
    }
}

/// Creates a mock db with the basic expectations required to
/// handle storage summary updates.
pub fn create_mock_db_with_summary_updates(
    highest_ledger_info: LedgerInfoWithSignatures,
    lowest_version: Version,
) -> MockDatabaseReader {
    // Create a new mock db reader
    let mut db_reader = create_mock_db_reader();

    // Set up the basic expectations to handle storage summary updates
    db_reader
        .expect_get_latest_ledger_info()
        .returning(move || Ok(highest_ledger_info.clone()));
    db_reader
        .expect_get_first_txn_version()
        .returning(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_first_write_set_version()
        .returning(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_epoch_snapshot_prune_window()
        .returning(move || Ok(100));
    db_reader
        .expect_is_state_merkle_pruner_enabled()
        .returning(move || Ok(true));

    db_reader
}

/// Creates a mock database reader
pub fn create_mock_db_reader() -> MockDatabaseReader {
    MockDatabaseReader::new()
}
