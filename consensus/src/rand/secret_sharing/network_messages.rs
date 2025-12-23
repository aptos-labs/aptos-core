// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    network::TConsensusMsg, network_interface::ConsensusMsg,
    rand::secret_sharing::types::RequestSecretShare,
};
use anyhow::{bail, ensure};
use aptos_enum_conversion_derive::EnumConversion;
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::RBMessage;
use aptos_types::{
    epoch_state::EpochState,
    secret_sharing::{SecretShare, SecretShareConfig},
};
use bytes::Bytes;
use futures_channel::oneshot;
use serde::{Deserialize, Serialize};
use std::cmp::min;

#[derive(Clone, Serialize, Deserialize, EnumConversion)]
pub enum SecretShareMessage {
    RequestShare(RequestSecretShare),
    Share(SecretShare),
}

impl SecretShareMessage {
    pub fn verify(
        &self,
        epoch_state: &EpochState,
        config: &SecretShareConfig,
    ) -> anyhow::Result<()> {
        ensure!(self.epoch() == epoch_state.epoch);
        match self {
            SecretShareMessage::RequestShare(_) => Ok(()),
            SecretShareMessage::Share(share) => share.verify(config),
        }
    }
}

impl RBMessage for SecretShareMessage {}

impl TConsensusMsg for SecretShareMessage {
    fn epoch(&self) -> u64 {
        match self {
            SecretShareMessage::RequestShare(request) => request.epoch(),
            SecretShareMessage::Share(share) => share.metadata.epoch,
        }
    }

    fn from_network_message(msg: ConsensusMsg) -> anyhow::Result<Self> {
        match msg {
            ConsensusMsg::SecretShareMsg(msg) => Ok(bcs::from_bytes(&msg.data)?),
            _ => bail!("unexpected consensus message type {:?}", msg),
        }
    }

    fn into_network_message(self) -> ConsensusMsg {
        ConsensusMsg::SecretShareMsg(SecretShareNetworkMessage {
            epoch: self.epoch(),
            data: bcs::to_bytes(&self).expect("SecretShareMessage must be bcs serialize"),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SecretShareNetworkMessage {
    epoch: u64,
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

impl SecretShareNetworkMessage {
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

impl core::fmt::Debug for SecretShareNetworkMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretShareNetworkMessage")
            .field("epoch", &self.epoch)
            .field("data", &hex::encode(&self.data[..min(20, self.data.len())]))
            .finish()
    }
}

pub struct SecretShareRpc {
    pub msg: SecretShareMessage,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}
