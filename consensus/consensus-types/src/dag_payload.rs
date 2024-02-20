use crate::common::{Author, Payload, Round};
use anyhow::ensure;
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_types::transaction::SignedTransaction;
use core::fmt;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use tokio::sync::oneshot;

pub type PayloadDigest = HashValue;

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct PayloadId {
    epoch: u64,
    round: Round,
    author: Author,
}

impl PayloadId {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn author(&self) -> Author {
        self.author
    }
}

impl fmt::Display for PayloadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PayloadId [Epoch: {}, Round: {}, Author: {}]",
            self.epoch, self.round, self.author
        )
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct PayloadInfo {
    id: PayloadId,
    digest: PayloadDigest,
    num_txns: usize,
    size_in_bytes: usize,
}

impl PayloadInfo {
    pub fn new_for_test(
        epoch: u64,
        round: Round,
        author: Author,
        digest: PayloadDigest,
        num_txns: usize,
        size_in_bytes: usize,
    ) -> Self {
        Self {
            id: PayloadId {
                epoch,
                round,
                author,
            },
            digest,
            num_txns,
            size_in_bytes,
        }
    }

    pub fn len(&self) -> usize {
        self.num_txns
    }

    pub fn size(&self) -> usize {
        self.size_in_bytes
    }

    pub fn id(&self) -> &PayloadId {
        &self.id
    }

    pub fn round(&self) -> Round {
        self.id.round
    }

    pub fn author(&self) -> &Author {
        &self.id.author
    }

    pub fn epoch(&self) -> u64 {
        self.id.epoch
    }

    pub fn digest(&self) -> &PayloadDigest {
        &self.digest
    }
}

impl Deref for PayloadInfo {
    type Target = PayloadId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, CryptoHasher)]
pub struct DecoupledPayload {
    info: PayloadInfo,
    payload: Payload,
}

impl DecoupledPayload {
    pub fn new(epoch: u64, round: Round, author: Author, payload: Payload) -> Self {
        let id = PayloadId {
            epoch,
            round,
            author,
        };
        let digest = Self::calculate_digest(&id, &payload);
        let info = PayloadInfo {
            id,
            digest,
            num_txns: payload.len(),
            size_in_bytes: payload.size(),
        };
        Self { info, payload }
    }

    pub fn id(&self) -> &PayloadId {
        &self.info.id
    }

    pub fn epoch(&self) -> u64 {
        self.info.epoch()
    }

    pub fn info(&self) -> PayloadInfo {
        self.info.clone()
    }

    pub fn round(&self) -> Round {
        self.info.round()
    }

    pub fn author(&self) -> &Author {
        self.info.author()
    }

    pub fn digest(&self) -> &PayloadDigest {
        self.info.digest()
    }

    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn calculate_digest(id: &PayloadId, payload: &Payload) -> PayloadDigest {
        #[derive(Serialize)]
        struct DecoupledPayloadWithoutDigest<'a> {
            id: &'a PayloadId,
            payload: &'a Payload,
        }

        impl<'a> CryptoHash for DecoupledPayloadWithoutDigest<'a> {
            type Hasher = DecoupledPayloadHasher;

            fn hash(&self) -> HashValue {
                let mut state = Self::Hasher::new();
                let bytes = bcs::to_bytes(&self).expect("Unable to serialize node");
                state.update(&bytes);
                state.finish()
            }
        }

        DecoupledPayloadWithoutDigest { id, payload }.hash()
    }

    pub fn verify(&self) -> anyhow::Result<()> {
        ensure!(self.digest() == &Self::calculate_digest(&self.info.id, &self.payload));

        Ok(())
    }
}

impl Deref for DecoupledPayload {
    type Target = Payload;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DAGPayloadBundle {
    payload_infos: Vec<PayloadInfo>,
}

impl DAGPayloadBundle {
    pub fn new_empty() -> Self {
        Self {
            payload_infos: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.payload_infos.iter().map(|info| info.num_txns).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn extend(&mut self, other: DAGPayloadBundle) {
        self.payload_infos.extend(other.payload_infos)
    }

    pub fn size(&self) -> usize {
        self.payload_infos
            .iter()
            .map(|info| info.size_in_bytes)
            .sum()
    }

    pub fn payload_infos(&self) -> &[PayloadInfo] {
        &self.payload_infos
    }
}

impl fmt::Display for DAGPayloadBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for info in &self.payload_infos {
            write!(f, "{}, ", info.id())?;
        }
        Ok(())
    }
}

impl From<PayloadInfo> for DAGPayloadBundle {
    fn from(info: PayloadInfo) -> Self {
        Self {
            payload_infos: vec![info],
        }
    }
}

impl From<PayloadInfo> for Payload {
    fn from(info: PayloadInfo) -> Self {
        Self::DAG(info.into())
    }
}

pub struct PayloadLinkMsg {
    bundle: DAGPayloadBundle,
    rx: oneshot::Sender<anyhow::Result<Vec<SignedTransaction>>>,
}

impl PayloadLinkMsg {
    pub fn new(
        bundle: DAGPayloadBundle,
        rx: oneshot::Sender<anyhow::Result<Vec<SignedTransaction>>>,
    ) -> Self {
        Self { bundle, rx }
    }

    pub fn unwrap(
        self,
    ) -> (
        DAGPayloadBundle,
        oneshot::Sender<anyhow::Result<Vec<SignedTransaction>>>,
    ) {
        (self.bundle, self.rx)
    }
}
