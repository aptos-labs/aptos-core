// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    logging::{LogEvent, LogSchema},
    monitor,
    network::QuorumStoreSender,
    quorum_store::{
        batch_generator::BatchGeneratorCommand,
        batch_store::BatchReader,
        counters,
        tracing::{observe_batch, observe_batch_vote_pct, BatchStage},
        utils::Timeouts,
    },
};
use velor_consensus_types::{
    payload::TDataInfo,
    proof_of_store::{
        BatchInfo, ProofCache, ProofOfStore, SignedBatchInfo, SignedBatchInfoError,
        SignedBatchInfoMsg,
    },
};
use velor_logger::prelude::*;
use velor_short_hex_str::AsShortHexStr;
use velor_types::{
    ledger_info::SignatureAggregator, validator_verifier::ValidatorVerifier, PeerId,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{mpsc::Receiver, oneshot as TokioOneshot},
    time,
};

#[derive(Debug)]
pub(crate) enum ProofCoordinatorCommand {
    AppendSignature(PeerId, SignedBatchInfoMsg),
    CommitNotification(Vec<BatchInfo>),
    Shutdown(TokioOneshot::Sender<()>),
}

struct IncrementalProofState {
    signature_aggregator: SignatureAggregator<BatchInfo>,
    aggregated_voting_power: u128,
    self_voted: bool,
    completed: bool,
    // Pct last time the diff was over 10%
    last_increment_pct: u8,
}

impl IncrementalProofState {
    fn new(info: BatchInfo) -> Self {
        Self {
            signature_aggregator: SignatureAggregator::new(info),
            aggregated_voting_power: 0,
            self_voted: false,
            completed: false,
            last_increment_pct: 0,
        }
    }

    pub fn voter_count(&self) -> u64 {
        self.signature_aggregator.all_voters().count() as u64
    }

    // Returns the aggregated voting power of all signatures include those that are invalid.
    #[allow(unused)]
    pub fn aggregate_voting_power(&self, verifier: &ValidatorVerifier) -> u64 {
        self.signature_aggregator
            .check_voting_power(verifier, true)
            .unwrap_or(0) as u64
    }

    fn add_signature(
        &mut self,
        signed_batch_info: &SignedBatchInfo,
        validator_verifier: &ValidatorVerifier,
    ) -> Result<(), SignedBatchInfoError> {
        if signed_batch_info.batch_info() != self.signature_aggregator.data() {
            return Err(SignedBatchInfoError::WrongInfo((
                signed_batch_info.batch_id().id,
                self.signature_aggregator.data().batch_id().id,
            )));
        }

        match validator_verifier.get_voting_power(&signed_batch_info.signer()) {
            Some(voting_power) => {
                self.signature_aggregator.add_signature(
                    signed_batch_info.signer(),
                    signed_batch_info.signature_with_status(),
                );
                self.aggregated_voting_power += voting_power as u128;
                if signed_batch_info.signer() == self.signature_aggregator.data().author() {
                    self.self_voted = true;
                }
            },
            None => {
                error!(
                    "Received signature from author not in validator set: {}",
                    signed_batch_info.signer()
                );
                return Err(SignedBatchInfoError::InvalidAuthor);
            },
        }

        Ok(())
    }

    fn check_voting_power(
        &self,
        validator_verifier: &ValidatorVerifier,
        check_super_majority: bool,
    ) -> bool {
        self.signature_aggregator
            .check_voting_power(validator_verifier, check_super_majority)
            .is_ok()
    }

    /// Observes the voting percentage if it's 10% higher than last observation i.e. it
    /// approximately observes every 10% increase in voting power.
    fn observe_voting_pct(&mut self, timestamp: u64, validator_verifier: &ValidatorVerifier) {
        let pct = self
            .aggregated_voting_power
            .saturating_mul(100)
            .saturating_div(validator_verifier.total_voting_power()) as u8;
        let author = self.signature_aggregator.data().author();
        if pct >= self.last_increment_pct + 10 {
            observe_batch_vote_pct(timestamp, author, pct);
            self.last_increment_pct = pct;
        }
    }

    /// Try to aggregate all signatures if the voting power is enough. If the aggregated signature is
    /// valid, return the ProofOfStore.
    pub fn aggregate_and_verify(
        &mut self,
        validator_verifier: &ValidatorVerifier,
    ) -> Result<ProofOfStore, SignedBatchInfoError> {
        if self.completed {
            panic!("Cannot call take twice, unexpected issue occurred");
        }
        match self
            .signature_aggregator
            .aggregate_and_verify(validator_verifier)
        {
            Ok((batch_info, aggregated_sig)) => {
                self.completed = true;
                Ok(ProofOfStore::new(batch_info, aggregated_sig))
            },
            Err(_) => Err(SignedBatchInfoError::UnableToAggregate),
        }
    }

    pub fn batch_info(&self) -> &BatchInfo {
        self.signature_aggregator.data()
    }
}

