use serde::{Deserialize, Serialize};
use rand::Rng;
use std::{fmt, sync::Arc};
use hex;

#[derive(Clone, Deserialize, Serialize, Hash, PartialEq, Eq, Copy)]
pub struct TxnStatus {
    pub txn_hash: [u8; 32],
    pub nonce: u64,
    pub sender: [u8; 32],
    pub is_discarded: bool,
}

#[derive(Clone, Deserialize, Serialize, Hash, PartialEq, Eq, Default)]
pub struct ComputeRes {
    pub data: [u8; 32],
    // todo(gravity_byteyue): Refactor to TxnInfo when refactoring
    pub txn_num: u64,
    pub txn_status: Arc<Option<Vec<TxnStatus>>>,
}

impl fmt::Display for ComputeRes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ComputeRes({}, txn_num: {})", hex::encode(self.data), self.txn_num)
    }
}

impl fmt::Debug for ComputeRes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ComputeRes({}, txn_num: {})", hex::encode(self.data), self.txn_num)
    }
}

impl ComputeRes {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 32] = rng.gen();
        let txn_num = rng.gen();
        Self { data: random_bytes, txn_num, txn_status: Arc::new(None) }
    }

    pub fn new(data: [u8; 32], txn_num: u64, txn_status: Vec<TxnStatus>) -> Self {
        Self { data, txn_num, txn_status: Arc::new(Some(txn_status)) }
    }

    pub fn bytes(&self) -> [u8; 32] {
        self.data
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn txn_num(&self) -> u64 {
        self.txn_num
    }
}
