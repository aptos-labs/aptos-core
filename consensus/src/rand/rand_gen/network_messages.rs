// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::types::FastShare;
use crate::{
    network::TConsensusMsg,
    network_interface::ConsensusMsg,
    rand::rand_gen::types::{
        AugData, AugDataSignature, CertifiedAugData, CertifiedAugDataAck, RandConfig, RandShare,
        RequestShare, TAugmentedData, TShare,
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
    FastShare(FastShare<S>),
}

impl<S: TShare, D: TAugmentedData> RandMessage<S, D> {
    pub fn verify(
        &self,
        epoch_state: &EpochState,
        rand_config: &RandConfig,
        fast_rand_config: &Option<RandConfig>,
        sender: Author,
    ) -> anyhow::Result<()> {
        ensure!(self.epoch() == epoch_state.epoch);
        match self {
            RandMessage::RequestShare(_) => Ok(()),
            RandMessage::Share(share) => {
                ensure_share_author_matches_sender(share.author(), &sender)?;
                share.verify(rand_config)
            },
            RandMessage::AugData(aug_data) => {
                aug_data.verify(rand_config, fast_rand_config, sender)
            },
            RandMessage::CertifiedAugData(certified_aug_data) => {
                certified_aug_data.verify(&epoch_state.verifier)
            },
            RandMessage::FastShare(share) => {
                ensure_share_author_matches_sender(share.author(), &sender)?;
                let cfg = fast_rand_config.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("[RandMessage] rand config for fast path not found")
                })?;
                share.share.verify(cfg)
            },
            _ => bail!("[RandMessage] unexpected message type"),
        }
    }
}

