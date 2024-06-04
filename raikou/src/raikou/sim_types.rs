use crate::{framework::NodeId, raikou::types::BatchSN};
use bitvec::prelude::BitVec;
use std::sync::Arc;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct BatchInfo {
    pub node: NodeId,
    pub sn: BatchSN,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AC {
    // In practice, this would be a hash pointer.
    pub batch: BatchInfo,
    pub signers: BitVec,
}

#[derive(Clone)]
pub struct BlockPayload {
    inner: Arc<BlockPayloadInner>,
}

struct BlockPayloadInner {
    acs: Vec<AC>,
    batches: Vec<BatchInfo>,
}

impl BlockPayload {
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
