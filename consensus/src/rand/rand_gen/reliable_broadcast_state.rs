// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    logging::{LogEvent, LogSchema},
    rand::rand_gen::{
        network_messages::RandMessage,
        rand_store::RandStore,
        types::{
            AugData, AugDataSignature, CertifiedAugData, CertifiedAugDataAck, RandConfig,
            RandShare, RequestCertifiedAugData, RequestShare, TAugmentedData, TShare,
        },
    },
};
use anyhow::ensure;
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    aggregate_signature::PartialSignatures, epoch_state::EpochState, randomness::RandMetadata,
};
use futures_channel::mpsc::UnboundedSender;
use std::{collections::HashSet, sync::Arc};

pub struct AugDataCertBuilder<D> {
    epoch_state: Arc<EpochState>,
    aug_data: AugData<D>,
    partial_signatures: Mutex<PartialSignatures>,
}

impl<D> AugDataCertBuilder<D> {
    pub fn new(aug_data: AugData<D>, epoch_state: Arc<EpochState>) -> Arc<Self> {
        Arc::new(Self {
            epoch_state,
            aug_data,
            partial_signatures: Mutex::new(PartialSignatures::empty()),
        })
    }
}

impl<S: TShare, D: TAugmentedData> BroadcastStatus<RandMessage<S, D>, RandMessage<S, D>>
    for Arc<AugDataCertBuilder<D>>
{
    type Aggregated = CertifiedAugData<D>;
    type Message = AugData<D>;
    type Response = AugDataSignature;

    fn add(&self, peer: Author, ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
        ack.verify(peer, &self.epoch_state.verifier, &self.aug_data)?;
        let mut parital_signatures_guard = self.partial_signatures.lock();
        parital_signatures_guard.add_signature(peer, ack.into_signature());
        let qc_aug_data = self
            .epoch_state
            .verifier
            .check_voting_power(parital_signatures_guard.signatures().keys(), true)
            .ok()
            .map(|_| {
                let aggregated_signature = self
                    .epoch_state
                    .verifier
                    .aggregate_signatures(parital_signatures_guard.signatures_iter())
                    .expect("Signature aggregation should succeed");
                CertifiedAugData::new(self.aug_data.clone(), aggregated_signature)
            });
        Ok(qc_aug_data)
    }
}

pub struct CertifiedAugDataAckState {
    validators: Mutex<HashSet<Author>>,
}

impl CertifiedAugDataAckState {
    pub fn new(validators: impl Iterator<Item = Author>) -> Self {
        Self {
            validators: Mutex::new(validators.collect()),
        }
    }
}

