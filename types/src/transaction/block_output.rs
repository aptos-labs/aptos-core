// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::BlockExecutableTransaction;
use move_core_types::language_storage::ModuleId;
use std::fmt::Debug;

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
    // Modules published by the block epilogue (deferred module publishing). Recorded so the
    // global module cache can be invalidated for these modules at commit time. Empty when the
    // feature is disabled or no modules were published.
    published_modules: Vec<ModuleId>,
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
            published_modules: vec![],
        }
    }

    pub fn with_published_modules(mut self, published_modules: Vec<ModuleId>) -> Self {
        self.published_modules = published_modules;
        self
    }

    pub fn published_modules(&self) -> &[ModuleId] {
        &self.published_modules
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
