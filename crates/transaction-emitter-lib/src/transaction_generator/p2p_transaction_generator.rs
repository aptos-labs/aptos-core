// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::transaction_generator::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{chain_id::ChainId, transaction::SignedTransaction, LocalAccount},
};
use rand::{
    distributions::{Distribution, Standard},
    prelude::{SliceRandom, StdRng},
    Rng,
};
use rand_core::RngCore;
use std::{cmp::max, sync::Arc};

pub struct P2PTransactionGenerator {
    rng: StdRng,
    send_amount: u64,
    txn_factory: TransactionFactory,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    invalid_transaction_ratio: usize,
    gas_price: u64,
}

impl P2PTransactionGenerator {
    pub fn new(
        rng: StdRng,
        send_amount: u64,
        txn_factory: TransactionFactory,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        invalid_transaction_ratio: usize,
        gas_price: u64,
    ) -> Self {
        Self {
            rng,
            send_amount,
            txn_factory,
            all_addresses,
            invalid_transaction_ratio,
            gas_price,
        }
    }

    fn gen_single_txn(
        &self,
        from: &mut LocalAccount,
        to: &AccountAddress,
        num_coins: u64,
        txn_factory: &TransactionFactory,
        gas_price: u64,
    ) -> SignedTransaction {
        from.sign_with_transaction_builder(
            txn_factory
                .payload(aptos_stdlib::aptos_coin_transfer(*to, num_coins))
                .gas_unit_price(gas_price),
        )
    }

    fn generate_invalid_transaction(
        &self,
        rng: &mut StdRng,
        sender: &mut LocalAccount,
        receiver: &AccountAddress,
        reqs: &[SignedTransaction],
    ) -> SignedTransaction {
        let mut invalid_account = LocalAccount::generate(rng);
        let invalid_address = invalid_account.address();
        match Standard.sample(rng) {
            InvalidTransactionType::ChainId => {
                let txn_factory = &self.txn_factory.clone().with_chain_id(ChainId::new(255));
                self.gen_single_txn(
                    sender,
                    receiver,
                    self.send_amount,
                    txn_factory,
                    self.gas_price,
                )
            }
            InvalidTransactionType::Sender => self.gen_single_txn(
                &mut invalid_account,
                receiver,
                self.send_amount,
                &self.txn_factory,
                self.gas_price,
            ),
            InvalidTransactionType::Receiver => self.gen_single_txn(
                sender,
                &invalid_address,
                self.send_amount,
                &self.txn_factory,
                self.gas_price,
            ),
            InvalidTransactionType::Duplication => {
                // if this is the first tx, default to generate invalid tx with wrong chain id
                // otherwise, make a duplication of an exist valid tx
                if reqs.is_empty() {
                    let txn_factory = &self.txn_factory.clone().with_chain_id(ChainId::new(255));
                    self.gen_single_txn(
                        sender,
                        receiver,
                        self.send_amount,
                        txn_factory,
                        self.gas_price,
                    )
                } else {
                    let random_index = rng.gen_range(0, reqs.len());
                    reqs[random_index].clone()
                }
            }
        }
    }
}

#[derive(Debug)]
enum InvalidTransactionType {
    /// invalid tx with wrong chain id
    ChainId,
    /// invalid tx with sender not on chain
    Sender,
    /// invalid tx with receiver not on chain
    Receiver,
    /// duplicate an exist tx
    Duplication,
}

impl Distribution<InvalidTransactionType> for Standard {
    fn sample<R: RngCore + ?Sized>(&self, rng: &mut R) -> InvalidTransactionType {
        match rng.gen_range(0, 4) {
            0 => InvalidTransactionType::ChainId,
            1 => InvalidTransactionType::Sender,
            2 => InvalidTransactionType::Receiver,
            _ => InvalidTransactionType::Duplication,
        }
    }
}

impl TransactionGenerator for P2PTransactionGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len() * transactions_per_account);
        let invalid_size = if self.invalid_transaction_ratio != 0 {
            // if enable mix invalid tx, at least 1 invalid tx per batch
            max(1, accounts.len() * self.invalid_transaction_ratio / 100)
        } else {
            0
        };
        let mut num_valid_tx = transactions_per_account * (accounts.len() - invalid_size);
        for sender in accounts {
            let receivers = self
                .all_addresses
                .read()
                .choose_multiple(&mut self.rng, transactions_per_account)
                .cloned()
                .collect::<Vec<_>>();
            for i in 0..transactions_per_account {
                let receiver = receivers.get(i).expect("all_addresses can't be empty");
                let request = if num_valid_tx > 0 {
                    num_valid_tx -= 1;
                    self.gen_single_txn(
                        sender,
                        receiver,
                        self.send_amount,
                        &self.txn_factory,
                        self.gas_price,
                    )
                } else {
                    self.generate_invalid_transaction(
                        &mut self.rng.clone(),
                        sender,
                        receiver,
                        &requests,
                    )
                };
                requests.push(request);
            }
        }
        requests
    }
}

pub struct P2PTransactionGeneratorCreator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    amount: u64,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    invalid_transaction_ratio: usize,
    gas_price: u64,
}

impl P2PTransactionGeneratorCreator {
    pub fn new(
        rng: StdRng,
        txn_factory: TransactionFactory,
        amount: u64,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        invalid_transaction_ratio: usize,
        gas_price: u64,
    ) -> Self {
        Self {
            rng,
            txn_factory,
            amount,
            all_addresses,
            invalid_transaction_ratio,
            gas_price,
        }
    }
}

impl TransactionGeneratorCreator for P2PTransactionGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(P2PTransactionGenerator::new(
            self.rng.clone(),
            self.amount,
            self.txn_factory.clone(),
            self.all_addresses.clone(),
            self.invalid_transaction_ratio,
            self.gas_price,
        ))
    }
}