impl<S: TShare, D: TAugmentedData> BroadcastStatus<RandMessage<S, D>, RandMessage<S, D>>
    for Arc<CertifiedAugDataAckState>
{
    type Aggregated = ();
    type Message = CertifiedAugData<D>;
    type Response = CertifiedAugDataAck;

    fn add(&self, peer: Author, _ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
        let mut validators_guard = self.validators.lock();
        ensure!(
            validators_guard.remove(&peer),
            "[RandMessage] Unknown author: {}",
            peer
        );
        // If receive from all validators, stop the reliable broadcast
        if validators_guard.is_empty() {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

pub struct ShareAggregateState<S> {
    rand_metadata: RandMetadata,
    rand_store: Arc<Mutex<RandStore<S>>>,
    rand_config: RandConfig,
    // notifies the RandManager main loop about an author whose share we cannot verify yet, so it
    // can fetch that author's certified aug data
    missing_apk_tx: UnboundedSender<Author>,
}

impl<S> ShareAggregateState<S> {
    pub fn new(
        rand_store: Arc<Mutex<RandStore<S>>>,
        rand_metadata: RandMetadata,
        rand_config: RandConfig,
        missing_apk_tx: UnboundedSender<Author>,
    ) -> Self {
        Self {
            rand_store,
            rand_metadata,
            rand_config,
            missing_apk_tx,
        }
    }
}

impl<S: TShare, D: TAugmentedData> BroadcastStatus<RandMessage<S, D>, RandMessage<S, D>>
    for Arc<ShareAggregateState<S>>
{
    type Aggregated = ();
    type Message = RequestShare;
    type Response = RandShare<S>;

    fn add(&self, peer: Author, share: Self::Response) -> anyhow::Result<Option<()>> {
        ensure!(
            share.metadata() == &self.rand_metadata,
            "Metadata does not match: local {:?}, received {:?}",
            self.rand_metadata,
            share.metadata()
        );
        // Without the author's certified aug data this share is not verifiable. Storing it anyway
        // would make `pre_aggregate_verify` fail it and add the author to the pessimistic set for
        // the rest of the epoch, so request the cert and let the reliable broadcast retry the peer
        // once we have it.
        let has_certified_apk = self.rand_config.get_certified_apk(&peer).is_some();
        if !has_certified_apk {
            // Best effort: the main loop dedupes concurrent requests per author.
            let _ = self.missing_apk_tx.unbounded_send(peer);
        }
        ensure!(
            has_certified_apk,
            "[RandManager] missing certified aug data for {}, cannot verify its share yet",
            peer
        );
        share.optimistic_verify(&self.rand_config, &peer)?;
        info!(LogSchema::new(LogEvent::ReceiveReactiveRandShare)
            .epoch(share.epoch())
            .round(share.metadata().round)
            .remote_peer(*share.author()));
        let mut store = self.rand_store.lock();
        let aggregated = if store.add_share(share)? {
            Some(())
        } else {
            None
        };
        Ok(aggregated)
    }
}

/// Reliable-broadcast state for recovering certified aug data on demand.
///
/// Unlike [`CertifiedAugDataAckState`], this drives a request that the peer answers with its own
/// `CertifiedAugData`, and it is created for one author once we notice the gap rather than at epoch
/// start. A response that fails verification returns `Err`, which makes the reliable broadcast keep
/// retrying that peer with backoff until a valid cert arrives.
pub struct CertifiedAugDataRecoveryState<D> {
    epoch_state: Arc<EpochState>,
    // authors whose cert we are still waiting on
    pending: Mutex<HashSet<Author>>,
    // hands the accepted cert to the RandManager main loop, which caches and persists it
    recovered_tx: UnboundedSender<(Author, CertifiedAugData<D>)>,
}

impl<D> CertifiedAugDataRecoveryState<D> {
    pub fn new(
        epoch_state: Arc<EpochState>,
        pending: HashSet<Author>,
        recovered_tx: UnboundedSender<(Author, CertifiedAugData<D>)>,
    ) -> Arc<Self> {
        Arc::new(Self {
            epoch_state,
            pending: Mutex::new(pending),
            recovered_tx,
        })
    }
}

impl<S: TShare, D: TAugmentedData> BroadcastStatus<RandMessage<S, D>, RandMessage<S, D>>
    for Arc<CertifiedAugDataRecoveryState<D>>
{
    type Aggregated = ();
    type Message = RequestCertifiedAugData;
    type Response = CertifiedAugData<D>;

    fn add(&self, peer: Author, certified_data: Self::Response) -> anyhow::Result<Option<()>> {
        // Only the cert of the peer we asked for, for the epoch we asked in, is acceptable.
        // Anything else means the responder is not answering the question, so keep retrying it.
        //
        // The epoch check is load-bearing here: this response never passes through
        // `RandMessage::verify` -- it is an RPC response, not an inbound request -- and
        // `AugDataStore::add_certified_aug_data` does not re-check the epoch either. A cert from a
        // previous epoch can still carry an aggregate signature that validates against the current
        // verifier when the validator set is unchanged across the boundary, and caching its delta
        // would give us the wrong apk for that peer for the rest of this epoch.
        ensure!(
            certified_data.epoch() == self.epoch_state.epoch,
            "[RandManager] requested certified aug data for epoch {}, got one for epoch {}",
            self.epoch_state.epoch,
            certified_data.epoch()
        );
        ensure!(
            certified_data.author() == &peer,
            "[RandManager] requested certified aug data from {}, got a cert for {}",
            peer,
            certified_data.author()
        );

        // The 2f+1 multi-signature over the AugData. Every signer ran the delta/pk-share pairing
        // check before signing, so this carries the same guarantee as the proactive path.
        certified_data.verify(&self.epoch_state.verifier)?;

        info!(LogSchema::new(LogEvent::ReceiveCertifiedAugData)
            .epoch(certified_data.epoch())
            .remote_peer(peer));

        // Deliberately handed to the RandManager main loop instead of calling `augment` here.
        // `RandKeys::add_certified_apk` is a non-atomic check-then-set on a `OnceCell`, so a second
        // concurrent inserter for the same peer hits `set(..).unwrap()` and panics. The main loop
        // is the only runtime writer today (the proactive `CertifiedAugData` arm) and it persists
        // the cert as well; augmenting from this reliable-broadcast task would make it two.
        // A closed channel means the RandManager is gone, so stop polling.
        if self
            .recovered_tx
            .unbounded_send((peer, certified_data))
            .is_err()
        {
            return Ok(Some(()));
        }

        let mut pending = self.pending.lock();
        pending.remove(&peer);
        if pending.is_empty() {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}
