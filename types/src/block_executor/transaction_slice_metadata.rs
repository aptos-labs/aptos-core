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
    /// Chunk execution, e.g., state sync or replay. Specifies the start (inclusive) and the end
    /// (exclusive) versions of a transaction slice.
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
        debug_assert!(
            begin < end,
            "Chunk must have non-negative size, but it has: {}-{}",
            begin,
            end
        );
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

    /// Returns the first transaction version for [TransactionSliceMetadata::Chunk], and [None]
    /// otherwise.
    pub fn begin_version(&self) -> Option<Version> {
        if let TransactionSliceMetadata::Chunk { begin, .. } = self {
            return Some(*begin);
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::assert_none;

    #[test]
    fn test_append_state_checkpoint_to_block() {
        assert_none!(TransactionSliceMetadata::unknown().append_state_checkpoint_to_block());
        assert_none!(TransactionSliceMetadata::chunk(1, 2).append_state_checkpoint_to_block());

        let metadata = TransactionSliceMetadata::block_from_u64(2, 3);
        assert_eq!(
            metadata.append_state_checkpoint_to_block(),
            Some(HashValue::from_u64(3))
        );
    }

    #[test]
    fn test_is_immediately_after() {
        let is_immediately_after = [
            (
                TransactionSliceMetadata::block_from_u64(2, 3),
                TransactionSliceMetadata::block_from_u64(3, 4),
            ),
            (
                TransactionSliceMetadata::chunk(0, 1),
                TransactionSliceMetadata::chunk(1, 2),
            ),
        ];

        for (fst, snd) in is_immediately_after {
            assert!(snd.is_immediately_after(&fst));
        }
    }

    #[test]
    fn test_is_not_immediately_after() {
        let is_not_immediately_after = [
            (
                TransactionSliceMetadata::unknown(),
                TransactionSliceMetadata::unknown(),
            ),
            (
                TransactionSliceMetadata::unknown(),
                TransactionSliceMetadata::block_from_u64(3, 4),
            ),
            (
                TransactionSliceMetadata::unknown(),
                TransactionSliceMetadata::chunk(3, 4),
            ),
            (
                TransactionSliceMetadata::block_from_u64(0, 1),
                TransactionSliceMetadata::unknown(),
            ),
            (
                TransactionSliceMetadata::block_from_u64(1, 2),
                TransactionSliceMetadata::block_from_u64(0, 1),
            ),
            (
                TransactionSliceMetadata::block_from_u64(1, 2),
                TransactionSliceMetadata::block_from_u64(1, 2),
            ),
            (
                TransactionSliceMetadata::block_from_u64(0, 1),
                TransactionSliceMetadata::chunk(2, 3),
            ),
            (
                TransactionSliceMetadata::chunk(0, 1),
                TransactionSliceMetadata::unknown(),
            ),
            (
                TransactionSliceMetadata::chunk(0, 1),
                TransactionSliceMetadata::block_from_u64(1, 2),
            ),
            (
                TransactionSliceMetadata::chunk(1, 2),
                TransactionSliceMetadata::chunk(0, 1),
            ),
            (
                TransactionSliceMetadata::chunk(1, 2),
                TransactionSliceMetadata::chunk(1, 2),
            ),
        ];

        for (fst, snd) in is_not_immediately_after {
            assert!(!snd.is_immediately_after(&fst));
        }
    }
}
