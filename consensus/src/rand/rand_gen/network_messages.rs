// Copyright © Velor Foundation
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
use velor_consensus_types::common::Author;
use velor_enum_conversion_derive::EnumConversion;
use velor_network::{protocols::network::RpcError, ProtocolId};
use velor_reliable_broadcast::RBMessage;
use velor_types::epoch_state::EpochState;
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
            RandMessage::Share(share) => share.verify(rand_config),
            RandMessage::AugData(aug_data) => {
                aug_data.verify(rand_config, fast_rand_config, sender)
            },
            RandMessage::CertifiedAugData(certified_aug_data) => {
                certified_aug_data.verify(&epoch_state.verifier)
            },
            RandMessage::FastShare(share) => {
                share.share.verify(fast_rand_config.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("[RandMessage] rand config for fast path not found")
                })?)
            },
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
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}
