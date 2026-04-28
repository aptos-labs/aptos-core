// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunky::{
        common::deserialize_chunky_transcript_and_verify,
        types::{
            AggregatedSubtranscriptWithHashes, ChunkyDKGTranscriptRequest, ChunkyTranscriptWithHash,
        },
    },
    counters,
    types::DKGMessage,
};
use anyhow::{anyhow, ensure, Context};
use aptos_bitvec::BitVec;
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_crypto::HashValue;
use aptos_dkg::pvss::traits::transcript::{Aggregatable, Aggregated};
use aptos_infallible::RwLock;
use aptos_logger::info;
use aptos_reliable_broadcast::{BroadcastStatus, ReliableBroadcast};
use aptos_types::{
    dkg::{
        chunky_dkg::{
            AggregatedSubtranscript, ChunkyDKGSession, ChunkyDKGTranscript, ChunkySubtranscript,
            DealerPublicKey,
        },
        DKGTranscriptMetadata,
    },
    epoch_state::EpochState,
    validator_verifier::VerifyError,
};
use futures::future::AbortHandle;
use futures_util::future::Abortable;
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio_retry::strategy::ExponentialBackoff;

/// Starts a task to collect transcripts from all validators. The subtranscripts are
/// extracted from valid transcripts and aggregated. When a quorum is aggregated,
/// the [ChunkyDKGManager] is notified via the channel.
pub fn start_subtranscript_aggregation(
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    epoch_state: Arc<EpochState>,
    my_addr: AccountAddress,
    dkg_config: Arc<ChunkyDKGSession>,
    spks: Vec<DealerPublicKey>,
    start_time: Duration,
    agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscriptWithHashes>>,
    received_transcripts: Arc<RwLock<HashMap<AccountAddress, ChunkyTranscriptWithHash>>>,
) -> AbortHandle {
    let epoch = dkg_config.session_metadata.dealer_epoch;
    let req = ChunkyDKGTranscriptRequest::new(epoch);

    let agg_state = Arc::new(ChunkyTranscriptAggregationState::new(
        epoch_state,
        my_addr,
        dkg_config,
        spks,
        start_time,
        agg_subtrx_tx,
        received_transcripts,
    ));
    let task = async move {
        reliable_broadcast
            .broadcast(req, agg_state)
            .await
            .expect("broadcast cannot fail");
        info!(
            epoch = epoch,
            my_addr = my_addr,
            "[ChunkyDKG] aggregated chunky transcript from all validators"
        );
    };
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    tokio::spawn(Abortable::new(task, abort_registration));
    abort_handle
}

/// Projective accumulator type for ChunkySubtranscript (aggregate_with in projective form, normalize when quorum met).
type ChunkySubtranscriptProjective = <ChunkySubtranscript as Aggregatable>::Aggregated;

struct InnerState {
    valid_peer_transcript_seen: bool,
    contributors: HashSet<AccountAddress>,
    /// Accumulator in projective form; use aggregate_with for each transcript, then normalize when quorum is met.
    subtrx: Option<ChunkySubtranscriptProjective>,
    agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscriptWithHashes>>,
}

impl InnerState {
    fn new(
        agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscriptWithHashes>>,
    ) -> Self {
        Self {
            valid_peer_transcript_seen: false,
            contributors: HashSet::new(),
            subtrx: None,
            agg_subtrx_tx,
        }
    }
}

pub struct ChunkyTranscriptAggregationState {
    epoch_state: Arc<EpochState>,
    my_addr: AccountAddress,
    dkg_config: Arc<ChunkyDKGSession>,
    signing_pubkeys: Vec<DealerPublicKey>,
    start_time: Duration,
    received_transcripts: Arc<RwLock<HashMap<AccountAddress, ChunkyTranscriptWithHash>>>,
    inner_state: RwLock<InnerState>,
}

impl ChunkyTranscriptAggregationState {
    pub fn new(
        epoch_state: Arc<EpochState>,
        my_addr: AccountAddress,
        dkg_config: Arc<ChunkyDKGSession>,
        signing_pubkeys: Vec<DealerPublicKey>,
        start_time: Duration,
        agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscriptWithHashes>>,
        received_transcripts: Arc<RwLock<HashMap<AccountAddress, ChunkyTranscriptWithHash>>>,
    ) -> Self {
        Self {
            epoch_state,
            my_addr,
            dkg_config,
            signing_pubkeys,
            start_time,
            received_transcripts,
            inner_state: RwLock::new(InnerState::new(agg_subtrx_tx)),
        }
    }

