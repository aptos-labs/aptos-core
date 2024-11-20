// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::Version;
use aptos_crypto::HashValue;

/// Specifies the kind of transactions for the block executor.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TransactionSliceMetadata {
    /// Block execution. Specifies the parent (executed) block, and the child (to be executed)
    /// block.
    Block { parent: HashValue, child: HashValue },
    /// Chunk execution, e.g., state sync or replay. Specifies the start and the end versions of
    /// transaction slice.
    Chunk { begin: Version, end: Version },
    /// The origin of transactions is not known, e.g., running a test.
    Unknown,
}

impl TransactionSliceMetadata {
    pub fn unknown() -> Self {
        Self::Unknown
    }

    pub fn block(parent: HashValue, child: HashValue) -> Self {
        Self::Block { parent, child }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn block_from_u64(parent: u64, child: u64) -> Self {
        Self::Block {
            parent: HashValue::from_u64(parent),
            child: HashValue::from_u64(child),
        }
    }

    pub fn chunk(begin: Version, end: Version) -> Self {
        Self::Chunk { begin, end }
    }

    /// Returns the hash of the block where to append the state checkpoint (i.e., the current hash
    /// of [TransactionSliceMetadata::Block]). For other variants, returns [None].
    pub fn append_state_checkpoint_to_block(&self) -> Option<HashValue> {
        use TransactionSliceMetadata::*;

        match self {
            Unknown => None,
            Block { child, .. } => Some(*child),
            Chunk { .. } => None,
        }
    }

    /// Returns true if transaction slice immediately follows the previous one. That is, if:
    ///   1. Both are [TransactionSliceMetadata::Block] and the previous child is equal to the
    ///      current parent.
    ///   2. Both are [TransactionSliceMetadata::Chunk] and the previous end version is equal to
    ///      the current start version.
    pub fn is_immediately_after(&self, previous: &TransactionSliceMetadata) -> bool {
        use TransactionSliceMetadata::*;

        match (previous, self) {
            (Unknown, Unknown)
            | (Unknown, Block { .. })
            | (Unknown, Chunk { .. })
            | (Block { .. }, Unknown)
            | (Block { .. }, Chunk { .. })
            | (Chunk { .. }, Unknown)
            | (Chunk { .. }, Block { .. }) => false,
            (Block { child, .. }, Block { parent, .. }) => parent == child,
            (Chunk { end, .. }, Chunk { begin, .. }) => begin == end,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_append_state_checkpoint_to_block() {
        assert!(TransactionSliceMetadata::unknown()
            .append_state_checkpoint_to_block()
            .is_none());
        assert!(TransactionSliceMetadata::chunk(1, 2)
            .append_state_checkpoint_to_block()
            .is_none());

        let parent = HashValue::from_u64(2);
        let child = HashValue::from_u64(3);
        let execution_state = TransactionSliceMetadata::block(parent, child);
        assert_eq!(
            execution_state.append_state_checkpoint_to_block(),
            Some(child)
        );
    }

    #[test]
    fn test_is_immediately_after() {
        let fst = TransactionSliceMetadata::block(HashValue::from_u64(2), HashValue::from_u64(3));
        let snd = TransactionSliceMetadata::block(HashValue::from_u64(3), HashValue::from_u64(4));
        assert!(snd.is_immediately_after(&fst));

        let fst = TransactionSliceMetadata::block(HashValue::from_u64(2), HashValue::from_u64(3));
        let snd = TransactionSliceMetadata::block(HashValue::from_u64(4), HashValue::from_u64(5));
        assert!(!snd.is_immediately_after(&fst));
    }
}
