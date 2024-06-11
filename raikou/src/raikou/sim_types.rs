use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use crate::framework::NodeId;
use bitvec::prelude::BitVec;
use std::sync::Arc;

pub type HashValue = u64;

pub type BatchId = i64;

#[derive(Clone)]
pub struct BatchInfo {
    pub author: NodeId,
    pub batch_id: BatchId,
    pub digest: HashValue,
}

pub fn hash(x: impl Hash) -> HashValue {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    x.hash(&mut hasher);
    hasher.finish()
}

impl Debug for crate::raikou::types::BatchInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ node: {}, sn: {}, hash: {:#x} }}", self.author, self.batch_id, self.digest)
    }
}

#[derive(Clone)]
pub struct AC {
    // In practice, this would be a hash pointer.
    pub batch: BatchInfo,
    pub signers: BitVec,
}

#[derive(Clone)]
pub struct Payload {
    inner: Arc<BlockPayloadInner>,
}

struct BlockPayloadInner {
    acs: Vec<AC>,
    batches: Vec<BatchInfo>,
}

impl Payload {
    pub fn new(acs: Vec<AC>, batches: Vec<BatchInfo>) -> Self {
        Self {
            inner: Arc::new(BlockPayloadInner { acs, batches }),
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![])
    }

    pub fn acs(&self) -> &Vec<AC> {
        &self.inner.acs
    }

    pub fn batches(&self) -> &Vec<BatchInfo> {
        &self.inner.batches
    }

    pub fn all(&self) -> impl Iterator<Item = &BatchInfo> {
        self.inner
            .acs
            .iter()
            .map(|ac| &ac.batch)
            .chain(self.inner.batches.iter())
    }
}