    /// Validates metadata, deserializes the transcript, verifies it, and checks dealer ID.
    fn validate_and_deserialize_transcript(
        &self,
        sender: Author,
        metadata: &DKGTranscriptMetadata,
        transcript_bytes: &[u8],
    ) -> anyhow::Result<(ChunkyTranscriptWithHash, u64)> {
        // Validate metadata (epoch, author, voting power) — specific to the aggregation context.
        ensure!(
            metadata.epoch == self.dkg_config.session_metadata.dealer_epoch,
            "[ChunkyDKG] adding peer chunky transcript failed with invalid node epoch",
        );
        ensure!(
            metadata.author == sender,
            "[ChunkyDKG] adding peer chunky transcript failed with node author mismatch"
        );

        let peer_power = self
            .epoch_state
            .verifier
            .get_voting_power(&sender)
            .ok_or_else(|| {
                anyhow!("[ChunkyDKG] adding peer chunky transcript failed with illegal dealer")
            })?;

        // Shared validation: deserialize, verify, check dealer ID.
        let transcript = deserialize_chunky_transcript_and_verify(
            sender,
            transcript_bytes,
            &self.dkg_config,
            &self.signing_pubkeys,
            &self.epoch_state,
        )?;

        Ok((transcript, peer_power))
    }
}

impl BroadcastStatus<DKGMessage> for Arc<ChunkyTranscriptAggregationState> {
    type Aggregated = ();
    type Message = ChunkyDKGTranscriptRequest;
    type Response = ChunkyDKGTranscript;

