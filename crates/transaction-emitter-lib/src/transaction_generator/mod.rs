// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use rand::rngs::StdRng;
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

    fn gen_single_txn(
        &self,
        _from: &mut LocalAccount,
        _to: &AccountAddress,
        _num_coins: u64,
        _txn_factory: &TransactionFactory,
        _gas_price: u64,
    ) -> SignedTransaction {
        unimplemented!()
    }

    fn generate_invalid_transaction(
        &self,
        _rng: &mut StdRng,
        _sender: &mut LocalAccount,
        _receiver: &AccountAddress,
        _gas_price: u64,
        _reqs: &[SignedTransaction],
    ) -> SignedTransaction {
        unimplemented!()
    }
}
