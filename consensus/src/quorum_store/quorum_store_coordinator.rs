// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network::NetworkSender;
use crate::quorum_store::batch_aggregator::BatchAggregator;
use crate::quorum_store::batch_reader::BatchReader;
use crate::quorum_store::batch_store::{BatchStore, BatchStoreCommand};
use crate::quorum_store::network_listener::NetworkListener;
use crate::quorum_store::proof_builder::{ProofBuilder, ProofBuilderCommand};
use crate::quorum_store::quorum_store_db::QuorumStoreDB;
use crate::round_manager::VerifiedEvent;
use aptos_channels::aptos_channel;
use aptos_config::config::QuorumStoreConfig;
use aptos_consensus_types::common::Round;
use aptos_logger::prelude::*;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_types::PeerId;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;

pub struct QuorumStoreCoordinator {
    epoch: u64,
    my_peer_id: PeerId,
    network_sender: NetworkSender,
    batch_aggregator: BatchAggregator,
    batch_store_tx: Sender<BatchStoreCommand>,
    proof_builder_tx: Sender<ProofBuilderCommand>,
    network_listener_tx_vec: Vec<aptos_channel::Sender<PeerId, VerifiedEvent>>,
}

impl QuorumStoreCoordinator {
    // TODO: pass epoch state
    pub fn new(
        epoch: u64, //TODO: pass the epoch config
        last_committed_round: Round,
        my_peer_id: PeerId,
        db: Arc<QuorumStoreDB>,
        network_msg_rx_vec: Vec<aptos_channel::Receiver<PeerId, VerifiedEvent>>,
        nerwork_listener_tx_vec: Vec<aptos_channel::Sender<PeerId, VerifiedEvent>>,
        network_sender: NetworkSender,
        config: QuorumStoreConfig,
        validator_verifier: ValidatorVerifier, //TODO: pass the epoch config
        signer: ValidatorSigner,
    ) -> (Self, Arc<BatchReader>) {
        debug!(
            "QS: QuorumStore new, epoch = {}, last r = {}, timeout ms {}",
            epoch, last_committed_round, config.batch_request_timeout_ms,
        );
        let validator_signer = Arc::new(signer);

        // Prepare communication channels among the threads.
        let (batch_store_tx, batch_store_rx) = channel(config.channel_size);
        let (batch_reader_tx, batch_reader_rx) = channel(config.channel_size);
        let (proof_builder_tx, proof_builder_rx) = channel(config.channel_size);

        let proof_builder = ProofBuilder::new(config.proof_timeout_ms, my_peer_id);
        let (batch_store, batch_reader) = BatchStore::new(
            epoch,
            last_committed_round,
            my_peer_id,
            network_sender.clone(),
            batch_store_tx.clone(),
            batch_reader_tx.clone(),
            batch_reader_rx,
            db,
            validator_verifier.clone(),
            validator_signer.clone(),
            config.batch_expiry_round_gap_when_init,
            config.batch_expiry_round_gap_behind_latest_certified,
            config.batch_expiry_round_gap_beyond_latest_certified,
            config.batch_expiry_grace_rounds,
            config.batch_request_num_peers,
            config.batch_request_timeout_ms,
            config.memory_quota,
            config.db_quota,
        );

        // let metrics_monitor = tokio_metrics::TaskMonitor::new();
        // {
        //     let metrics_monitor = metrics_monitor.clone();
        //     tokio::spawn(async move {
        //         for interval in metrics_monitor.intervals() {
        //             println!("QuorumStore:{:?}", interval);
        //             tokio::time::sleep(Duration::from_secs(5)).await;
        //         }
        //     });
        // }

        tokio::spawn(proof_builder.start(proof_builder_rx, validator_verifier));

        // _ = spawn_named!(
        //     &("Quorum:ProofBuilder epoch ".to_owned() + &epoch.to_string()),
        //     metrics_monitor.instrument(proof_builder.start(proof_builder_rx, validator_verifier))
        // );

        for network_msg_rx in network_msg_rx_vec.into_iter() {
            let net = NetworkListener::new(
                epoch,
                network_msg_rx,
                batch_store_tx.clone(),
                batch_reader_tx.clone(),
                proof_builder_tx.clone(),
                config.max_batch_bytes,
            );
            tokio::spawn(net.start());

            // _ = spawn_named!(
            //     &("Quorum:NetworkListener epoch ".to_owned() + &epoch.to_string()),
            //     metrics_monitor.instrument(net.start())
            // );
        }

        tokio::spawn(batch_store.start(batch_store_rx, proof_builder_tx.clone()));

        // _ = spawn_named!(
        //     &("Quorum:BatchStore epoch ".to_owned() + &epoch.to_string()),
        //     metrics_monitor.instrument(batch_store.start(batch_store_rx, proof_builder_tx.clone()))
        // );

        debug!("QS: QuorumStore created");
        (
            Self {
                epoch,
                my_peer_id,
                network_sender,
                network_listener_tx_vec: nerwork_listener_tx_vec,
                batch_aggregator: BatchAggregator::new(config.max_batch_bytes),
                batch_store_tx,
                proof_builder_tx,
            },
            batch_reader,
        )
    }

    pub async fn start(
        mut self,
        mut shutdown_rx: futures_channel::mpsc::Receiver<futures_channel::oneshot::Sender<()>>,
    ) {
        while let Some(ack_tx) = shutdown_rx.next().await {
            // TODO: shutdown batch generator and proof manager and batch coordinator

            let (batch_store_shutdown_tx, batch_store_shutdown_rx) = oneshot::channel();
            self.batch_store_tx
                .send(BatchStoreCommand::Shutdown(batch_store_shutdown_tx))
                .await
                .expect("Failed to send to BatchStore");

            batch_store_shutdown_rx
                .await
                .expect("Failed to stop BatchStore");

            let (proof_builder_shutdown_tx, proof_builder_shutdown_rx) = oneshot::channel();
            self.proof_builder_tx
                .send(ProofBuilderCommand::Shutdown(proof_builder_shutdown_tx))
                .await
                .expect("Failed to send to ProofBuilder");

            proof_builder_shutdown_rx
                .await
                .expect("Failed to stop ProofBuilder");

            for network_listener_tx in self.network_listener_tx_vec {
                let (network_listener_shutdown_tx, network_listener_shutdown_rx) =
                    oneshot::channel();
                match network_listener_tx.push(
                    self.my_peer_id,
                    VerifiedEvent::Shutdown(network_listener_shutdown_tx),
                ) {
                    Ok(()) => debug!("QS: shutdown network listener sent"),
                    Err(err) => panic!("Failed to send to NetworkListener, Err {:?}", err),
                };
                network_listener_shutdown_rx
                    .await
                    .expect("Failed to stop NetworkListener");
            }

            ack_tx
                .send(())
                .expect("Failed to send shutdown ack from QuorumStore");
            break;
        }
    }
}
