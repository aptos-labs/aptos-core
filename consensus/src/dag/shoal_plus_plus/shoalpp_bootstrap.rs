// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0


use std::{sync::Arc, time::Duration};
use futures_channel::mpsc::UnboundedSender;
use futures_channel::oneshot;
use tokio::sync::mpsc::{channel, Receiver};
use tokio_retry::strategy::ExponentialBackoff;
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::aptos_channel;
use aptos_channels::message_queues::QueueStyle;
use aptos_config::config::DagConsensusConfig;
use aptos_consensus_types::common::Author;
use aptos_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use aptos_types::epoch_state::EpochState;
use aptos_types::on_chain_config::{DagConsensusConfigV1, Features, ValidatorTxnConfig};
use aptos_types::validator_signer::ValidatorSigner;
use crate::dag::{DagBootstrapper, DAGMessage, DAGRpcResult, ProofNotifier, TDAGNetworkSender};
use crate::dag::shoal_plus_plus::shoalpp_broadcast_sync::{BoltBroadcastSync, BroadcastSync};
use crate::dag::shoal_plus_plus::shoalpp_handler::BoltHandler;
use crate::dag::shoal_plus_plus::shoalpp_types::{BoltBCParms, BoltBCRet};
use crate::dag::storage::DAGStorage;
use crate::network::IncomingShoalppRequest;
use crate::payload_client::PayloadClient;
use crate::payload_manager::PayloadManager;
use crate::pipeline::buffer_manager::OrderedBlocks;
use crate::pipeline::execution_client::TExecutionClient;


pub struct ShoalppBootstrapper {
    epoch_state: Arc<EpochState>,
    dags: Vec<DagBootstrapper>,
    receivers: Vec<Receiver<(oneshot::Sender<BoltBCRet>, BoltBCParms)>>,
    rb: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff, DAGRpcResult>>,
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
        executor: BoundedExecutor,
        features: Features,
    ) -> Self {
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

        for dag_id in 0..3 {
            let (broadcast_sender, broadcast_receiver) = channel(100);
            receiver_vec.push(broadcast_receiver);
            let dag_bootstrapper = DagBootstrapper::new(
                dag_id,
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
                payload_client.clone(),
                ordered_nodes_tx.clone(),
                execution_client.clone(),
                quorum_store_enabled,
                vtxn_config.clone(),
                executor.clone(),
                features.clone(),
                rb.clone(),
                broadcast_sender,
            );
            dags.push(dag_bootstrapper);
        }
        Self {
            epoch_state,
            dags,
            receivers: receiver_vec,
            rb,
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
            let (dag_rpc_tx, dag_rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 10, None);
            dag_rpc_tx_vec.push(dag_rpc_tx);
            let (dag_shutdown_tx, dag_shutdown_rx) = oneshot::channel();
            dag_shutdown_tx_vec.push(dag_shutdown_tx);
            tokio::spawn(dag_bootstrapper.start(
                dag_rpc_rx,
                dag_shutdown_rx,
            ));
        });
        let bolt_handler = BoltHandler::new(self.epoch_state.clone());
        tokio::spawn(bolt_handler.run(
            shoalpp_rpc_rx,
            shutdown_rx,
            dag_rpc_tx_vec,
            dag_shutdown_tx_vec,
        ));

        let broadcast_sync = BoltBroadcastSync::new(self.rb.clone(), self.receivers);
        tokio::spawn(broadcast_sync.run());
    }

}