/// Rejects shares whose embedded author does not match the authenticated peer
/// that delivered the message. Without this, a peer could submit a share
/// claiming a different validator's identity and clobber that validator's
/// self-share inside the aggregator, potentially stalling randomness for the
/// round.
fn ensure_share_author_matches_sender(
    claimed_author: &Author,
    authenticated_sender: &Author,
) -> anyhow::Result<()> {
    ensure!(
        claimed_author == authenticated_sender,
        "[RandMessage] share carries author {} but the authenticated sender is {}",
        claimed_author,
        authenticated_sender,
    );
    Ok(())
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
            RandMessage::FastShare(share) => share.share.epoch(),
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
    /// The authenticated network peer that delivered `req`. Forwarded from the
    /// transport layer so the consumer can re-check author/sender bindings as
    /// defense-in-depth.
    pub sender: Author,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::rand_gen::types::{MockAugData, MockShare, RandShare};
    use aptos_consensus_types::common::Author;
    use aptos_crypto::{bls12381, Uniform};
    use aptos_dkg::{
        pvss::{traits::Transcript, Player, WeightedConfig},
        weighted_vuf::traits::WeightedVUF,
    };
    use aptos_types::{
        dkg::{real_dkg::maybe_dk_from_bls_sk, DKGSessionMetadata, DKGTrait, DefaultDKG},
        epoch_state::EpochState,
        on_chain_config::OnChainRandomnessConfig,
        randomness::{RandKeys, RandMetadata, WvufPP, WVUF},
        validator_verifier::{
            ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct, ValidatorVerifier,
        },
    };
    use rand::thread_rng;
    use std::{str::FromStr, sync::Arc};

    /// Per-validator key material derived up-front, then handed to the DKG.
    struct ValidatorBundle {
        author: Author,
        bls_sk: bls12381::PrivateKey,
        bls_pk: bls12381::PublicKey,
        dk: <DefaultDKG as DKGTrait>::NewValidatorDecryptKey,
        weight: u64,
    }

    fn spawn_validator_bundles(weights: &[u64]) -> Vec<ValidatorBundle> {
        weights
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let bls_sk = bls12381::PrivateKey::generate_for_testing();
                let bls_pk = bls12381::PublicKey::from(&bls_sk);
                let dk = maybe_dk_from_bls_sk(&bls_sk).unwrap();
                ValidatorBundle {
                    author: Author::from_str(&format!("{:x}", i)).unwrap(),
                    bls_sk,
                    bls_pk,
                    dk,
                    weight: *w,
                }
            })
            .collect()
    }

    fn assemble_validator_set(bundles: &[ValidatorBundle]) -> Arc<ValidatorVerifier> {
        let consensus_infos: Vec<ValidatorConsensusInfo> = bundles
            .iter()
            .map(|b| ValidatorConsensusInfo::new(b.author, b.bls_pk.clone(), b.weight))
            .collect();
        Arc::new(ValidatorVerifier::new(consensus_infos))
    }

    fn move_consensus_infos(bundles: &[ValidatorBundle]) -> Vec<ValidatorConsensusInfoMoveStruct> {
        bundles
            .iter()
            .map(|b| {
                ValidatorConsensusInfoMoveStruct::from(ValidatorConsensusInfo::new(
                    b.author,
                    b.bls_pk.clone(),
                    b.weight,
                ))
            })
            .collect()
    }

    /// Holds the post-DKG key material the local validator needs.
    struct LocalKeyBundle {
        rand_keys: RandKeys,
        vuf_pp: WvufPP,
    }

    fn run_dkg_and_derive_local_keys(
        bundles: &[ValidatorBundle],
        local_index: usize,
    ) -> LocalKeyBundle {
        let mut rng = thread_rng();
        let validator_set = move_consensus_infos(bundles);
        let session = DKGSessionMetadata {
            dealer_epoch: 0,
            randomness_config: OnChainRandomnessConfig::default_enabled().into(),
            dealer_validator_set: validator_set.clone(),
            target_validator_set: validator_set,
        };
        let pub_params = DefaultDKG::new_public_params(&session);
        let vuf_pp = WvufPP::from(&pub_params.pvss_config.pp);

        let input = <DefaultDKG as DKGTrait>::InputSecret::generate_for_testing();
        let transcript = DefaultDKG::generate_transcript(
            &mut rng,
            &pub_params,
            &input,
            0,
            &bundles[0].bls_sk,
        );

        // Pull this validator's secret WVUF share, then augment it.
        let (decrypted_sk, decrypted_pk) = DefaultDKG::decrypt_secret_share_from_transcript(
            &pub_params,
            &transcript,
            local_index as u64,
            &bundles[local_index].dk,
        )
        .unwrap();
        let (ask, apk) =
            WVUF::augment_key_pair(&vuf_pp, decrypted_sk.main, decrypted_pk.main, &mut rng);

        // ...and gather every validator's public WVUF share for verification.
        let pk_shares: Vec<_> = (0..bundles.len())
            .map(|id| {
                transcript
                    .main
                    .get_public_key_share(&pub_params.pvss_config.wconfig, &Player { id })
            })
            .collect();

        LocalKeyBundle {
            rand_keys: RandKeys::new(ask, apk, pk_shares, bundles.len()),
            vuf_pp,
        }
    }

    fn weighted_threshold_config(weights: &[u64]) -> WeightedConfig {
        let usize_weights: Vec<usize> = weights.iter().map(|w| *w as usize).collect();
        let total: usize = usize_weights.iter().sum();
        WeightedConfig::new(total / 2, usize_weights).unwrap()
    }

    /// Minimal fixture for exercising `RandMessage::verify`. The cryptographic
    /// material is real, but the share-author binding check fires before any
    /// crypto runs, so for the failure paths under test the keys are inert.
    struct MessageVerifyFixture {
        validators: Vec<Author>,
        epoch_state: Arc<EpochState>,
        primary_config: RandConfig,
    }

    const TEST_EPOCH: u64 = 1;
    const TEST_ROUND: u64 = 1;

    fn build_fixture(weights: Vec<u64>, local_index: usize) -> MessageVerifyFixture {
        let bundles = spawn_validator_bundles(&weights);
        let validators: Vec<Author> = bundles.iter().map(|b| b.author).collect();
        let verifier = assemble_validator_set(&bundles);
        let local_keys = run_dkg_and_derive_local_keys(&bundles, local_index);
        let wconfig = weighted_threshold_config(&weights);

        let primary_config = RandConfig::new(
            validators[local_index],
            TEST_EPOCH,
            verifier.clone(),
            local_keys.vuf_pp,
            local_keys.rand_keys,
            wconfig,
        );
        let epoch_state = Arc::new(EpochState {
            epoch: TEST_EPOCH,
            verifier,
        });

        MessageVerifyFixture {
            validators,
            epoch_state,
            primary_config,
        }
    }

    fn round_one_metadata() -> RandMetadata {
        RandMetadata {
            epoch: TEST_EPOCH,
            round: TEST_ROUND,
        }
    }

    #[test]
    fn helper_accepts_when_author_equals_sender() {
        let me = Author::from_str("a").unwrap();
        assert!(ensure_share_author_matches_sender(&me, &me).is_ok());
    }

    #[test]
    fn helper_rejects_when_author_and_sender_differ() {
        let claimed = Author::from_str("a").unwrap();
        let actual = Author::from_str("b").unwrap();
        let err = ensure_share_author_matches_sender(&claimed, &actual)
            .expect_err("mismatch must be rejected");
        let rendered = err.to_string();
        assert!(rendered.contains("authenticated sender"), "{rendered}");
    }

    #[test]
    fn share_message_rejected_when_sender_impersonates_another_validator() {
        // 4 validators; pick non-zero indices so the test does not coincidentally
        // mirror the most obvious "first vs. last" choice.
        let fx = build_fixture(vec![10, 10, 10, 10], 1);
        let pretended_author = fx.validators[3];
        let real_sender = fx.validators[2];

        let forged = RandShare::<MockShare>::new(pretended_author, round_one_metadata(), MockShare);
        let envelope = RandMessage::<MockShare, MockAugData>::Share(forged);

        let outcome = envelope.verify(
            &fx.epoch_state,
            &fx.primary_config,
            &None,
            real_sender,
        );
        let err = outcome.expect_err("verify must reject impersonation");
        assert!(
            err.to_string().contains("authenticated sender"),
            "unexpected error: {err}",
        );
    }

    #[test]
    fn share_message_accepted_when_sender_matches_author() {
        let fx = build_fixture(vec![10, 10, 10, 10], 1);
        let honest_author = fx.validators[3];
        let honest = RandShare::<MockShare>::new(honest_author, round_one_metadata(), MockShare);
        let envelope = RandMessage::<MockShare, MockAugData>::Share(honest);

        envelope
            .verify(&fx.epoch_state, &fx.primary_config, &None, honest_author)
            .expect("self-share from the same peer must verify");
    }

    #[test]
    fn fast_share_message_rejected_when_sender_impersonates_another_validator() {
        let fx = build_fixture(vec![10, 10, 10, 10], 1);
        let pretended_author = fx.validators[2];
        let real_sender = fx.validators[3];

        let inner =
            RandShare::<MockShare>::new(pretended_author, round_one_metadata(), MockShare);
        let forged_fast = FastShare::new(inner);
        let envelope = RandMessage::<MockShare, MockAugData>::FastShare(forged_fast);

        let fast_cfg = Some(fx.primary_config.clone());
        let outcome = envelope.verify(
            &fx.epoch_state,
            &fx.primary_config,
            &fast_cfg,
            real_sender,
        );
        let err = outcome.expect_err("FastShare verify must reject impersonation");
        assert!(
            err.to_string().contains("authenticated sender"),
            "unexpected error: {err}",
        );
    }
}
