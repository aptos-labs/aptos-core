// Copyright © Aptos Foundation

use aptos_mvhashmap::types::TxnIndex;

/// In standard BlockSTM, the internal states assume contiguous transactions indices.
/// To also support sharded execution (where each shard gets non-contiguous transaction indices
/// and possibly waits for transaction results from other shards),
/// an index mapping is need as an additional input to our existing BlockSTM implementation.
#[derive(Clone)]
pub struct IndexMapping {
    /// A sorted list of transaction indices.
    indices: Vec<usize>,
    /// A TxnIndex -> local position mapping.
    /// Currently implemented as a `Vec` of size equal to the block size, assuming it's not too large.
    positions_by_index: Vec<usize>,
}

impl IndexMapping {
    /// Create Positions by TxnIndex mapping from the TxnIndex list.
    fn create_positions_by_index(block_size: usize, indices: &[usize]) -> Vec<usize> {
        let mut ret = vec![usize::MAX; block_size];
        for (pos, &index) in indices.iter().enumerate() {
            ret[index] = pos;
        }
        ret
    }

    pub fn new(indices: Vec<usize>, block_size: usize) -> Self {
        let positions_by_index = Self::create_positions_by_index(block_size, &indices);
        Self {
            indices,
            positions_by_index,
        }
    }

    pub fn new_unsharded(block_size: usize) -> Self {
        Self {
            indices: (0..block_size).collect(),
            positions_by_index: (0..block_size).collect(),
        }
    }

    pub fn is_end_index(&self, index: TxnIndex) -> bool {
        index as usize == self.positions_by_index.len()
    }

    pub fn is_last_index(&self, index: TxnIndex) -> bool {
        self.index(self.num_txns() - 1) == index
    }

    pub fn next_index(&self, index: TxnIndex) -> TxnIndex {
        if self.is_end_index(index) || self.is_last_index(index) {
            self.end_index()
        } else {
            let pos = self.position_by_index(index).unwrap();
            self.index(pos + 1)
        }
    }

    pub fn num_txns(&self) -> usize {
        self.indices.len()
    }

    pub fn iter_txn_indices(&self) -> Box<dyn Iterator<Item = TxnIndex> + '_> {
        Box::new(self.indices.iter().map(|&i| i as TxnIndex))
    }

    pub fn position_by_index(&self, index: TxnIndex) -> Option<usize> {
        let index = index as usize;
        if index >= self.positions_by_index.len() {
            None
        } else {
            Some(self.positions_by_index[index])
        }
    }

    pub fn end_index(&self) -> TxnIndex {
        self.positions_by_index.len() as TxnIndex
    }

    pub fn index(&self, i: usize) -> TxnIndex {
        self.indices[i] as TxnIndex
    }
}
