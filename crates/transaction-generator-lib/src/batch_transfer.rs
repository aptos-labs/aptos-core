// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ObjectPool, ReplayProtectionType, TransactionGenerator, TransactionGeneratorCreator};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
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
    replay_protection_type: ReplayProtectionType,
}

impl BatchTransferTransactionGenerator {
    pub fn new(
        rng: StdRng,
        batch_size: usize,
        send_amount: u64,
        txn_factory: TransactionFactory,
        all_addresses: Arc<ObjectPool<AccountAddress>>,
        replay_protection_type: ReplayProtectionType,
    ) -> Self {
        Self {
            rng,
            batch_size,
            send_amount,
            txn_factory,
            all_addresses,
            replay_protection_type,
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
            let mut txn_builder =
                self.txn_factory
                    .payload(aptos_stdlib::aptos_account_batch_transfer(receivers, vec![
                    self.send_amount;
                    self.batch_size
                ]));
            if let ReplayProtectionType::Nonce = self.replay_protection_type {
                txn_builder = txn_builder.upgrade_payload(true, true);
            }
            requests.push(account.sign_with_transaction_builder(txn_builder));
        }

        requests
    }
}

pub struct BatchTransferTransactionGeneratorCreator {
    txn_factory: TransactionFactory,
    amount: u64,
    all_addresses: Arc<ObjectPool<AccountAddress>>,
    batch_size: usize,
    replay_protection_type: ReplayProtectionType,
}

impl BatchTransferTransactionGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        amount: u64,
        all_addresses: Arc<ObjectPool<AccountAddress>>,
        batch_size: usize,
        replay_protection_type: ReplayProtectionType,
    ) -> Self {
        Self {
            txn_factory,
            amount,
            all_addresses,
            batch_size,
            replay_protection_type,
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
            self.replay_protection_type,
        ))
    }
}
