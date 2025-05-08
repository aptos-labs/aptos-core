pub mod account;
pub mod mock_execution_layer;
pub mod simple_hash;
pub mod u256_define;
pub mod compute_res;
pub mod events;
pub mod on_chain_config;
pub mod config_storage;
use crate::account::{ExternalAccountAddress, ExternalChainId};
use crate::u256_define::HashValue;
use async_trait::async_trait;
use compute_res::ComputeRes;
use core::str;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::{fmt::Debug, hash::Hasher, sync::Arc};
use u256_define::{BlockId, Random, TxnHash};

pub type Round = u64;

#[async_trait]
pub trait ConsensusApi: Send + Sync {
    async fn send_ordered_block(&self, parent_id: [u8; 32], ordered_block: ExternalBlock);

    async fn recv_executed_block_hash(&self, head: ExternalBlockMeta) -> ComputeRes;

    async fn commit_block_hash(&self, head: [u8; 32]);
}

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

#[async_trait]
pub trait ExecutionChannel: Send + Sync {
    ///
    /// # Returns
    /// A `Vec` containing tuples, where each tuple consists of:
    /// - `TxnHash`: The committed hash for the newly added txn
    /// - `sender_latest_committed_sequence_number`: The latest committed sequence number associated with the sender on the execution layer.
    ///
    async fn send_user_txn(&self, bytes: ExecTxn) -> Result<TxnHash, ExecError>;

    async fn recv_unbroadcasted_txn(&self) -> Result<Vec<VerifiedTxn>, ExecError>;

    async fn check_block_txns(
        &self,
        payload_attr: ExternalPayloadAttr,
        txns: Vec<VerifiedTxn>,
    ) -> Result<bool, ExecError>;

    ///
    /// # Returns
    /// A `Vec` containing tuples, where each tuple consists of:
    /// - `VerifiedTxn`: The transaction object.
    /// - `sender_latest_committed_sequence_number`: The latest committed sequence number associated with the sender on the execution layer.
    ///
    async fn send_pending_txns(&self) -> Result<Vec<VerifiedTxnWithAccountSeqNum>, ExecError>;

    // async fn send_ordered_block(&self, ordered_block: Vec<Txns>, block_number: BlockNumber, parent_mata_data: ExternalBlockMeta) -> Result<(), ExecError>;
    async fn recv_ordered_block(
        &self,
        parent_id: BlockId,
        ordered_block: ExternalBlock,
    ) -> Result<(), ExecError>;

    // the block hash is the hash of the block that has been executed, which is passed by the send_ordered_block
    async fn send_executed_block_hash(
        &self,
        head: ExternalBlockMeta,
    ) -> Result<ComputeRes, ExecError>;

    // this function is called by the execution layer commit the block hash
    async fn recv_committed_block_info(&self, block_id: BlockId) -> Result<(), ExecError>;
}

pub struct ExecutionArgs {
    pub block_number_to_block_id: BTreeMap<u64, HashValue>,
}

#[derive(Clone)]
pub struct ExecutionLayer {
    pub execution_api: Arc<dyn ExecutionChannel>,
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
