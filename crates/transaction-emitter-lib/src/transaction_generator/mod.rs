// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_sdk::{
    move_types::account_address::AccountAddress,
    types::{transaction::SignedTransaction, LocalAccount},
};
use std::{fmt::Debug, sync::Arc};

pub mod account_generator;
pub mod nft_mint;
pub mod p2p_transaction_generator;

pub trait TransactionGenerator: Debug + Sync + Send {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        all_addresses: Arc<Vec<AccountAddress>>,
        invalid_transaction_ratio: usize,
        gas_price: u64,
    ) -> Vec<SignedTransaction>;
}

pub trait TransactionGeneratorCreator: Debug {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator>;
}
