// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ObjectPool, TransactionGenerator, TransactionGeneratorCreator};
use velor_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{velor_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Arc;

pub struct BatchTransferTransactionGenerator {
    rng: StdRng,
    batch_size: usize,
    send_amount: u64,
    txn_factory: TransactionFactory,
    all_addresses: Arc<ObjectPool<AccountAddress>>,
}

impl BatchTransferTransactionGenerator {
    pub fn new(
        rng: StdRng,
        batch_size: usize,
        send_amount: u64,
        txn_factory: TransactionFactory,
        all_addresses: Arc<ObjectPool<AccountAddress>>,
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
                .clone_from_pool(self.batch_size, &mut self.rng);
            requests.push(
                account.sign_with_transaction_builder(self.txn_factory.payload(
                    velor_stdlib::velor_account_batch_transfer(receivers, vec![
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
    all_addresses: Arc<ObjectPool<AccountAddress>>,
    batch_size: usize,
}

impl BatchTransferTransactionGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        amount: u64,
        all_addresses: Arc<ObjectPool<AccountAddress>>,
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
