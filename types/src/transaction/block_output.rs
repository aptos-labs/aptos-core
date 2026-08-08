// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::BlockExecutableTransaction;
use crate::error::PanicError;
use std::fmt::{self, Debug, Display};

/// Unrecoverable error that aborts execution of an entire block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockError {
    message: String,
}

impl BlockError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BlockError {}

impl From<PanicError> for BlockError {
    fn from(err: PanicError) -> Self {
        Self::new(err.to_string())
    }
}

/// Result of executing a block: either its output, or an unrecoverable [BlockError].
pub type BlockExecutionResult<T, Output> = Result<BlockOutput<T, Output>, BlockError>;

/// Opaque, forward-compatible description of what a block's outputs require for
/// cache coherence (e.g. module-cache invalidation when the block publishes
/// code). Extended with real payloads as the code-publish flow lands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheInvalidationInfo {
    /// Placeholder marker until real invalidation payloads exist.
    Dummy,
    // Future: published module ids / serialized blobs / granularity markers.
}

#[derive(Debug)]
pub struct BlockOutput<T, Output>
where
    T: BlockExecutableTransaction,
    Output: Debug,
{
    transaction_outputs: Vec<Output>,
    // A BlockEpilogueTxn might be appended to the block.
    // This field will be None iff the input is not a block, or an epoch change is triggered.
    block_epilogue_txn: Option<T>,
    // Side-band information for the cache coherence protocol. None means the block
    // requires no cache invalidation (the common case).
    cache_invalidation_info: Option<CacheInvalidationInfo>,
}

impl<T, Output> BlockOutput<T, Output>
where
    T: BlockExecutableTransaction,
    Output: Debug,
{
    pub fn new(transaction_outputs: Vec<Output>, block_epilogue_txn: Option<T>) -> Self {
        Self {
            transaction_outputs,
            block_epilogue_txn,
            cache_invalidation_info: None,
        }
    }

    pub fn new_with_cache_invalidation_info(
        transaction_outputs: Vec<Output>,
        block_epilogue_txn: Option<T>,
        cache_invalidation_info: Option<CacheInvalidationInfo>,
    ) -> Self {
        Self {
            transaction_outputs,
            block_epilogue_txn,
            cache_invalidation_info,
        }
    }

    pub fn into_transaction_outputs_forced(self) -> Vec<Output> {
        self.transaction_outputs
    }

    pub fn into_inner(self) -> (Vec<Output>, Option<T>, Option<CacheInvalidationInfo>) {
        (
            self.transaction_outputs,
            self.block_epilogue_txn,
            self.cache_invalidation_info,
        )
    }

    pub fn get_transaction_outputs_forced(&self) -> &[Output] {
        &self.transaction_outputs
    }

    pub fn cache_invalidation_info(&self) -> &Option<CacheInvalidationInfo> {
        &self.cache_invalidation_info
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::signature_verified_transaction::SignatureVerifiedTransaction;

    // `T` appears only as a type parameter; with `block_epilogue_txn = None` no
    // transaction is ever constructed, so a real transaction type suffices.
    type TestBlockOutput = BlockOutput<SignatureVerifiedTransaction, u64>;

    #[test]
    fn new_defaults_to_no_cache_invalidation_info() {
        let output = TestBlockOutput::new(vec![1, 2], None);
        assert!(output.cache_invalidation_info().is_none());

        let (outputs, epilogue, info) = output.into_inner();
        assert_eq!(outputs, vec![1, 2]);
        assert!(epilogue.is_none());
        assert!(info.is_none());
    }

    #[test]
    fn cache_invalidation_info_round_trips() {
        let output = TestBlockOutput::new_with_cache_invalidation_info(
            vec![1, 2],
            None,
            Some(CacheInvalidationInfo::Dummy),
        );
        assert_eq!(
            output.cache_invalidation_info(),
            &Some(CacheInvalidationInfo::Dummy)
        );

        let (_outputs, _epilogue, info) = output.into_inner();
        assert_eq!(info, Some(CacheInvalidationInfo::Dummy));
    }
}
