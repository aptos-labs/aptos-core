// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    publishing::publish_util::PackageHandler, ReplayProtectionType, TransactionGenerator,
    TransactionGeneratorCreator,
};
use aptos_infallible::RwLock;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Arc;

pub struct PublishPackageGenerator {
    rng: StdRng,
    package_handler: Arc<RwLock<PackageHandler>>,
    txn_factory: TransactionFactory,
    replay_protection_type: ReplayProtectionType,
}

impl PublishPackageGenerator {
    pub fn new(
        rng: StdRng,
        package_handler: Arc<RwLock<PackageHandler>>,
        txn_factory: TransactionFactory,
        replay_protection_type: ReplayProtectionType,
    ) -> Self {
        Self {
            rng,
            package_handler,
            txn_factory,
            replay_protection_type,
        }
    }
}

impl TransactionGenerator for PublishPackageGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);

        // First publish the module and then use it
        let package = self
            .package_handler
            .write()
            .pick_package(&mut self.rng, account.address());

        for payload in package.publish_transaction_payload(&self.txn_factory.get_chain_id()) {
            let mut txn_builder = self.txn_factory.payload(payload);
            if let ReplayProtectionType::Nonce = self.replay_protection_type {
                txn_builder = txn_builder.upgrade_payload(true, true);
            }
            let txn = account.sign_with_transaction_builder(txn_builder);
            requests.push(txn);
        }
        // for _ in 1..num_to_create {
        //     let request = package.use_random_transaction(&mut self.rng, account, &self.txn_factory);
        //     requests.push(request);
        // }
        // republish
        // let package = self
        //     .package_handler
        //     .write()
        //     .pick_package(&mut self.rng, account.address());
        // let txn = package.publish_transaction(account, &self.txn_factory);
        // requests.push(txn);
        requests
    }
}

pub struct PublishPackageCreator {
    txn_factory: TransactionFactory,
    package_handler: Arc<RwLock<PackageHandler>>,
    replay_protection_type: ReplayProtectionType,
}

impl PublishPackageCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        package_handler: PackageHandler,
        replay_protection_type: ReplayProtectionType,
    ) -> Self {
        Self {
            txn_factory,
            package_handler: Arc::new(RwLock::new(package_handler)),
            replay_protection_type,
        }
    }
}

impl TransactionGeneratorCreator for PublishPackageCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(PublishPackageGenerator::new(
            StdRng::from_entropy(),
            self.package_handler.clone(),
            self.txn_factory.clone(),
            self.replay_protection_type,
        ))
    }
}
