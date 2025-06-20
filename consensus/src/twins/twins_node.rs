// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    epoch_manager::EpochManager,
    network::NetworkTask,
    network_interface::{ConsensusNetworkClient, DIRECT_SEND, RPC},
    network_tests::{NetworkPlayground, TwinId},
    payload_manager::DirectMempoolPayloadManager,
    pipeline::buffer_manager::OrderedBlocks,
    quorum_store::quorum_store_db::MockQuorumStoreDB,
    rand::rand_gen::storage::in_memory::InMemRandDb,
    test_utils::{mock_execution_client::MockExecutionClient, MockStorage},
    util::time_service::ClockTimeService,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::{NodeConfig, WaypointConfig},
    generator::{self, ValidatorSwarm},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_consensus_types::common::{Author, Round};
use aptos_event_notifications::{ReconfigNotification, ReconfigNotificationListener};
use aptos_mempool::mocks::MockSharedMempool;
use aptos_network::{
    application::interface::{NetworkClient, NetworkServiceEvents},
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{
        network,
        network::{NetworkEvents, NewNetworkEvents, NewNetworkSender},
        wire::handshake::v1::ProtocolIdSet,
    },
    transport::ConnectionMetadata,
    ProtocolId,
};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::{
        ConsensusConfigV1, InMemoryOnChainConfig, OnChainConfig, OnChainConfigPayload,
        OnChainConsensusConfig,
        ProposerElectionType::{self, RoundProposer},
        ValidatorSet,
    },
    transaction::SignedTransaction,
    validator_info::ValidatorInfo,
    waypoint::Waypoint,
};
use aptos_validator_transaction_pool::VTxnPoolState;
use futures::{channel::mpsc, StreamExt};
use maplit::hashmap;
use std::{collections::HashMap, iter::FromIterator, sync::Arc};
use tokio::runtime::Runtime;

/// Auxiliary struct that is preparing SMR for the test
pub struct SMRNode {
    pub id: TwinId,
    pub storage: Arc<MockStorage>,
    pub commit_cb_receiver: mpsc::UnboundedReceiver<LedgerInfoWithSignatures>,
    _runtime: Runtime,
    _shared_mempool: MockSharedMempool,
    _state_sync: mpsc::UnboundedReceiver<Vec<SignedTransaction>>,
}

fn author_from_config(config: &NodeConfig) -> Author {
    config.validator_network.as_ref().unwrap().peer_id()
}

impl SMRNode {
    fn start(
        playground: &mut NetworkPlayground,
        config: NodeConfig,
        consensus_config: OnChainConsensusConfig,
        storage: Arc<MockStorage>,
        twin_id: TwinId,
        vtxn_pool: VTxnPoolState,
    ) -> Self {
        // Create a runtime for the twin
        let thread_name = format!("twin-{}", twin_id.id);
        let runtime = aptos_runtimes::spawn_named_runtime(thread_name, None);
        let _entered_runtime = runtime.enter();

        // Setup the network and SMR node
        let (network_reqs_tx, network_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
        let (connection_reqs_tx, _) = aptos_channel::new(QueueStyle::FIFO, 8, None);
        let (consensus_tx, consensus_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
        let (_conn_mgr_reqs_tx, conn_mgr_reqs_rx) = aptos_channels::new_test(8);
        let network_sender = network::NetworkSender::new(
            PeerManagerRequestSender::new(network_reqs_tx),
            ConnectionRequestSender::new(connection_reqs_tx),
        );
        let network_client = NetworkClient::new(
            DIRECT_SEND.into(),
            RPC.into(),
            hashmap! {NetworkId::Validator => network_sender},
            playground.peer_protocols(),
        );
        let consensus_network_client = ConsensusNetworkClient::new(network_client);
        let network_events = NetworkEvents::new(consensus_rx, None, true);
        let network_service_events =
            NetworkServiceEvents::new(hashmap! {NetworkId::Validator => network_events});

        playground.add_node(twin_id, consensus_tx, network_reqs_rx, conn_mgr_reqs_rx);

        let (state_sync_client, state_sync) = mpsc::unbounded();
        let (ordered_blocks_tx, mut ordered_blocks_events) = mpsc::unbounded::<OrderedBlocks>();
        let shared_mempool = MockSharedMempool::new();
        let (quorum_store_to_mempool_sender, _) = mpsc::channel(1_024);

        let execution_client = Arc::new(MockExecutionClient::new(
            state_sync_client,
            ordered_blocks_tx,
            Arc::clone(&storage),
        ));
        let (reconfig_sender, reconfig_events) = aptos_channel::new(QueueStyle::LIFO, 1, None);
        let reconfig_listener = ReconfigNotificationListener {
            notification_receiver: reconfig_events,
        };
        let _commit_notifier = Arc::from(DirectMempoolPayloadManager::new());
        let mut configs = HashMap::new();
        configs.insert(
            ValidatorSet::CONFIG_ID,
            bcs::to_bytes(storage.get_validator_set()).unwrap(),
        );
        configs.insert(
            OnChainConsensusConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&consensus_config).unwrap()).unwrap(),
        );
        let payload = OnChainConfigPayload::new(1, InMemoryOnChainConfig::new(configs));

        reconfig_sender
            .push((), ReconfigNotification {
                version: 1,
                on_chain_configs: payload,
            })
            .unwrap();

        let time_service = Arc::new(ClockTimeService::new(runtime.handle().clone()));

        let (timeout_sender, timeout_receiver) =
            aptos_channels::new(1_024, &counters::PENDING_ROUND_TIMEOUTS);
        let (self_sender, self_receiver) =
            aptos_channels::new_unbounded(&counters::PENDING_SELF_MESSAGES);

        let quorum_store_storage = Arc::new(MockQuorumStoreDB::new());
        let bounded_executor = BoundedExecutor::new(2, playground.handle());

        let epoch_mgr = EpochManager::new(
            &config,
            time_service,
            self_sender,
            consensus_network_client,
            timeout_sender,
            quorum_store_to_mempool_sender,
            execution_client.clone(),
            storage.clone(),
            quorum_store_storage,
            reconfig_listener,
            bounded_executor,
            aptos_time_service::TimeService::real(),
            vtxn_pool,
            Arc::new(InMemRandDb::new()),
            None,
        );
        let (network_task, network_receiver) =
            NetworkTask::new(network_service_events, self_receiver);

        runtime.spawn(network_task.start());
        runtime.spawn(epoch_mgr.start(timeout_receiver, network_receiver));

        let (commit_cb_sender, commit_cb_receiver) = mpsc::unbounded::<LedgerInfoWithSignatures>();
        runtime.spawn(async move {
            loop {
                let ordered_blocks = ordered_blocks_events.next().await.unwrap();
                let commit = ordered_blocks.ordered_proof.clone();
                execution_client
                    .commit_to_storage(ordered_blocks)
                    .await
                    .unwrap();

                commit_cb_sender.unbounded_send(commit.clone()).unwrap();
            }
        });

        Self {
            id: twin_id,
            _runtime: runtime,
            commit_cb_receiver,
            storage,
            _shared_mempool: shared_mempool,
            _state_sync: state_sync,
        }
    }

