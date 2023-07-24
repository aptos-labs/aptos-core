// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc, thread, time::Duration};
use aptos_consensus_types::common::Author;
use aptos_logger::info;
use aptos_types::{transaction::SignedTransaction, validator_verifier::ValidatorVerifier, epoch_state::EpochState};
use crate::dkg::types::{DKGNodeMetadata, DKGNode};
use serde::Serialize;
use tokio::{sync::{oneshot, mpsc}, time::Interval};
use crate::{
    quorum_store::batch_generator::BatchGeneratorCommand, block_storage::BlockReader,
    dag::reliable_broadcast::{ReliableBroadcast, DAGNetworkSender},
};
use aptos_dkg::pvss::scrape::Transcript;
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt,
};

use super::{dkg_store::DKGStorage, types::{DKGAggregatedNodes, DKGSignatureBuilder, DKGCertificateAckState, DKGCertifiedNode}};

// the transcript size is 3.25MB
const TRANSCRIPT_SIZE: usize = 3_250_000;
const TRANSCRIPT_COMPUTE_TIME_MS: u64 = 4760;
const TRANSCRIPT_VERIFY_TIME_MS: u64 = 555;
const TRANSCRIPT_AGGREGATE_TIME_MS: u64 = 21;

// dkg todo: use the same format for stake distribution as in PVSS library
#[derive(Debug)]
pub struct StakeDis {
    pub distribution: HashMap<Author, u64>,
}

#[derive(Debug)]
pub enum DKGManagerCommand {
    StartDKG(StakeDis),
    ReceivePVSS(Author, Transcript),
    Shutdown(futures_channel::oneshot::Sender<()>),
}

#[derive(Debug)]
pub enum DKGToProposalCommand {
    DKGPayload(DKGAggregatedNodes),
}

pub struct DKGManager {
    author: Author,
    epoch_state: Arc<EpochState>,
    storage: Arc<DKGStorage>,
    // Channel to send the aggregated dkg node to proposal generator
    proposal_generator_tx: mpsc::Sender<DKGAggregatedNodes>,
    reliable_broadcast: Arc<ReliableBroadcast>,
    rb_abort_handle: Option<AbortHandle>,
}

impl DKGManager {
    pub fn new(author: Author, epoch_state: Arc<EpochState>, storage: Arc<DKGStorage>, proposal_generator_tx: mpsc::Sender<DKGAggregatedNodes>, reliable_broadcast: Arc<ReliableBroadcast>) -> Self {
        Self {
            author,
            epoch_state,
            storage,
            proposal_generator_tx,
            reliable_broadcast,
            rb_abort_handle: None,
        }
    }

    fn compute_pvss(&mut self, stake_dis: StakeDis) -> anyhow::Result<()> {
        // dkg todo: compute pvss transcript
        thread::sleep(Duration::from_millis(TRANSCRIPT_COMPUTE_TIME_MS));
        Ok(())
    }

    pub fn broadcast_dkg_node(&mut self, node: DKGNode) {
        let rb = self.reliable_broadcast.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let signature_builder =
            DKGSignatureBuilder::new(node.metadata().clone(), self.epoch_state.clone());
        let cert_ack_set = DKGCertificateAckState::new(self.epoch_state.verifier.len());
        let task = self
            .reliable_broadcast
            .broadcast(node.clone(), signature_builder)
            .then(move |certificate| {
                let certified_node = DKGCertifiedNode::new(node, certificate.signatures().to_owned());
                rb.broadcast(certified_node, cert_ack_set)
            });
        tokio::spawn(Abortable::new(task, abort_registration));
        if let Some(prev_handle) = self.rb_abort_handle.replace(abort_handle) {
            prev_handle.abort();
        }
    }

    pub fn add_dkg_node(&mut self, node: DKGCertifiedNode) -> anyhow::Result<()> {
        let mut dag_writer = self.dag.write();
        let round = node.metadata().round();
        if dag_writer.all_exists(node.parents()) {
            dag_writer.add_node(node)?;
            if self.current_round == round {
                let maybe_strong_links = dag_writer
                    .get_strong_links_for_round(self.current_round, &self.epoch_state.verifier);
                drop(dag_writer);
                if let Some(strong_links) = maybe_strong_links {
                    self.enter_new_round(strong_links);
                }
            }
        }
        // TODO: handle fetching missing dependencies
        Ok(())
    }

    async fn broadcast_pvss(&self) {
        // dkg todo: reliably broadcast pvss transcript, need to ensure all validators receive it
        // waiting for the reliable broadcast implementation on main
        let validators = self.old_validators.get_ordered_account_addresses();
        let transcript_bytes = serde_json::to_vec(&self.my_pvss.clone().unwrap()).unwrap();
        let message = DKGMsg(transcript_bytes);
        let (tx, rx) = oneshot::channel();
        let (_cancel_tx, cancel_rx) = oneshot::channel();
        tokio::spawn(self.dkg_rbc.broadcast::<DKGBroadcastStatus>(message, tx, cancel_rx));
        assert_eq!(rx.await.unwrap(), validators.into_iter().collect());
    }

    fn aggregate_pvss(&self) -> Option<Transcript> {
        // dkg todo: aggregate all pvss transcripts
        thread::sleep(Duration::from_millis(TRANSCRIPT_AGGREGATE_TIME_MS));
        None
    }

    pub async fn start(
        mut self,
        mut rx: tokio::sync::mpsc::Receiver<DKGManagerCommand>,
    ) {
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    match msg {
                        // dkg todo: triggering PVSS computation from block prolouge
                        DKGManagerCommand::StartDKG(stake_dis) => {
                            if self.my_pvss.is_some() {
                                // If we already have a PVSS transcript for this epoch, ignore
                                continue;
                            }
                            // dkg todo: start PVSS generation, once done reliably multicast to all validators
                            if self.compute_pvss(stake_dis).is_ok() {
                                self.all_pvss.insert(self.author, self.my_pvss.clone().unwrap());
                                self.broadcast_pvss().await;
                            }
                        }
                        DKGManagerCommand::ReceivePVSS(peer, transcript) => {
                            // dkg todo: verify if the PVSS transcript is valid
                            if !self.all_pvss.contains_key(&peer) && transcript.verify(TRANSCRIPT_VERIFY_TIME_MS).is_ok() {
                                self.all_pvss.insert(peer, transcript);
                                if self.old_validators.check_voting_power(self.all_pvss.keys()).is_ok() {
                                    // dkg todo: aggregate PVSS transcripts from other validators
                                    if let Some(aggregated_pvss) = self.aggregate_pvss() {
                                        // dkg todo: generate a new transaction for the aggregated pvss transcript
                                        // dkg todo: send aggregated PVSS transcript to batch generator
                                        self.batch_generator_cmd_tx.send(BatchGeneratorCommand::SendPVSSBatch(None)).await.unwrap();
                                    }
                                }
                            }
                        }
                        DKGManagerCommand::Shutdown(ack_tx) => {
                            ack_tx.send(()).expect("Failed to send shutdown ack to round manager");
                            break;
                        }
                    }
                }
            }
        }
        info!("DKGManager of epoch {} stopped", self.epoch);
    }
}