    fn add(
        &self,
        sender: Author,
        chunky_dkg_transcript: ChunkyDKGTranscript,
    ) -> anyhow::Result<Option<Self::Aggregated>> {
        let epoch = self.epoch_state.epoch;

        let ChunkyDKGTranscript {
            metadata,
            transcript_bytes,
        } = &chunky_dkg_transcript;

        {
            let inner_state = self.inner_state.read();
            if inner_state.contributors.contains(&sender) {
                return Ok(None);
            }
        }

        let (transcript, peer_power) =
            self.validate_and_deserialize_transcript(sender, metadata, transcript_bytes)?;

        let mut inner_state = self.inner_state.write();
        // Re-check under write lock to prevent TOCTOU race (concurrent adds may have
        // already inserted this sender between the read-lock check and the write-lock acquire).
        if inner_state.contributors.contains(&sender) {
            return Ok(None);
        }
        // Track first peer transcript for metrics
        let is_self = self.my_addr == sender;
        if !is_self && !inner_state.valid_peer_transcript_seen {
            inner_state.valid_peer_transcript_seen = true;
            counters::observe_chunky_dkg_stage(
                self.start_time,
                self.my_addr,
                "first_valid_peer_transcript",
            );
        }

        // Store the transcript before aggregation so that `received_transcripts` and
        // `contributors` stay consistent even if `aggregate_with` fails via `?`.
        // The quorum path below reads `received_transcripts` and expects every contributor
        // to have a stored transcript.
        let subtranscript = transcript.get_subtranscript();
        {
            let mut received_transcripts = self.received_transcripts.write();
            received_transcripts.insert(metadata.author, transcript);
        }

        inner_state.contributors.insert(metadata.author);

        // Quorum already reached — transcript is stored for the fetcher but no
        // further aggregation is needed.
        if inner_state.agg_subtrx_tx.is_none() {
            let all_received = inner_state.contributors.len() >= self.epoch_state.verifier.len();
            return Ok(all_received.then_some(()));
        }
        if let Some(agg_subtrx) = inner_state.subtrx.as_mut() {
            agg_subtrx
                .aggregate_with(&self.dkg_config.threshold_config, &subtranscript)
                .context("chunky transcript aggregation failed")?;
        } else {
            inner_state.subtrx = Some(subtranscript.to_aggregated());
        }

        // Check quorum and send if needed
        let threshold = self.epoch_state.verifier.quorum_voting_power();
        let power_check_result = self
            .epoch_state
            .verifier
            .check_voting_power(inner_state.contributors.iter(), true);
        let new_total_power = match &power_check_result {
            Ok(x) => Some(*x),
            Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => Some(*voting_power),
            _ => None,
        };
        let quorum_met = power_check_result.is_ok();

        // Send to agg_subtrx_tx when quorum is met (only once)
        if quorum_met {
            let tx = inner_state
                .agg_subtrx_tx
                .take()
                .expect("agg_subtrx_tx must be Some due to early return above");
            let agg_trx = inner_state.subtrx.take().unwrap().normalize();
            let num_validators = self.epoch_state.verifier.len();
            let addr_to_index = self.epoch_state.verifier.address_to_validator_index();
            let received = self.received_transcripts.read();

            let mut dealer_bitmask = BitVec::with_num_bits(num_validators as u16);
            let mut indexed_hashes: Vec<(usize, HashValue)> = Vec::new();
            for addr in inner_state.contributors.iter() {
                let index = *addr_to_index
                    .get(addr)
                    .ok_or_else(|| anyhow!("contributor {} not in validator set", addr))?;
                dealer_bitmask.set(index as u16);
                let hash = received
                    .get(addr)
                    .ok_or_else(|| anyhow!("contributor {} missing stored transcript", addr))?
                    .hash();
                indexed_hashes.push((index, hash));
            }
            indexed_hashes.sort_by_key(|(idx, _)| *idx);
            let dealer_transcript_hashes: Vec<HashValue> =
                indexed_hashes.into_iter().map(|(_, h)| h).collect();
            drop(received);

            let with_hashes = AggregatedSubtranscriptWithHashes {
                aggregated_subtranscript: AggregatedSubtranscript {
                    dealer_epoch: self.dkg_config.session_metadata.dealer_epoch,
                    subtranscript: agg_trx,
                    dealer_bitmask,
                },
                dealer_transcript_hashes,
            };
            if let Err(e) = tx.push((), with_hashes) {
                info!(
                    epoch = epoch,
                    "[ChunkyDKG] Failed to send aggregated chunky transcript to ChunkyDKGManager when quorum met: {:?}", e
                );
            } else {
                info!(
                    epoch = epoch,
                    "[ChunkyDKG] sent aggregated chunky transcript to ChunkyDKGManager (quorum met)"
                );
            }
        }

        // Check if all validators have contributed
        let total_validators = self.epoch_state.verifier.len();
        let contributors_count = inner_state.contributors.len();
        let all_received = contributors_count >= total_validators;

        info!(
            epoch = epoch,
            peer = sender,
            is_self = is_self,
            peer_power = peer_power,
            new_total_power = new_total_power,
            threshold = threshold,
            quorum_met = quorum_met,
            contributors_count = contributors_count,
            total_validators = total_validators,
            all_received = all_received,
            "[ChunkyDKG] added chunky transcript from validator {}, {} out of {} aggregated ({} total validators).",
            self.epoch_state
                .verifier
                .address_to_validator_index()
                .get(&sender)
                .unwrap(),
            new_total_power.unwrap_or(0),
            threshold,
            total_validators
        );

        Ok(all_received.then_some(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunky::test_utils::ChunkyTestSetup;
    use aptos_infallible::duration_since_epoch;
    use futures_util::{FutureExt, StreamExt};

    fn make_agg_state(
        setup: &ChunkyTestSetup,
        validator_index: usize,
    ) -> (
        Arc<ChunkyTranscriptAggregationState>,
        aptos_channel::Receiver<(), AggregatedSubtranscriptWithHashes>,
    ) {
        let (tx, rx) =
            aptos_channel::new(aptos_channels::message_queues::QueueStyle::KLAST, 1, None);
        let state = Arc::new(ChunkyTranscriptAggregationState::new(
            setup.epoch_state.clone(),
            setup.addrs[validator_index],
            setup.dkg_config.clone(),
            setup.spks(),
            duration_since_epoch(),
            Some(tx),
            Arc::new(RwLock::new(HashMap::<
                AccountAddress,
                ChunkyTranscriptWithHash,
            >::new())),
        ));
        (state, rx)
    }

    #[tokio::test]
    async fn test_aggregation_happy_path() {
        let setup = ChunkyTestSetup::new_uniform(4);
        let (state, mut rx) = make_agg_state(&setup, 0);

        // Add transcripts from validators 0, 1, 2 — first two below quorum.
        let (trx0, _) = setup.deal_transcript(0);
        let result = BroadcastStatus::add(&state, setup.addrs[0], trx0);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // no quorum yet

        let (trx1, _) = setup.deal_transcript(1);
        let result = BroadcastStatus::add(&state, setup.addrs[1], trx1);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // still no quorum

        // Third transcript triggers quorum (3 of 4 uniform = 2f+1).
        let (trx2, _) = setup.deal_transcript(2);
        let result = BroadcastStatus::add(&state, setup.addrs[2], trx2);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // not all received yet

        // Verify the aggregated subtranscript was sent via the channel.
        let agg = rx.select_next_some().now_or_never();
        assert!(agg.is_some());
        let agg_with_hashes = agg.unwrap();
        assert_eq!(
            agg_with_hashes
                .aggregated_subtranscript
                .dealer_bitmask
                .count_ones(),
            3
        );
        assert_eq!(agg_with_hashes.dealer_transcript_hashes.len(), 3);

        // Fourth transcript — all received, returns Some(()).
        let (trx3, _) = setup.deal_transcript(3);
        let result = BroadcastStatus::add(&state, setup.addrs[3], trx3);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some()); // all received
    }

    #[tokio::test]
    async fn test_aggregation_rejects_invalid_transcripts() {
        let setup = ChunkyTestSetup::new_uniform(4);
        let (state, _rx) = make_agg_state(&setup, 0);

        // Wrong epoch.
        let wrong_epoch_trx = ChunkyDKGTranscript::new(
            1, // wrong epoch
            setup.addrs[0],
            vec![],
        );
        let result = BroadcastStatus::add(&state, setup.addrs[0], wrong_epoch_trx);
        assert!(result.is_err());

        // Author mismatch — transcript says validator 0, but sent by validator 1.
        let (trx0, _) = setup.deal_transcript(0);
        let result = BroadcastStatus::add(&state, setup.addrs[1], trx0);
        assert!(result.is_err());

        // Unknown sender.
        let unknown_addr = AccountAddress::random();
        let (trx1, _) = setup.deal_transcript(1);
        let mut trx_unknown = trx1;
        trx_unknown.metadata.author = unknown_addr;
        let result = BroadcastStatus::add(&state, unknown_addr, trx_unknown);
        assert!(result.is_err());

        // Dealer ID mismatch — deal as validator 0 but change metadata to validator 1.
        let (mut trx_dealer_mismatch, _) = setup.deal_transcript(0);
        trx_dealer_mismatch.metadata.author = setup.addrs[1];
        let result = BroadcastStatus::add(&state, setup.addrs[1], trx_dealer_mismatch);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_aggregation_ignores_duplicate_sender() {
        let setup = ChunkyTestSetup::new_uniform(4);
        let (state, _rx) = make_agg_state(&setup, 0);

        let (trx0, _) = setup.deal_transcript(0);
        let result = BroadcastStatus::add(&state, setup.addrs[0], trx0.clone());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Same sender again — should be silently ignored.
        let result = BroadcastStatus::add(&state, setup.addrs[0], trx0);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_aggregation_unequal_voting_power() {
        // Validator 3 has power 7, total = 10, quorum = 7.
        let setup = ChunkyTestSetup::new(4, vec![1, 1, 1, 7]);
        let (state, mut rx) = make_agg_state(&setup, 0);

        // Single add from the high-power validator should trigger quorum.
        let (trx3, _) = setup.deal_transcript(3);
        let result = BroadcastStatus::add(&state, setup.addrs[3], trx3);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // not all received

        // Channel should have received the aggregated subtranscript.
        let agg = rx.select_next_some().now_or_never();
        assert!(agg.is_some());
        let agg_with_hashes = agg.unwrap();
        assert_eq!(
            agg_with_hashes
                .aggregated_subtranscript
                .dealer_bitmask
                .count_ones(),
            1
        );
        assert_eq!(agg_with_hashes.dealer_transcript_hashes.len(), 1);
    }
}