pub(crate) struct ProofCoordinator {
    peer_id: PeerId,
    proof_timeout_ms: usize,
    batch_info_to_proof: HashMap<BatchInfo, IncrementalProofState>,
    // to record the batch creation time
    batch_info_to_time: HashMap<BatchInfo, Instant>,
    timeouts: Timeouts<BatchInfo>,
    batch_reader: Arc<dyn BatchReader>,
    batch_generator_cmd_tx: tokio::sync::mpsc::Sender<BatchGeneratorCommand>,
    proof_cache: ProofCache,
    broadcast_proofs: bool,
    batch_expiry_gap_when_init_usecs: u64,
}

//PoQS builder object - gather signed digest to form PoQS
impl ProofCoordinator {
    pub fn new(
        proof_timeout_ms: usize,
        peer_id: PeerId,
        batch_reader: Arc<dyn BatchReader>,
        batch_generator_cmd_tx: tokio::sync::mpsc::Sender<BatchGeneratorCommand>,
        proof_cache: ProofCache,
        broadcast_proofs: bool,
        batch_expiry_gap_when_init_usecs: u64,
    ) -> Self {
        Self {
            peer_id,
            proof_timeout_ms,
            batch_info_to_proof: HashMap::new(),
            batch_info_to_time: HashMap::new(),
            timeouts: Timeouts::new(),
            batch_reader,
            batch_generator_cmd_tx,
            proof_cache,
            broadcast_proofs,
            batch_expiry_gap_when_init_usecs,
        }
    }

    fn init_proof(
        &mut self,
        signed_batch_info: &SignedBatchInfo,
    ) -> Result<(), SignedBatchInfoError> {
        // Check if the signed digest corresponding to our batch
        if signed_batch_info.author() != self.peer_id {
            return Err(SignedBatchInfoError::WrongAuthor);
        }
        let batch_author = self
            .batch_reader
            .exists(signed_batch_info.digest())
            .ok_or(SignedBatchInfoError::NotFound)?;
        if batch_author != signed_batch_info.author() {
            return Err(SignedBatchInfoError::WrongAuthor);
        }

        self.timeouts.add(
            signed_batch_info.batch_info().clone(),
            self.proof_timeout_ms,
        );
        self.batch_info_to_proof.insert(
            signed_batch_info.batch_info().clone(),
            IncrementalProofState::new(signed_batch_info.batch_info().clone()),
        );
        self.batch_info_to_time
            .entry(signed_batch_info.batch_info().clone())
            .or_insert(Instant::now());
        debug!(
            LogSchema::new(LogEvent::ProofOfStoreInit),
            digest = signed_batch_info.digest(),
            batch_id = signed_batch_info.batch_id().id,
        );
        Ok(())
    }

    fn add_signature(
        &mut self,
        signed_batch_info: SignedBatchInfo,
        validator_verifier: &ValidatorVerifier,
    ) -> Result<Option<ProofOfStore>, SignedBatchInfoError> {
        if !self
            .batch_info_to_proof
            .contains_key(signed_batch_info.batch_info())
        {
            self.init_proof(&signed_batch_info)?;
        }
        if let Some(value) = self
            .batch_info_to_proof
            .get_mut(signed_batch_info.batch_info())
        {
            value.add_signature(&signed_batch_info, validator_verifier)?;
            if !value.completed && value.check_voting_power(validator_verifier, true) {
                let proof = {
                    let _timer = counters::SIGNED_BATCH_INFO_VERIFY_DURATION.start_timer();
                    value.aggregate_and_verify(validator_verifier)?
                };
                // proof validated locally, so adding to cache
                self.proof_cache
                    .insert(proof.info().clone(), proof.multi_signature().clone());
                // quorum store measurements
                let duration = self
                    .batch_info_to_time
                    .remove(signed_batch_info.batch_info())
                    .ok_or(
                        // Batch created without recording the time!
                        SignedBatchInfoError::NoTimeStamps,
                    )?
                    .elapsed();
                counters::BATCH_TO_POS_DURATION.observe_duration(duration);
                return Ok(Some(proof));
            }
        } else {
            return Err(SignedBatchInfoError::NotFound);
        }
        Ok(None)
    }

    fn update_counters_on_expire(state: &IncrementalProofState) {
        // Count late votes separately
        if !state.completed && !state.self_voted {
            counters::BATCH_RECEIVED_LATE_REPLIES_COUNT.inc_by(state.voter_count());
            return;
        }

        counters::BATCH_RECEIVED_REPLIES_COUNT.observe(state.voter_count() as f64);
        counters::BATCH_RECEIVED_REPLIES_VOTING_POWER.observe(state.aggregated_voting_power as f64);
        if !state.completed {
            counters::BATCH_SUCCESSFUL_CREATION.observe(0.0);
        }
    }

