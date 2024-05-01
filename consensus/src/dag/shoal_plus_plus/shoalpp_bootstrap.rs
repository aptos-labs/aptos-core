// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        self,
        adapter::{compute_initial_block_and_ledger_info, LedgerInfoProvider},
        shoal_plus_plus::{
            shoalpp_broadcast_sync::{BoltBroadcastSync, BroadcastSync},
            shoalpp_handler::BoltHandler,
            shoalpp_order_notifier::ShoalppOrderNotifier,
            shoalpp_payload_client::ShoalppPayloadClient,
            shoalpp_types::{BoltBCParms, BoltBCRet},
        },
        storage::DAGStorage,
        DAGMessage, DAGRpcResult, DagBootstrapper, ProofNotifier, TDAGNetworkSender,
    },
    network::IncomingShoalppRequest,
    payload_client::PayloadClient,
    payload_manager::PayloadManager,
    pipeline::{buffer_manager::OrderedBlocks, execution_client::TExecutionClient},
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::DagConsensusConfig;
use aptos_consensus_types::common::Author;
use aptos_infallible::RwLock;
use aptos_logger::{debug, error};
use aptos_network::application::storage::PeersAndMetadata;
use aptos_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use aptos_types::{
    epoch_state::EpochState,
    on_chain_config::{
        DagConsensusConfigV1, OnChainJWKConsensusConfig, OnChainRandomnessConfig,
        ValidatorTxnConfig,
    },
    validator_signer::ValidatorSigner,
};
use arc_swap::ArcSwapOption;
use futures::executor::block_on;
use futures_channel::{mpsc::UnboundedSender, oneshot};
use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::sync::mpsc::{channel, Receiver};
use tokio_retry::strategy::ExponentialBackoff;

pub struct ShoalppBootstrapper {
    epoch_state: Arc<EpochState>,
    dags: Vec<DagBootstrapper>,
    receivers: Vec<Receiver<(oneshot::Sender<BoltBCRet>, BoltBCParms)>>,
    rb: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff, DAGRpcResult>>,
    shoalpp_order_notifier: ShoalppOrderNotifier,
}

impl ShoalppBootstrapper {
    pub fn new(
        self_peer: Author,
        config: DagConsensusConfig,
        onchain_config: DagConsensusConfigV1,
        signer: Arc<ValidatorSigner>,
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        rb_network_sender: Arc<dyn RBNetworkSender<DAGMessage, DAGRpcResult>>,
        dag_network_sender: Arc<dyn TDAGNetworkSender>,
        proof_notifier: Arc<dyn ProofNotifier>,
        time_service: aptos_time_service::TimeService,
        payload_manager: Arc<PayloadManager>,
        payload_client: Arc<dyn PayloadClient>,
        ordered_nodes_tx: UnboundedSender<OrderedBlocks>,
        execution_client: Arc<dyn TExecutionClient>,
        quorum_store_enabled: bool,
        vtxn_config: ValidatorTxnConfig,
        randomness_config: OnChainRandomnessConfig,
        jwk_consensus_config: OnChainJWKConsensusConfig,
        executor: BoundedExecutor,
        allow_batches_without_pos_in_proposal: bool,
        peers_and_metadata: Arc<PeersAndMetadata>,
    ) -> Self {
        let ledger_info_from_storage = storage
            .get_latest_ledger_info()
            .expect("latest ledger info must exist");
        let (block_info, ledger_info) =
            compute_initial_block_and_ledger_info(ledger_info_from_storage);

        let ledger_info_provider = Arc::new(RwLock::new(LedgerInfoProvider::new(
            epoch_state.clone(),
            ledger_info,
        )));

        let mut dag_store_vec = Vec::new();
        for _i in 0..3 {
            // TDOO: consider changing  ArcSwapOption -> Mutex
            dag_store_vec.push(Arc::new(ArcSwapOption::from(None)));
        }

        let shoalpp_payload_client = Arc::new(ShoalppPayloadClient::new(
            dag_store_vec.clone(),
            payload_client,
            ledger_info_provider.clone(),
            onchain_config.dag_ordering_causal_history_window as u64,
        ));
        let validators = epoch_state.verifier.get_ordered_account_addresses();
        let rb_config = config.rb_config.clone();
        // A backoff policy that starts at _base_*_factor_ ms and multiplies by _base_ each iteration.
        let rb_backoff_policy = ExponentialBackoff::from_millis(rb_config.backoff_policy_base_ms)
            .factor(rb_config.backoff_policy_factor)
            .max_delay(Duration::from_millis(rb_config.backoff_policy_max_delay_ms));
        let rb = Arc::new(ReliableBroadcast::new(
            validators.clone(),
            rb_network_sender.clone(),
            rb_backoff_policy,
            time_service.clone(),
            Duration::from_millis(rb_config.rpc_timeout_ms),
            executor.clone(),
        ));
        let mut dags = Vec::new();
        let mut receiver_vec = Vec::new();
        let mut receiver_ordered_nodes_vec = Vec::new();

        let mut txs = VecDeque::new();
        let mut rxs = VecDeque::new();
        for _ in 1..=3 {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            txs.push_back(tx);
            rxs.push_back(rx);
        }
        txs[0].send(());
        txs.rotate_left(1);
        for dag_id in 0..3 {
            let (order_nodes_tx, ordered_node_rx) = tokio::sync::mpsc::unbounded_channel();
            let (broadcast_sender, broadcast_receiver) = channel(100);
            receiver_vec.push(broadcast_receiver);
            receiver_ordered_nodes_vec.push(ordered_node_rx);

            let tx = txs.pop_front().unwrap();
            let rx = rxs.pop_front().unwrap();

            let dag_bootstrapper = DagBootstrapper::new(
                dag_id as u8,
                self_peer,
                config.clone(),
                onchain_config.clone(),
                signer.clone(),
                epoch_state.clone(),
                storage.clone(),
                rb_network_sender.clone(),
                dag_network_sender.clone(),
                proof_notifier.clone(),
                time_service.clone(),
                payload_manager.clone(),
                shoalpp_payload_client.clone(),
                order_nodes_tx,
                execution_client.clone(),
                quorum_store_enabled,
                vtxn_config.clone(),
                randomness_config.clone(),
                jwk_consensus_config.clone(),
                executor.clone(),
                allow_batches_without_pos_in_proposal,
                peers_and_metadata.clone(),
                rb.clone(),
                broadcast_sender,
                ledger_info_provider.clone(),
                dag_store_vec[dag_id as usize].clone(),
                tx,
                rx,
            );
            dags.push(dag_bootstrapper);
        }

        let shoalpp_order_notifier = ShoalppOrderNotifier::new(
            dag_store_vec,
            ordered_nodes_tx,
            receiver_ordered_nodes_vec,
            ledger_info_provider,
            epoch_state.clone(),
            block_info,
        );
        Self {
            epoch_state,
            dags,
            receivers: receiver_vec,
            rb,
            shoalpp_order_notifier,
        }
    }

