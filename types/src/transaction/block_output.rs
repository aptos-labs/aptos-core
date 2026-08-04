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
        }
    }

    pub fn into_transaction_outputs_forced(self) -> Vec<Output> {
        self.transaction_outputs
    }

    pub fn into_inner(self) -> (Vec<Output>, Option<T>) {
        (self.transaction_outputs, self.block_epilogue_txn)
    }

    pub fn get_transaction_outputs_forced(&self) -> &[Output] {
        &self.transaction_outputs
    }
}
