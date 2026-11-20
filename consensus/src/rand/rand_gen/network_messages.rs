// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    network::TConsensusMsg,
    network_interface::ConsensusMsg,
    rand::rand_gen::types::{
        AugData, AugDataSignature, CertifiedAugData, CertifiedAugDataAck, RandConfig, RandShare,
        RequestCertifiedAugData, RequestShare, TAugmentedData, TShare,
    },
};
use anyhow::{bail, ensure};
use aptos_consensus_types::common::Author;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::RBMessage;
use aptos_types::epoch_state::EpochState;
use bytes::Bytes;
use futures_channel::oneshot;
use serde::{Deserialize, Serialize};
use std::cmp::min;

#[derive(Clone, Serialize, Deserialize, EnumConversion)]
pub enum RandMessage<S, D> {
    RequestShare(RequestShare),
    Share(RandShare<S>),
    AugData(AugData<D>),
    AugDataSignature(AugDataSignature),
    CertifiedAugData(CertifiedAugData<D>),
    CertifiedAugDataAck(CertifiedAugDataAck),
    RequestCertifiedAugData(RequestCertifiedAugData),
}

impl<S: TShare, D: TAugmentedData> RandMessage<S, D> {
    pub fn verify(
        &self,
        epoch_state: &EpochState,
        rand_config: &RandConfig,
        sender: Author,
    ) -> anyhow::Result<()> {
        ensure!(self.epoch() == epoch_state.epoch);
        match self {
            RandMessage::RequestShare(_) => Ok(()),
            RandMessage::Share(share) => share.optimistic_verify(rand_config, &sender),
            RandMessage::AugData(aug_data) => aug_data.verify(rand_config, sender),
            RandMessage::CertifiedAugData(certified_aug_data) => {
                certified_aug_data.verify(&epoch_state.verifier)
            },
            // Nothing to authenticate: the response is a `CertifiedAugData`, which carries its
            // own 2f+1 multi-signature and is verified on the way back in.
            RandMessage::RequestCertifiedAugData(_) => Ok(()),
            _ => bail!("[RandMessage] unexpected message type"),
        }
    }
}

impl<S: TShare, D: TAugmentedData> RBMessage for RandMessage<S, D> {}

impl<S: TShare, D: TAugmentedData> TConsensusMsg for RandMessage<S, D> {
    fn epoch(&self) -> u64 {
        match self {
            RandMessage::RequestShare(request) => request.epoch(),
            RandMessage::Share(share) => share.epoch(),
            RandMessage::AugData(aug_data) => aug_data.epoch(),
            RandMessage::AugDataSignature(signature) => signature.epoch(),
            RandMessage::CertifiedAugData(certified_aug_data) => certified_aug_data.epoch(),
            RandMessage::CertifiedAugDataAck(ack) => ack.epoch(),
            RandMessage::RequestCertifiedAugData(request) => request.epoch(),
        }
    }

    fn from_network_message(msg: ConsensusMsg) -> anyhow::Result<Self> {
        match msg {
            ConsensusMsg::RandGenMessage(msg) => Ok(bcs::from_bytes(&msg.data)?),
            _ => bail!("unexpected consensus message type {:?}", msg),
        }
    }

