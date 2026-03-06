// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
            RandMessage::Share(share) => share.optimistic_verify(rand_config, &sender),
            RandMessage::AugData(aug_data) => {
                aug_data.verify(rand_config, fast_rand_config, sender)
            },
            RandMessage::CertifiedAugData(certified_aug_data) => {
                certified_aug_data.verify(&epoch_state.verifier)
            },
            RandMessage::FastShare(share) => share.optimistic_verify(
                fast_rand_config.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("[RandMessage] rand config for fast path not found")
                })?,
                &sender,
            ),
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
    pub sender: Author,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::rand_gen::types::{MockAugData, MockShare, RandConfig, RandShare};
    use aptos_consensus_types::common::Author;
    use aptos_crypto::{bls12381, Uniform};
    use aptos_dkg::{
        pvss::{traits::TranscriptCore, Player, WeightedConfigBlstrs},
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

    struct TestContext {
        authors: Vec<Author>,
        epoch_state: Arc<EpochState>,
        rand_config: RandConfig,
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
        let result = msg.verify(&ctx.epoch_state, &ctx.rand_config, &None, attacker);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not match sender"),);

        // Verify with victim as sender should succeed
        let result = msg.verify(&ctx.epoch_state, &ctx.rand_config, &None, victim);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fast_share_verify_rejects_mismatched_sender() {
        let ctx = build_test_context(vec![100, 100, 100], 0);
        let victim = ctx.authors[0];
        let attacker = ctx.authors[2];
        let metadata = RandMetadata {
            epoch: ctx.epoch_state.epoch,
            round: 1,
        };

        // Create a fast share claiming to be from victim
        let forged_share = RandShare::<MockShare>::new(victim, metadata, MockShare);
        let forged_fast_share = super::super::types::FastShare::new(forged_share);
        let msg = RandMessage::<MockShare, MockAugData>::FastShare(forged_fast_share);

        // Verify with attacker as sender should fail
        let result = msg.verify(
            &ctx.epoch_state,
            &ctx.rand_config,
            &Some(ctx.rand_config.clone()),
            attacker,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not match sender"),);

        // Verify with victim as sender should succeed
        let result = msg.verify(
            &ctx.epoch_state,
            &ctx.rand_config,
            &Some(ctx.rand_config.clone()),
            victim,
        );
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

        let result = msg.verify(&ctx.epoch_state, &ctx.rand_config, &None, author);
        assert!(result.is_ok());
    }
}
