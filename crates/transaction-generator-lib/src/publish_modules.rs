// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    publishing::{module_simple::PreBuiltPackagesImpl, publish_util::PackageHandler},
    TransactionGenerator, TransactionGeneratorCreator,
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
}

impl PublishPackageGenerator {
    pub fn new(
        rng: StdRng,
        package_handler: Arc<RwLock<PackageHandler>>,
        txn_factory: TransactionFactory,
    ) -> Self {
        Self {
            rng,
            package_handler,
            txn_factory,
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
        let txn = account.sign_with_transaction_builder(
            self.txn_factory
                .payload(package.publish_transaction_payload()),
        );
        requests.push(txn);
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
}

impl PublishPackageCreator {
    pub fn new(txn_factory: TransactionFactory) -> Self {
        Self {
            txn_factory,
            package_handler: Arc::new(RwLock::new(PackageHandler::new(
                &PreBuiltPackagesImpl,
                "simple",
            ))),
        }
    }
}

impl TransactionGeneratorCreator for PublishPackageCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(PublishPackageGenerator::new(
            StdRng::from_entropy(),
            self.package_handler.clone(),
            self.txn_factory.clone(),
        ))
    }
}
