// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_rest_client::Client;
use aptos_sdk::types::{transaction::SignedTransaction, LocalAccount};
use async_trait::async_trait;

pub mod account_generator;
pub mod nft_mint_and_transfer;
pub mod p2p_transaction_generator;
pub mod transaction_mix_generator;

#[async_trait]
pub trait TransactionGenerator: Sync + Send {
    async fn setup(&mut self, _client: &Client, _accounts: &[LocalAccount]) {
        // default implementation is empty.
    }

    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction>;
}

pub trait TransactionGeneratorCreator: Sync + Send {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator>;
}
