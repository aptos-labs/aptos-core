// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunky::types::{AggregatedSubtranscript, ChunkyDKGTranscriptRequest},
    counters,
    types::DKGMessage,
};
use anyhow::{anyhow, ensure, Context};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_dkg::pvss::{
    traits::{
        transcript::{HasAggregatableSubtranscript, Transcript},
        Aggregatable,
    },
    Player,
};
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::info;
use aptos_reliable_broadcast::{BroadcastStatus, ReliableBroadcast};
use aptos_types::{
    dkg::{
        chunky_dkg::{
            ChunkyDKGConfig, ChunkyDKGTranscript, ChunkySubtranscript, ChunkyTranscript,
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
/// extracted from valid transcripts and aggregated. When a quorum is aggragated,
/// the [ChunkyDKGManager] is notified via the channel.
#[allow(dead_code)]
pub fn start_subtranscript_aggregation(
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    epoch_state: Arc<EpochState>,
    my_addr: AccountAddress,
    dkg_config: ChunkyDKGConfig,
    spks: Vec<DealerPublicKey>,
    start_time: Duration,
    agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscript>>,
    received_transcripts: Arc<Mutex<HashMap<AccountAddress, ChunkyTranscript>>>,
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

struct InnerState {
    valid_peer_transcript_seen: bool,
    contributors: HashSet<AccountAddress>,
    subtrx: Option<ChunkySubtranscript>,
    agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscript>>,
}

impl InnerState {
    fn new(agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscript>>) -> Self {
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
    dkg_config: ChunkyDKGConfig,
    signing_pubkeys: Vec<DealerPublicKey>,
    start_time: Duration,
    received_transcripts: Arc<Mutex<HashMap<AccountAddress, ChunkyTranscript>>>,
    inner_state: RwLock<InnerState>,
}

impl ChunkyTranscriptAggregationState {
    pub fn new(
        epoch_state: Arc<EpochState>,
        my_addr: AccountAddress,
        dkg_config: ChunkyDKGConfig,
        signing_pubkeys: Vec<DealerPublicKey>,
        start_time: Duration,
        agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscript>>,
        received_transcripts: Arc<Mutex<HashMap<AccountAddress, ChunkyTranscript>>>,
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

    /// Validates metadata and deserializes the transcript, and verifies it.
    fn validate_and_deserialize_transcript(
        &self,
        sender: Author,
        metadata: &DKGTranscriptMetadata,
        transcript_bytes: &[u8],
    ) -> anyhow::Result<(ChunkyTranscript, u64)> {
        // Validate metadata
        ensure!(
            metadata.epoch == self.dkg_config.session_metadata.dealer_epoch,
            "[ChunkyDKG] adding peer chunky transcript failed with invalid node epoch",
        );
        ensure!(
            metadata.author == sender,
            "[ChunkyDKG] adding peer chunky transcript failed with node author mismatch"
        );

        let peer_power = self.epoch_state.verifier.get_voting_power(&sender);
        ensure!(
            peer_power.is_some(),
            "[ChunkyDKG] adding peer chunky transcript failed with illegal dealer"
        );
        let peer_power = peer_power.expect("Peer must be valid");
        // Deserialize transcript
        let transcript: ChunkyTranscript = bcs::from_bytes(transcript_bytes)
            .map_err(|e| anyhow!("[ChunkyDKG] Unable to deserialize chunky transcript: {e}"))?;

        // Verify the transcript
        transcript
            .verify(
                &self.dkg_config.threshold_config,
                &self.dkg_config.public_parameters,
                &self.signing_pubkeys,
                &self.dkg_config.eks,
                &self.dkg_config.session_metadata,
            )
            .context("chunky transcript verification failed")?;

        // Ensure the transcript's dealer id matches the sender's validator index.
        // Otherwise a malicious validator could replay another validator's legitimately-signed
        // transcript, causing attribution mismatch between the aggregated subtranscript content
        // and the dealers list built from contributors.
        let sender_index = self
            .epoch_state
            .verifier
            .address_to_validator_index()
            .get(&sender)
            .copied()
            .ok_or_else(|| anyhow!("[ChunkyDKG] sender not in validator set"))?;
        let dealers = transcript.get_dealers();
        ensure!(
            dealers.len() == 1,
            "[ChunkyDKG] adding peer chunky transcript failed: expected single dealer, got {}",
            dealers.len(),
        );
        ensure!(
            dealers[0].id == sender_index,
            "[ChunkyDKG] adding peer chunky transcript failed: transcript dealer id {} does not match sender validator index {}",
            dealers[0].id,
            sender_index,
        );

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

        // RwLock allows concurrent validation of multiple transcripts
        let (transcript, peer_power) =
            self.validate_and_deserialize_transcript(sender, metadata, transcript_bytes)?;

        let mut inner_state = self.inner_state.write();
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

        // Store the transcript
        {
            let mut received_transcripts = self.received_transcripts.lock();
            received_transcripts.insert(metadata.author, transcript.clone());
        }

        // Aggregate the transcript
        // TODO(ibalajiarun): Should the transcript be aggregated if quorum is already met?
        inner_state.contributors.insert(metadata.author);
        if let Some(agg_subtrx) = inner_state.subtrx.as_mut() {
            agg_subtrx
                .aggregate_with(
                    &self.dkg_config.threshold_config,
                    &transcript.get_subtranscript(),
                )
                .context("chunky transcript aggregation failed")?;
        } else {
            inner_state.subtrx = Some(transcript.get_subtranscript());
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
            if let Some(tx) = inner_state.agg_subtrx_tx.take() {
                let agg_trx = inner_state.subtrx.clone().unwrap();
                // Convert AccountAddress contributors to Player by getting their validator indices.
                // Sort by AccountAddress so dealers order is deterministic (HashSet iteration is
                // non-deterministic); AggregatedSubtranscript is BCSCryptoHash'd for certification.
                let mut contributors: Vec<_> = inner_state.contributors.iter().copied().collect();
                contributors.sort();
                let dealers: Vec<Player> = contributors
                    .into_iter()
                    .map(|addr| {
                        self.epoch_state
                            .verifier
                            .address_to_validator_index()
                            .get(&addr)
                            .map(|&index| Player { id: index })
                            .expect("Request must be sent to validators in current set only")
                    })
                    .collect();
                let agg_subtrx = AggregatedSubtranscript {
                    subtranscript: agg_trx,
                    dealers,
                };
                if let Err(e) = tx.push((), agg_subtrx) {
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
    use aptos_bounded_executor::BoundedExecutor;
    use aptos_crypto::{
        bls12381::{PrivateKey, PublicKey},
        Uniform,
    };
    use aptos_infallible::duration_since_epoch;
    use aptos_reliable_broadcast::RBNetworkSender;
    use aptos_time_service::TimeService;
    use aptos_types::{
        dkg::chunky_dkg::{ChunkyDKG, ChunkyDKGSessionMetadata},
        on_chain_config::OnChainChunkyDKGConfig,
        validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct},
    };
    use async_trait::async_trait;
    use bytes::Bytes;
    use std::collections::HashMap;
    use tokio::runtime::Handle;

    struct DummyNetworkSender;

    #[async_trait]
    impl RBNetworkSender<DKGMessage> for DummyNetworkSender {
        async fn send_rb_rpc_raw(
            &self,
            _receiver: AccountAddress,
            _raw_message: Bytes,
            _timeout: Duration,
        ) -> anyhow::Result<DKGMessage> {
            // Dummy implementation - return error to prevent actual network calls
            anyhow::bail!("dummy sender")
        }

        async fn send_rb_rpc(
            &self,
            author: AccountAddress,
            _message: DKGMessage,
            timeout: Duration,
        ) -> anyhow::Result<DKGMessage> {
            self.send_rb_rpc_raw(author, Bytes::new(), timeout).await
        }

        fn to_bytes_by_protocol(
            &self,
            _peers: Vec<AccountAddress>,
            _message: DKGMessage,
        ) -> anyhow::Result<HashMap<AccountAddress, Bytes>> {
            Ok(HashMap::new())
        }

        fn sort_peers_by_latency(&self, _: &mut [AccountAddress]) {}
    }

    #[tokio::test]
    async fn test_start_chunky_transcript_aggregation() {
        // Setup minimal test data
        let epoch = 999;
        let my_addr = AccountAddress::random();
        let private_key = PrivateKey::generate_for_testing();
        let public_key = PublicKey::from(&private_key);
        let voting_power = 1u64;

        let validator_info = ValidatorConsensusInfo::new(my_addr, public_key.clone(), voting_power);
        let validator_info_move = ValidatorConsensusInfoMoveStruct::from(validator_info.clone());
        let verifier =
            aptos_types::validator_verifier::ValidatorVerifier::new(vec![validator_info]);
        let epoch_state = Arc::new(EpochState::new(epoch, verifier));

        let session_metadata = ChunkyDKGSessionMetadata {
            dealer_epoch: epoch,
            chunky_dkg_config: OnChainChunkyDKGConfig::default_enabled().into(),
            dealer_validator_set: vec![validator_info_move.clone()],
            target_validator_set: vec![validator_info_move],
        };
        let dkg_config = ChunkyDKG::generate_config(&session_metadata);

        let reliable_broadcast = Arc::new(ReliableBroadcast::new(
            my_addr,
            vec![my_addr],
            Arc::new(DummyNetworkSender),
            ExponentialBackoff::from_millis(10),
            TimeService::real(),
            Duration::from_millis(500),
            BoundedExecutor::new(2, Handle::current()),
        ));

        let start_time = duration_since_epoch();
        let received_transcripts = Arc::new(Mutex::new(HashMap::new()));

        // Test that the function returns an AbortHandle without panicking
        let abort_handle = start_subtranscript_aggregation(
            reliable_broadcast,
            epoch_state,
            my_addr,
            dkg_config,
            vec![public_key],
            start_time,
            None,
            received_transcripts,
        );

        // Verify it returns an AbortHandle
        assert!(!abort_handle.is_aborted());

        // TODO(ibalajiarun): Complete this test

        // Clean up
        abort_handle.abort();
    }
}
