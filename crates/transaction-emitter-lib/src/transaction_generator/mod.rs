// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_sdk::types::{transaction::SignedTransaction, LocalAccount};

pub mod account_generator;
pub mod nft_mint;
pub mod p2p_transaction_generator;
pub mod transaction_mix_generator;

pub trait TransactionGenerator: Sync + Send {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction>;
}

pub trait TransactionGeneratorCreator: Sync + Send {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator>;
}
