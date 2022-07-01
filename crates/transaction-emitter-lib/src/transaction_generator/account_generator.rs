// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::transaction_generator::TransactionGenerator;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        transaction::{
            authenticator::{AuthenticationKey, AuthenticationKeyPreimage},
            SignedTransaction,
        },
        LocalAccount,
    },
};
use rand::prelude::{SliceRandom, StdRng};
use std::{fmt::Debug, sync::Arc};

#[derive(Clone, Debug)]
pub struct AccountGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
}

impl AccountGenerator {
    pub fn new(rng: StdRng, txn_factory: TransactionFactory) -> Self {
        Self { rng, txn_factory }
    }
}

impl TransactionGenerator for AccountGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        all_addresses: Arc<Vec<AccountAddress>>,
        _invalid_transaction_ratio: usize,
        gas_price: u64,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len());
        for account in accounts {
            let receiver = all_addresses
                .choose(&mut self.rng)
                .expect("all_addresses can't be empty");
            let request = self.gen_single_txn(account, receiver, 0, &self.txn_factory, gas_price);
            requests.push(request);
        }
        requests
    }

    fn gen_single_txn(
        &self,
        from: &mut LocalAccount,
        _to: &AccountAddress,
        _num_coins: u64,
        txn_factory: &TransactionFactory,
        gas_price: u64,
    ) -> SignedTransaction {
        let preimage = AuthenticationKeyPreimage::ed25519(from.public_key());
        let auth_key = AuthenticationKey::from_preimage(&preimage);
        from.sign_with_transaction_builder(
            txn_factory
                .payload(aptos_stdlib::encode_account_create_account(
                    auth_key.derived_address(),
                ))
                .gas_unit_price(gas_price),
        )
    }

    // TODO(skedia): Add support for this.
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
