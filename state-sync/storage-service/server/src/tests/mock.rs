// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics, network::StorageServiceNetworkEvents, storage::StorageReader, StorageServiceServer,
};
use anyhow::Result;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{config::StorageServiceConfig, network_id::NetworkId};
use aptos_crypto::HashValue;
use aptos_logger::Level;
use aptos_network::{
    application::{interface::NetworkServiceEvents, storage::PeersAndMetadata},
    peer_manager::PeerManagerNotification,
    protocols::{
        network::{NetworkEvents, NewNetworkEvents},
        rpc::InboundRpcRequest,
        wire::handshake::v1::ProtocolId,
    },
};
use aptos_storage_interface::{DbReader, ExecutedTrees, Order};
use aptos_storage_service_types::{
    requests::StorageServiceRequest, responses::StorageServiceResponse, StorageServiceError,
    StorageServiceMessage,
};
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{
    account_address::AccountAddress,
    contract_event::EventWithVersion,
    epoch_change::EpochChangeProof,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    proof::{AccumulatorConsistencyProof, SparseMerkleProof, TransactionAccumulatorSummary},
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        AccountTransactionsWithProof, TransactionInfo, TransactionListWithProof,
        TransactionOutputListWithProof, TransactionWithProof, Version,
    },
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
        HashMap<NetworkId, aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>>,
}

impl MockClient {
    pub fn new(
        db_reader: Option<MockDatabaseReader>,
        storage_config: Option<StorageServiceConfig>,
    ) -> (
        Self,
        StorageServiceServer<StorageReader>,
        MockTimeService,
        Arc<PeersAndMetadata>,
    ) {
        initialize_logger();

        // Create the storage reader
        let storage_config = storage_config.unwrap_or_default();
        let storage_reader = StorageReader::new(
            storage_config,
            Arc::new(db_reader.unwrap_or_else(create_mock_db_reader)),
        );

        // Setup the networks and the network events
        let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
        let mut network_and_events = HashMap::new();
        let mut peer_manager_notifiers = HashMap::new();
        for network_id in network_ids.clone() {
            let queue_cfg =
                aptos_channel::Config::new(storage_config.max_network_channel_size as usize)
                    .queue_style(QueueStyle::FIFO)
                    .counters(&metrics::PENDING_STORAGE_SERVER_NETWORK_EVENTS);
            let (peer_manager_notifier, peer_manager_notification_receiver) = queue_cfg.build();
            let (_, connection_notification_receiver) = queue_cfg.build();

            let network_events = NetworkEvents::new(
                peer_manager_notification_receiver,
                connection_notification_receiver,
            );
            network_and_events.insert(network_id, network_events);
            peer_manager_notifiers.insert(network_id, peer_manager_notifier);
        }
        let storage_service_network_events =
            StorageServiceNetworkEvents::new(NetworkServiceEvents::new(network_and_events));

        // Create the storage service
        let peers_and_metadata = create_peers_and_metadata(network_ids);
        let executor = tokio::runtime::Handle::current();
        let mock_time_service = TimeService::mock();
        let storage_server = StorageServiceServer::new(
            storage_config,
            executor,
            storage_reader,
            mock_time_service.clone(),
            peers_and_metadata.clone(),
            storage_service_network_events,
        );

        // Return the client and service
        let mock_client = Self {
            peer_manager_notifiers,
        };
        (
            mock_client,
            storage_server,
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
        let inbound_rpc = InboundRpcRequest {
            protocol_id,
            data: data.into(),
            res_tx,
        };
        let notification = PeerManagerNotification::RecvRpc(peer_id, inbound_rpc);

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
    let random_number: u8 = rng.gen();
    match random_number % 3 {
        0 => NetworkId::Validator,
        1 => NetworkId::Vfn,
        2 => NetworkId::Public,
        num => panic!("This shouldn't be possible! Got num: {:?}", num),
    }
}

/// Initializes the Aptos logger for tests
fn initialize_logger() {
    aptos_logger::Logger::builder()
        .is_async(false)
        .level(Level::Debug)
        .build();
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
            ledger_version: Version,
        ) -> Result<Vec<EventWithVersion>>;

        fn get_block_timestamp(&self, version: u64) -> Result<u64>;

        fn get_last_version_before_timestamp(
            &self,
            _timestamp: u64,
            _ledger_version: Version,
        ) -> Result<Version>;

        fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>>;

        fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures>;

        fn get_latest_version(&self) -> Result<Version>;

        fn get_latest_commit_metadata(&self) -> Result<(Version, u64)>;

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

        fn get_state_value_with_proof_by_version(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<(Option<StateValue>, SparseMerkleProof)>;

        fn get_latest_executed_trees(&self) -> Result<ExecutedTrees>;

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

        fn get_epoch_snapshot_prune_window(&self) -> Result<usize>;

        fn is_state_merkle_pruner_enabled(&self) -> Result<bool>;
    }
}

/// Creates a mock db with the basic expectations required to handle optimistic fetch requests
pub fn create_mock_db_for_optimistic_fetch(
    highest_ledger_info_clone: LedgerInfoWithSignatures,
    lowest_version: Version,
) -> MockDatabaseReader {
    let mut db_reader = create_mock_db_reader();
    db_reader
        .expect_get_latest_ledger_info()
        .returning(move || Ok(highest_ledger_info_clone.clone()));
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
