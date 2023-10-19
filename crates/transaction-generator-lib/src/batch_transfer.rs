// Copyright © Aptos Foundation

use crate::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::sync::Arc;

pub struct BatchTransferTransactionGenerator {
    rng: StdRng,
    batch_size: usize,
    send_amount: u64,
    txn_factory: TransactionFactory,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
}

impl BatchTransferTransactionGenerator {
    pub fn new(
        rng: StdRng,
        batch_size: usize,
        send_amount: u64,
        txn_factory: TransactionFactory,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    ) -> Self {
        Self {
            rng,
            batch_size,
            send_amount,
            txn_factory,
            all_addresses,
        }
    }
}

impl TransactionGenerator for BatchTransferTransactionGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);
        for _ in 0..num_to_create {
            let receivers = self
                .all_addresses
                .read()
                .choose_multiple(&mut self.rng, self.batch_size)
                .cloned()
                .collect::<Vec<_>>();
            requests.push(
                account.sign_with_transaction_builder(self.txn_factory.payload(
                    aptos_stdlib::aptos_account_batch_transfer(receivers, vec![
                        self.send_amount;
                        self.batch_size
                    ]),
                )),
            );
        }

        requests
    }
}

pub struct BatchTransferTransactionGeneratorCreator {
    txn_factory: TransactionFactory,
    amount: u64,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    batch_size: usize,
}

impl BatchTransferTransactionGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        amount: u64,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        batch_size: usize,
    ) -> Self {
        Self {
            txn_factory,
            amount,
            all_addresses,
            batch_size,
        }
    }
}

impl TransactionGeneratorCreator for BatchTransferTransactionGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(BatchTransferTransactionGenerator::new(
            StdRng::from_entropy(),
            self.batch_size,
            self.amount,
            self.txn_factory.clone(),
            self.all_addresses.clone(),
        ))
    }
}