    #[allow(clippy::unwrap_used)]
    fn into_network_message(self) -> ConsensusMsg {
        ConsensusMsg::RandGenMessage(RandGenMessage {
            epoch: self.epoch(),
            data: bcs::to_bytes(&self).unwrap(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RandGenMessage {
    epoch: u64,
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

impl RandGenMessage {
    pub fn new(epoch: u64, data: Vec<u8>) -> Self {
        Self { epoch, data }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

impl core::fmt::Debug for RandGenMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RandGenMessage")
            .field("epoch", &self.epoch)
            .field("data", &hex::encode(&self.data[..min(20, self.data.len())]))
            .finish()
    }
}

pub struct RpcRequest<S, D> {
    pub req: RandMessage<S, D>,
    pub sender: Author,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::rand_gen::{
        rand_manager::Receiver,
        reliable_broadcast_state::CertifiedAugDataRecoveryState,
        types::{MockAugData, MockShare, RandConfig, RandShare},
    };
    use aptos_consensus_types::common::Author;
    use aptos_crypto::{bls12381, Uniform};
    use aptos_dkg::{
        pvss::{traits::TranscriptCore, Player, WeightedConfigBlstrs},
        weighted_vuf::traits::WeightedVUF,
    };
    use aptos_reliable_broadcast::BroadcastStatus;
    use aptos_types::{
        aggregate_signature::{AggregateSignature, PartialSignatures},
        dkg::{real_dkg::maybe_dk_from_bls_sk, DKGSessionMetadata, DKGTrait, DefaultDKG},
        epoch_state::EpochState,
        on_chain_config::OnChainRandomnessConfig,
        randomness::{RandKeys, RandMetadata, WvufPP, WVUF},
        validator_signer::ValidatorSigner,
        validator_verifier::{
            ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct, ValidatorVerifier,
        },
    };
    use futures::{FutureExt, StreamExt};
    use futures_channel::mpsc::unbounded;
    use rand::thread_rng;
    use std::{str::FromStr, sync::Arc};

    struct TestContext {
        authors: Vec<Author>,
        epoch_state: Arc<EpochState>,
        rand_config: RandConfig,
        private_keys: Vec<bls12381::PrivateKey>,
    }

    fn build_test_context(weights: Vec<u64>, my_index: usize) -> TestContext {
        let target_epoch = 1;
        let num_validators = weights.len();
        let mut rng = thread_rng();
        let authors: Vec<_> = (0..num_validators)
            .map(|i| Author::from_str(&format!("{:x}", i)).unwrap())
            .collect();
        let private_keys: Vec<bls12381::PrivateKey> = (0..num_validators)
            .map(|_| bls12381::PrivateKey::generate_for_testing())
            .collect();
        let public_keys: Vec<bls12381::PublicKey> =
            private_keys.iter().map(bls12381::PublicKey::from).collect();
        let dkg_decrypt_keys: Vec<<DefaultDKG as DKGTrait>::NewValidatorDecryptKey> = private_keys
            .iter()
            .map(|sk| maybe_dk_from_bls_sk(sk).unwrap())
            .collect();
        let consensus_infos: Vec<ValidatorConsensusInfo> = (0..num_validators)
            .map(|idx| {
                ValidatorConsensusInfo::new(authors[idx], public_keys[idx].clone(), weights[idx])
            })
            .collect();
        let consensus_info_move_structs = consensus_infos
            .clone()
            .into_iter()
            .map(ValidatorConsensusInfoMoveStruct::from)
            .collect::<Vec<_>>();
        let verifier = Arc::new(ValidatorVerifier::new(consensus_infos));
        let epoch_state = Arc::new(EpochState {
            epoch: target_epoch,
            verifier: verifier.clone(),
        });
        let dkg_session_metadata = DKGSessionMetadata {
            dealer_epoch: 0,
            randomness_config: OnChainRandomnessConfig::default_enabled().into(),
            dealer_validator_set: consensus_info_move_structs.clone(),
            target_validator_set: consensus_info_move_structs,
        };
        let dkg_pub_params = DefaultDKG::new_public_params(&dkg_session_metadata);
        let input_secret = <DefaultDKG as DKGTrait>::InputSecret::generate_for_testing();
        let transcript = DefaultDKG::generate_transcript(
            &mut rng,
            &dkg_pub_params,
            &input_secret,
            0,
            &private_keys[0],
            &public_keys[0],
        );
        let (sk, pk) = DefaultDKG::decrypt_secret_share_from_transcript(
            &dkg_pub_params,
            &transcript,
            my_index as u64,
            &dkg_decrypt_keys[my_index],
        )
        .unwrap();
        let pk_shares = (0..num_validators)
            .map(|id| {
                transcript
                    .main
                    .get_public_key_share(&dkg_pub_params.pvss_config.wconfig, &Player { id })
            })
            .collect::<Vec<_>>();
        let vuf_pub_params = WvufPP::from(&dkg_pub_params.pvss_config.pp);
        let aggregate_pk = transcript.main.get_dealt_public_key();
        let (ask, apk) = WVUF::augment_key_pair(&vuf_pub_params, sk.main, pk.main, &mut rng);
        let rand_keys = RandKeys::new(ask, apk, pk_shares, num_validators);
        let weights_usize: Vec<usize> = weights.into_iter().map(|x| x as usize).collect();
        let half_total_weights = weights_usize.iter().sum::<usize>() / 2;
        let weighted_config = WeightedConfigBlstrs::new(half_total_weights, weights_usize).unwrap();
        let rand_config = RandConfig::new(
            authors[my_index],
            target_epoch,
            verifier,
            vuf_pub_params,
            rand_keys,
            weighted_config,
            aggregate_pk,
            false,
        );

        TestContext {
            authors,
            epoch_state,
            rand_config,
            private_keys,
        }
    }

    #[test]
    fn test_share_verify_rejects_mismatched_sender() {
        let ctx = build_test_context(vec![100, 100, 100], 0);
        let victim = ctx.authors[0];
        let attacker = ctx.authors[2];
        let metadata = RandMetadata {
            epoch: ctx.epoch_state.epoch,
            round: 1,
        };

        // Create a share claiming to be from victim
        let forged_share = RandShare::<MockShare>::new(victim, metadata, MockShare);
        let msg = RandMessage::<MockShare, MockAugData>::Share(forged_share);

        // Verify with attacker as sender should fail
        let result = msg.verify(&ctx.epoch_state, &ctx.rand_config, attacker);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not match sender"),);

        // Verify with victim as sender should succeed
        let result = msg.verify(&ctx.epoch_state, &ctx.rand_config, victim);
        assert!(result.is_ok());
    }

    #[test]
    fn test_share_verify_accepts_matching_sender() {
        let ctx = build_test_context(vec![100, 100, 100], 0);
        let author = ctx.authors[0];
        let metadata = RandMetadata {
            epoch: ctx.epoch_state.epoch,
            round: 1,
        };

        let share = RandShare::<MockShare>::new(author, metadata, MockShare);
        let msg = RandMessage::<MockShare, MockAugData>::Share(share);

        let result = msg.verify(&ctx.epoch_state, &ctx.rand_config, author);
        assert!(result.is_ok());
    }

    // Recovery of a peer's certified aug data on demand. The request is answered with the
    // responder's own `CertifiedAugData`, which `CertifiedAugDataRecoveryState` validates before
    // handing it to the RandManager.

    /// Certify an aug data for `author` in `epoch` the way the proactive broadcast does: every
    /// validator signs it and the signatures are aggregated.
    fn certify_aug_data_for_epoch(
        ctx: &TestContext,
        epoch: u64,
        author: Author,
    ) -> CertifiedAugData<MockAugData> {
        let aug_data = AugData::new(epoch, author, MockAugData);
        let mut partial = PartialSignatures::empty();
        for (idx, sk) in ctx.private_keys.iter().enumerate() {
            let signer = ValidatorSigner::new(ctx.authors[idx], Arc::new(sk.clone()));
            partial.add_signature(ctx.authors[idx], signer.sign(&aug_data).unwrap());
        }
        let signatures = ctx
            .epoch_state
            .verifier
            .aggregate_signatures(partial.signatures_iter())
            .unwrap();
        CertifiedAugData::new(aug_data, signatures)
    }

    fn certify_aug_data(ctx: &TestContext, author: Author) -> CertifiedAugData<MockAugData> {
        certify_aug_data_for_epoch(ctx, ctx.epoch_state.epoch, author)
    }

    /// Wraps the recovery state to pin the `TShare` type of the messages it broadcasts, which the
    /// `BroadcastStatus` impl leaves free (the reliable broadcast normally pins it).
    struct MockRecovery {
        state: Arc<CertifiedAugDataRecoveryState<MockAugData>>,
    }

    impl MockRecovery {
        fn add(
            &self,
            peer: Author,
            certified: CertifiedAugData<MockAugData>,
        ) -> anyhow::Result<Option<()>> {
            <Arc<CertifiedAugDataRecoveryState<MockAugData>> as BroadcastStatus<
                RandMessage<MockShare, MockAugData>,
                RandMessage<MockShare, MockAugData>,
            >>::add(&self.state, peer, certified)
        }
    }

    fn recovery_state(
        ctx: &TestContext,
        peer: Author,
    ) -> (
        MockRecovery,
        Receiver<(Author, CertifiedAugData<MockAugData>)>,
    ) {
        let (tx, rx) = unbounded();
        let state = CertifiedAugDataRecoveryState::new(
            ctx.epoch_state.clone(),
            [peer].into_iter().collect(),
            tx,
        );
        (MockRecovery { state }, rx)
    }

    #[test]
    fn test_request_certified_aug_data_verify_and_epoch() {
        let ctx = build_test_context(vec![100, 100, 100], 0);
        let msg = RandMessage::<MockShare, MockAugData>::RequestCertifiedAugData(
            RequestCertifiedAugData::new(ctx.epoch_state.epoch),
        );

        // A request carries nothing to authenticate: the response is a `CertifiedAugData`, which
        // brings its own 2f+1 multi-signature and is verified when it comes back.
        assert!(msg
            .verify(&ctx.epoch_state, &ctx.rand_config, ctx.authors[1])
            .is_ok());
        // The epoch routes the message to the RandManager of the epoch it belongs to.
        assert_eq!(msg.epoch(), ctx.epoch_state.epoch);
    }

    #[test]
    fn test_cert_recovery_accepts_the_requested_peers_cert() {
        let ctx = build_test_context(vec![100, 100, 100], 0);
        let peer = ctx.authors[1];
        let (state, mut rx) = recovery_state(&ctx, peer);
        let epoch = ctx.epoch_state.epoch;

        // `Ok(Some(()))` tells the reliable broadcast to stop polling this peer.
        assert!(state
            .add(peer, certify_aug_data(&ctx, peer))
            .unwrap()
            .is_some());

        // The accepted cert is handed to the RandManager main loop, which persists it.
        let (author, recovered) = rx
            .next()
            .now_or_never()
            .flatten()
            .expect("cert should be forwarded");
        assert_eq!(author, peer);
        assert_eq!(recovered.author(), &peer);
        assert_eq!(recovered.epoch(), epoch);
        // A second response would find nothing left to wait for.
        assert!(rx.next().now_or_never().flatten().is_none());
    }

    #[test]
    fn test_cert_recovery_rejects_a_cert_for_another_author() {
        let ctx = build_test_context(vec![100, 100, 100], 0);
        let peer = ctx.authors[1];
        let (state, mut rx) = recovery_state(&ctx, peer);

        // Properly certified, but for a different validator: the responder is not answering the
        // question, so the reliable broadcast must keep retrying it.
        assert!(state
            .add(peer, certify_aug_data(&ctx, ctx.authors[2]))
            .is_err());
        assert!(rx.next().now_or_never().flatten().is_none());
    }

    #[test]
    fn test_cert_recovery_rejects_a_cert_from_another_epoch() {
        let ctx = build_test_context(vec![100, 100, 100], 0);
        let peer = ctx.authors[1];
        let (state, mut rx) = recovery_state(&ctx, peer);

        // Validly signed by the current validator set, but for a previous epoch. The set is often
        // unchanged across an epoch boundary, so the aggregate signature still verifies -- and
        // caching that delta would give us the wrong apk for this peer for the rest of this epoch.
        // This response never passes through `RandMessage::verify`, which is where the proactive
        // path checks the epoch, so the recovery state has to check it itself.
        let stale = certify_aug_data_for_epoch(&ctx, ctx.epoch_state.epoch - 1, peer);
        assert!(state.add(peer, stale).is_err());
        assert!(rx.next().now_or_never().flatten().is_none());
    }

    #[test]
    fn test_cert_recovery_rejects_a_cert_without_quorum() {
        let ctx = build_test_context(vec![100, 100, 100], 0);
        let peer = ctx.authors[1];
        let (state, mut rx) = recovery_state(&ctx, peer);
        let aug_data = AugData::new(ctx.epoch_state.epoch, peer, MockAugData);

        // Right author, but no 2f+1 multi-signature behind it.
        let certified = CertifiedAugData::new(aug_data, AggregateSignature::empty());
        assert!(state.add(peer, certified).is_err());
        assert!(rx.next().now_or_never().flatten().is_none());
    }
}