    pub async fn start(
        self,
        shoalpp_rpc_rx: aptos_channel::Receiver<Author, (Author, IncomingShoalppRequest)>,
        shutdown_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        assert_eq!(self.dags.len(), 3);
        let mut dag_rpc_tx_vec = Vec::new();
        let mut dag_shutdown_tx_vec = Vec::new();

        self.dags.into_iter().for_each(|dag_bootstrapper| {
            let (dag_rpc_tx, dag_rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 50, None);
            dag_rpc_tx_vec.push(dag_rpc_tx);
            let (dag_shutdown_tx, dag_shutdown_rx) = oneshot::channel();
            dag_shutdown_tx_vec.push(dag_shutdown_tx);
            tokio::spawn(dag_bootstrapper.start(dag_rpc_rx, dag_shutdown_rx));
        });

        let notifier_tokio_handler = tokio::spawn(self.shoalpp_order_notifier.run());

        let shoalpp_handler = BoltHandler::new(self.epoch_state.clone());
        let handler_tokio_handler =
            tokio::spawn(shoalpp_handler.run(shoalpp_rpc_rx, dag_rpc_tx_vec));

        let broadcast_sync = BoltBroadcastSync::new(self.rb.clone(), self.receivers);
        let broadcast_sync_tokio_handler = tokio::spawn(broadcast_sync.run());

        if let Ok(ack_tx) = shutdown_rx.await {
            debug!("[Bolt] shutting down Bolt");
            notifier_tokio_handler.abort();
            broadcast_sync_tokio_handler.abort();
            let _ = block_on(notifier_tokio_handler);
            let _ = block_on(broadcast_sync_tokio_handler);
            while !dag_shutdown_tx_vec.is_empty() {
                let (ack_tx, ack_rx) = oneshot::channel();
                dag_shutdown_tx_vec
                    .pop()
                    .unwrap()
                    .send(ack_tx)
                    .expect("[Bolt] Fail to drop DAG bootstrapper");
                ack_rx.await.expect("[Bolt] Fail to drop DAG bootstrapper");
            }
            if let Err(e) = ack_tx.send(()) {
                error!(error = ?e, "unable to ack to shutdown signal");
            }
            debug!("[Bolt] shutting down Bolt");
            handler_tokio_handler.abort();
            let _ = block_on(handler_tokio_handler);

            return;
        } else {
            error!("[Bolt] failed to receive shutdown");
        }
    }
}