    /// Starts a given number of nodes and their twins
    pub fn start_num_nodes_with_twins(
        num_nodes: usize,
        num_twins: usize,
        playground: &mut NetworkPlayground,
        proposer_type: ProposerElectionType,
        round_proposers_idx: Option<HashMap<Round, usize>>,
    ) -> Vec<Self> {
        assert!(num_nodes >= num_twins);
        let ValidatorSwarm {
            nodes: mut node_configs,
        } = generator::validator_swarm_for_testing(num_nodes);
        let peers_and_metadata = playground.peer_protocols();
        node_configs.iter().for_each(|config| {
            let peer_id = author_from_config(config);
            let mut conn_meta = ConnectionMetadata::mock(peer_id);
            conn_meta.application_protocols = ProtocolIdSet::from_iter([
                ProtocolId::ConsensusDirectSendJson,
                ProtocolId::ConsensusDirectSendBcs,
                ProtocolId::ConsensusRpcBcs,
            ]);
            let peer_network_id = PeerNetworkId::new(NetworkId::Validator, peer_id);
            peers_and_metadata
                .insert_connection_metadata(peer_network_id, conn_meta)
                .unwrap();
        });

        node_configs.sort_by_key(author_from_config);
        let validator_set = ValidatorSet::new(
            node_configs
                .iter()
                .enumerate()
                .map(|(index, config)| {
                    let sr_test_config = config.consensus.safety_rules.test.as_ref().unwrap();
                    ValidatorInfo::new_with_test_network_keys(
                        sr_test_config.author,
                        sr_test_config.consensus_key.as_ref().unwrap().public_key(),
                        1,
                        index as u64,
                    )
                })
                .collect(),
        );
        // sort by the peer id

        let proposer_type = match proposer_type {
            RoundProposer(_) => {
                let mut round_proposers: HashMap<Round, Author> = HashMap::new();

                if let Some(proposers) = round_proposers_idx {
                    proposers.iter().for_each(|(round, idx)| {
                        round_proposers.insert(*round, author_from_config(&node_configs[*idx]));
                    })
                }
                RoundProposer(round_proposers)
            },
            _ => proposer_type,
        };

        // We don't add twins to ValidatorSet or round_proposers above
        // because a node with twins should be treated the same at the
        // consensus level
        for i in 0..num_twins {
            let twin = node_configs[i].clone();
            node_configs.push(twin);
        }

        let mut smr_nodes = vec![];

        for (smr_id, mut config) in node_configs.into_iter().enumerate() {
            let (_, storage) = MockStorage::start_for_testing(validator_set.clone());

            let waypoint = Waypoint::new_epoch_boundary(&storage.get_ledger_info())
                .expect("Unable to produce waypoint with the provided LedgerInfo");
            config
                .consensus
                .safety_rules
                .test
                .as_mut()
                .unwrap()
                .waypoint = Some(waypoint);
            config.base.waypoint = WaypointConfig::FromConfig(waypoint);
            // Disable timeout in twins test to avoid flakiness
            config.consensus.round_initial_timeout_ms = 2_000_000;

            let author = author_from_config(&config);

            let twin_id = TwinId { id: smr_id, author };

            let consensus_config = OnChainConsensusConfig::V1(ConsensusConfigV1 {
                proposer_election_type: proposer_type.clone(),
                ..ConsensusConfigV1::default()
            });

            let vtxn_pool = VTxnPoolState::default();
            smr_nodes.push(Self::start(
                playground,
                config,
                consensus_config,
                storage,
                twin_id,
                vtxn_pool,
            ));
        }
        smr_nodes
    }
}