    async fn expire(&mut self) {
        let mut batch_ids = vec![];
        for signed_batch_info_info in self.timeouts.expire() {
            if let Some(state) = self.batch_info_to_proof.remove(&signed_batch_info_info) {
                if !state.completed {
                    batch_ids.push(signed_batch_info_info.batch_id());
                }
                Self::update_counters_on_expire(&state);

                // We skip metrics if the proof did not complete and did not get a self vote, as it
                // is considered a proof that was re-inited due to a very late vote.
                if !state.completed && !state.self_voted {
                    continue;
                }

                if !state.completed {
                    counters::TIMEOUT_BATCHES_COUNT.inc();
                    info!(
                        LogSchema::new(LogEvent::IncrementalProofExpired),
                        digest = signed_batch_info_info.digest(),
                        self_voted = state.self_voted,
                    );
                }
            }
        }
        if self
            .batch_generator_cmd_tx
            .send(BatchGeneratorCommand::ProofExpiration(batch_ids))
            .await
            .is_err()
        {
            warn!("Failed to send proof expiration to batch generator");
        }
    }

    pub async fn start(
        mut self,
        mut rx: Receiver<ProofCoordinatorCommand>,
        mut network_sender: impl QuorumStoreSender,
        validator_verifier: Arc<ValidatorVerifier>,
    ) {
        let mut interval = time::interval(Duration::from_millis(100));
        loop {
            tokio::select! {
                Some(command) = rx.recv() => monitor!("proof_coordinator_handle_command", {
                    match command {
                        ProofCoordinatorCommand::Shutdown(ack_tx) => {
                            counters::QUORUM_STORE_MSG_COUNT.with_label_values(&["ProofCoordinator::shutdown"]).inc();
                            ack_tx
                                .send(())
                                .expect("Failed to send shutdown ack to QuorumStore");
                            break;
                        },
                        ProofCoordinatorCommand::CommitNotification(batches) => {
                            counters::QUORUM_STORE_MSG_COUNT.with_label_values(&["ProofCoordinator::commit_notification"]).inc();
                            for batch in batches {
                                let digest = batch.digest();
                                if let Entry::Occupied(existing_proof) = self.batch_info_to_proof.entry(batch.clone()) {
                                    if batch == *existing_proof.get().batch_info() {
                                        let incremental_proof = existing_proof.get();
                                        if incremental_proof.completed {
                                            counters::BATCH_SUCCESSFUL_CREATION.observe(1.0);
                                        } else {
                                            info!("QS: received commit notification for batch that did not complete: {}, self_voted: {}", digest, incremental_proof.self_voted);
                                        }
                                        debug!(
                                            LogSchema::new(LogEvent::ProofOfStoreCommit),
                                            digest = digest,
                                            batch_id = batch.batch_id().id,
                                            proof_completed = incremental_proof.completed,
                                        );
                                    }
                                }
                            }
                        },
                        ProofCoordinatorCommand::AppendSignature(signer, signed_batch_infos) => {
                            let signed_batch_infos = signed_batch_infos.take();
                            let Some(signed_batch_info) = signed_batch_infos.first() else {
                                error!("Empty signed batch info received from {}", signer.short_str().as_str());
                                return;
                            };
                            let info = signed_batch_info.info().clone();
                            let approx_created_ts_usecs = signed_batch_info
                                .expiration()
                                .saturating_sub(self.batch_expiry_gap_when_init_usecs);

                            let mut proofs = vec![];
                            for signed_batch_info in signed_batch_infos.into_iter() {
                                let peer_id = signed_batch_info.signer();
                                let digest = *signed_batch_info.digest();
                                let batch_id = signed_batch_info.batch_id();
                                match self.add_signature(signed_batch_info, &validator_verifier) {
                                    Ok(result) => {
                                        if let Some(proof) = result {
                                            debug!(
                                                LogSchema::new(LogEvent::ProofOfStoreReady),
                                                digest = digest,
                                                batch_id = batch_id.id,
                                            );
                                            proofs.push(proof);
                                        }
                                    },
                                    Err(e) => {
                                        // Can happen if we already garbage collected, the commit notification is late, or the peer is misbehaving.
                                        if peer_id == self.peer_id {
                                            info!("QS: could not add signature from self, digest = {}, batch_id = {}, err = {:?}", digest, batch_id, e);
                                        } else {
                                            debug!("QS: could not add signature from peer {}, digest = {}, batch_id = {}, err = {:?}", peer_id, digest, batch_id, e);
                                        }
                                    },
                                }
                            }
                            if let Some(value) = self.batch_info_to_proof.get_mut(&info) {
                                value.observe_voting_pct(approx_created_ts_usecs, &validator_verifier);
                            }
                            if !proofs.is_empty() {
                                observe_batch(approx_created_ts_usecs, self.peer_id, BatchStage::POS_FORMED);
                                if self.broadcast_proofs {
                                    network_sender.broadcast_proof_of_store_msg(proofs).await;
                                } else {
                                    network_sender.send_proof_of_store_msg_to_self(proofs).await;
                                }
                            }
                        },
                    }
                }),
                _ = interval.tick() => {
                    monitor!("proof_coordinator_handle_tick", self.expire().await);
                }
            }
        }
    }
}
