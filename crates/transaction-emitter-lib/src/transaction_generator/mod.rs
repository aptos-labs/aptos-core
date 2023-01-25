// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use std::sync::atomic::AtomicUsize;

pub mod account_generator;
pub mod call_custom_modules;
pub mod nft_mint_and_transfer;
pub mod p2p_transaction_generator;
pub mod publish_modules;
mod publishing;
pub mod transaction_mix_generator;
pub use publishing::module_simple::EntryPoints;

pub trait TransactionGenerator: Sync + Send {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction>;
}

#[async_trait]
pub trait TransactionGeneratorCreator: Sync + Send {
    async fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator>;
}

#[async_trait]
pub trait TransactionExecutor: Sync + Send {
    async fn get_account_balance(&self, account_address: AccountAddress) -> Result<u64>;

    async fn query_sequence_number(&self, account_address: AccountAddress) -> Result<u64>;

    async fn execute_transactions(&self, txns: &[SignedTransaction]) -> Result<()>;

    async fn execute_transactions_with_counter(
        &self,
        txns: &[SignedTransaction],
        failure_counter: &AtomicUsize,
    ) -> Result<()>;
}
