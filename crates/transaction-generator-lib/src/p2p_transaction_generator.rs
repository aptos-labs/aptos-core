// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{chain_id::ChainId, transaction::SignedTransaction, LocalAccount},
};
use rand::{distributions::{Distribution, Standard}, prelude::SliceRandom, rngs::StdRng, Rng, RngCore, SeedableRng, thread_rng};
use std::{cmp::max, sync::Arc};
use std::borrow::BorrowMut;

pub struct P2PTransactionGenerator {
    rng: StdRng,
    send_amount: u64,
    txn_factory: TransactionFactory,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    accounts: Vec<Arc<RwLock<LocalAccount>>>,
    invalid_transaction_ratio: usize,
}

impl P2PTransactionGenerator {
    pub fn new(
        rng: StdRng,
        send_amount: u64,
        txn_factory: TransactionFactory,
        accounts: Vec<Arc<RwLock<LocalAccount>>>,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        invalid_transaction_ratio: usize,
    ) -> Self {
        Self {
            rng,
            send_amount,
            txn_factory,
            all_addresses,
            accounts,
            invalid_transaction_ratio,
        }
    }

    fn gen_single_txn(
        &self,
        from: &mut LocalAccount,
        to: &AccountAddress,
        num_coins: u64,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        from.sign_with_transaction_builder(
            txn_factory.payload(aptos_stdlib::aptos_coin_transfer(*to, num_coins)),
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
                self.gen_single_txn(sender, receiver, self.send_amount, txn_factory)
            },
            InvalidTransactionType::Sender => self.gen_single_txn(
                &mut invalid_account,
                receiver,
                self.send_amount,
                &self.txn_factory,
            ),
            InvalidTransactionType::Receiver => self.gen_single_txn(
                sender,
                &invalid_address,
                self.send_amount,
                &self.txn_factory,
            ),
            InvalidTransactionType::Duplication => {
                // if this is the first tx, default to generate invalid tx with wrong chain id
                // otherwise, make a duplication of an exist valid tx
                if reqs.is_empty() {
                    let txn_factory = &self.txn_factory.clone().with_chain_id(ChainId::new(255));
                    self.gen_single_txn(sender, receiver, self.send_amount, txn_factory)
                } else {
                    let random_index = rng.gen_range(0, reqs.len());
                    reqs[random_index].clone()
                }
            },
        }
    }

    /// Generate a given number (`num_txns`) of transactions that satisfies some constraints.
    pub fn generate_block(&mut self, num_txns: usize, transfer_amount: u64, no_rw_conflict: bool, num_txns_per_sender: usize) -> Vec<(SignedTransaction, AccountAddress)> {
        let num_accounts = self.accounts().len();
        if no_rw_conflict {
            // We need `num_txns` distinct senders and `num_txns` distinct recipients.
            assert!(num_accounts >= num_txns * 2);
            let indices = rand::seq::index::sample(&mut self.rng, self.accounts.len(), num_txns * 2);
            let txns = (0..num_txns).map(|i| {
                let sender = self.accounts[indices.index(i)].write().borrow_mut();
                let recipient = self.accounts[indices.index(num_txns + i)].read().address();
                let txn = self.gen_single_txn(sender, &recipient, transfer_amount, &self.txn_factory);
                (txn, recipient)
            }).collect();
            txns
        } else {
            assert_eq!(0, num_txns % num_txns_per_sender);
            let num_senders = num_txns / num_txns_per_sender;
            assert!(num_accounts >= num_senders);
            let txns = (0..num_senders).flat_map(|_| {
                let sender_idx = self.rng.gen_range(0, num_accounts);
                let sender = self.accounts[sender_idx].write().borrow_mut();
                (0..num_txns_per_sender).map(||{
                    let recipient_idx = self.rng.gen_range(0, num_accounts);
                    let recipient = self.accounts[recipient_idx].read().address();
                    let txn = self.gen_single_txn(sender, &recipient, transfer_amount, &self.txn_factory);
                    (txn, recipient)
                })
            }).collect();
            txns
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
        account: &mut LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);
        let invalid_size = if self.invalid_transaction_ratio != 0 {
            // if enable mix invalid tx, at least 1 invalid tx per batch
            max(1, self.invalid_transaction_ratio / 100)
        } else {
            0
        };
        let mut num_valid_tx = num_to_create * (1 - invalid_size);

        let receivers = self
            .all_addresses
            .read()
            .choose_multiple(&mut self.rng, num_to_create)
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            receivers.len() >= num_to_create,
            "failed: {} >= {}",
            receivers.len(),
            num_to_create
        );
        for i in 0..num_to_create {
            let receiver = receivers.get(i).expect("all_addresses can't be empty");
            let request = if num_valid_tx > 0 {
                num_valid_tx -= 1;
                self.gen_single_txn(account, receiver, self.send_amount, &self.txn_factory)
            } else {
                self.generate_invalid_transaction(
                    &mut self.rng.clone(),
                    account,
                    receiver,
                    &requests,
                )
            };
            requests.push(request);
        }
        requests
    }
}

pub struct P2PTransactionGeneratorCreator {
    txn_factory: TransactionFactory,
    amount: u64,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    invalid_transaction_ratio: usize,
}

impl P2PTransactionGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        amount: u64,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        invalid_transaction_ratio: usize,
    ) -> Self {
        Self {
            txn_factory,
            amount,
            all_addresses,
            invalid_transaction_ratio,
        }
    }
}

impl TransactionGeneratorCreator for P2PTransactionGeneratorCreator {
    fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        Box::new(P2PTransactionGenerator::new(
            StdRng::from_entropy(),
            self.amount,
            self.txn_factory.clone(),
            vec![],
            self.all_addresses.clone(),
            self.invalid_transaction_ratio,
        ))
    }
}
