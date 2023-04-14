// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_executable_store::ExecutableStore;
use aptos_executor::{
    block_executor::TransactionBlockExecutor, components::chunk_output::ChunkOutput,
};
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::{
    account_address::AccountAddress, executable::ExecutableTestType,
    state_store::state_key::StateKey, transaction::Transaction,
};
use aptos_vm::AptosVM;
use std::sync::Arc;

pub struct TransferInfo {
    pub sender: AccountAddress,
    pub receiver: AccountAddress,
    pub amount: u64,
}

pub struct AccountCreationInfo {
    pub sender: AccountAddress,
    pub new_account: AccountAddress,
    pub initial_balance: u64,
}

pub enum ExtraInfo {
    TransferInfo(TransferInfo),
    AccountCreationInfo(AccountCreationInfo),
}

pub struct BenchmarkTransaction {
    pub transaction: Transaction,
    pub extra_info: Option<ExtraInfo>,
}

impl TransferInfo {
    pub fn new(sender: AccountAddress, receiver: AccountAddress, amount: u64) -> Self {
        Self {
            sender,
            receiver,
            amount,
        }
    }
}

impl AccountCreationInfo {
    pub fn new(sender: AccountAddress, new_account: AccountAddress, initial_balance: u64) -> Self {
        Self {
            sender,
            new_account,
            initial_balance,
        }
    }
}

impl BenchmarkTransaction {
    pub fn new(transaction: Transaction, extra_info: ExtraInfo) -> Self {
        Self {
            transaction,
            extra_info: Some(extra_info),
        }
    }
}

impl From<Transaction> for BenchmarkTransaction {
    fn from(transaction: Transaction) -> Self {
        Self {
            transaction,
            extra_info: None,
        }
    }
}

impl TransactionBlockExecutor<BenchmarkTransaction> for AptosVM {
    fn execute_transaction_block(
        transactions: Vec<BenchmarkTransaction>,
        state_view: CachedStateView,
        executable_cache: Arc<ExecutableStore<StateKey, ExecutableTestType>>,
    ) -> Result<ChunkOutput> {
        AptosVM::execute_transaction_block(
            transactions
                .into_iter()
                .map(|txn| txn.transaction)
                .collect(),
            state_view,
            executable_cache,
        )
    }
}
