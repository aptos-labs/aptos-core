pub mod account;
pub mod simple_hash;
pub mod u256_define;
pub mod compute_res;
pub mod events;
pub mod on_chain_config;
pub mod config_storage;
use crate::account::{ExternalAccountAddress, ExternalChainId};
use crate::u256_define::HashValue;
use compute_res::ComputeRes;
use core::str;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::{fmt::Debug, hash::Hasher};
use u256_define::{BlockId, Random, TxnHash};

pub type Round = u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ExecutionBlocks {
    pub latest_block_hash: [u8; 32],
    pub latest_block_number: u64,
    pub latest_ts: u64,
    pub blocks: Vec<Vec<u8>>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ExternalPayloadAttr {
    // s since epoch
    pub ts: u64,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct ExternalBlockMeta {
    // Unique identifier for block: hash of block body
    pub block_id: BlockId,
    pub block_number: u64,
    pub usecs: u64,
    pub epoch: u64,
    pub randomness: Option<Random>,
    pub block_hash: Option<ComputeRes>,
}

#[derive(Debug, Clone)]
pub struct ExternalBlock {
    pub block_meta: ExternalBlockMeta,
    pub txns: Vec<VerifiedTxn>,
}

#[derive(Debug)]
pub enum ExecError {
    InternalError,
    DuplicateExecError,
}

pub enum ExecTxn {
    RawTxn(Vec<u8>),          // from client
    VerifiedTxn(VerifiedTxn), // from peer
}

#[derive(Debug, Clone)]
pub struct VerifiedTxnWithAccountSeqNum {
    pub txn: VerifiedTxn,
    pub account_seq_num: u64,
}

pub struct ExecutionArgs {
    pub block_number_to_block_id: BTreeMap<u64, HashValue>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct VerifiedTxn {
    pub bytes: Vec<u8>,
    pub sender: ExternalAccountAddress,
    pub sequence_number: u64,
    pub chain_id: ExternalChainId,
    #[serde(skip)]
    pub committed_hash: OnceCell<TxnHash>,
}

// implment the Debug for VerifiedTxn
impl std::fmt::Debug for VerifiedTxn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VerifiedTxn {{ sender: {:?}, sequence_number: {:?}, committed_hash: {:?} }}", self.sender, self.sequence_number, self.committed_hash)
    }
}

impl Hash for VerifiedTxn {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
        self.sender.hash(state);
        self.sequence_number.hash(state);
        self.chain_id.hash(state);
    }
}

pub trait CryptoTxnHasher : Send + Sync {
    fn get_hash(bytes: &Vec<u8>) -> [u8; 32];
}

pub static GLOBAL_CRYPTO_TXN_HASHER: OnceLock<Box<dyn Fn(&Vec<u8>) -> [u8; 32] + Send + Sync>> = OnceLock::new();

impl VerifiedTxn {
    pub fn new(
        bytes: Vec<u8>,
        sender: ExternalAccountAddress,
        sequence_number: u64,
        chain_id: ExternalChainId,
        committed_hash: TxnHash,
    ) -> Self {
        Self { bytes, sender, sequence_number, chain_id, committed_hash: committed_hash.into() }
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    pub fn sender(&self) -> &ExternalAccountAddress {
        &self.sender
    }

    pub fn seq_number(&self) -> u64 {
        self.sequence_number
    }

    pub fn committed_hash(&self) -> [u8; 32] {
        self.committed_hash
            .get_or_init(|| {
                u256_define::TxnHash::new(GLOBAL_CRYPTO_TXN_HASHER.get().unwrap()(self.bytes()))
            })
            .bytes()
    }
}
