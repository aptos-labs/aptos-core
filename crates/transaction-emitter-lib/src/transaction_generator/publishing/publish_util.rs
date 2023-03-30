// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::module_simple::EntryPoints;
use crate::transaction_generator::publishing::module_simple;
use aptos_framework::natives::code::PackageMetadata;
use aptos_sdk::{
    bcs,
    move_types::identifier::Identifier,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{account_address::AccountAddress, transaction::SignedTransaction, LocalAccount},
};
use move_binary_format::{access::ModuleAccess, CompiledModule};
use rand::{rngs::StdRng, Rng};

// Information used to track a publisher and what allows to identify and
// version the package published.
#[derive(Clone, Debug)]
struct PackageInfo {
    publisher: AccountAddress,
    suffix: u64,
    fn_count: usize,
    // TODO: do we need upgrade number? it seems to be assigned by the system
    // upgrade_number: u64,
}

// Given a Package, track all publishers.
#[derive(Clone, Debug)]
struct PackageTracker {
    publishers: Vec<PackageInfo>,
    suffix: u64,
    package: Package,
}

impl PackageTracker {
    fn find_info(&self, publisher: &AccountAddress) -> Option<usize> {
        self.publishers
            .iter()
            .position(|info| &info.publisher == publisher)
    }
}

// Holds all the packages known and return a proper Package to be used.
#[derive(Clone, Debug)]
pub struct PackageHandler {
    packages: Vec<PackageTracker>,
}

impl PackageHandler {
    pub fn new() -> Self {
        let packages = vec![PackageTracker {
            publishers: vec![],
            suffix: 0,
            package: Package::simple(),
        }];
        PackageHandler { packages }
    }

    // Return a `Package` to be published. Packages are tracked by publisher so if
    // the same `LocalAccount` is used, the package will be an upgrade of the existing one
    // otherwise a "new" package will be generated (new suffix)
    pub fn pick_package(&mut self, rng: &mut StdRng, publisher: &mut LocalAccount) -> Package {
        let idx = rng.gen_range(0usize, self.packages.len());
        let tracker = self
            .packages
            .get_mut(idx)
            .expect("PackageTracker must exisit");
        let publisher_address = publisher.address();
        let (idx, version) = match tracker.find_info(&publisher_address) {
            Some(idx) => (idx, true),
            None => {
                let fn_count = rng.gen_range(0usize, 30usize);
                tracker.publishers.push(PackageInfo {
                    publisher: publisher_address,
                    suffix: tracker.suffix,
                    fn_count,
                });
                tracker.suffix += 1;
                (tracker.publishers.len() - 1, false)
            },
        };
        let mut package = tracker.package.update(
            tracker.publishers[idx].publisher,
            tracker.publishers[idx].suffix,
        );
        if version {
            package.version(rng);
        }
        package.scramble(tracker.publishers[idx].fn_count, rng);
        // info!("PACKAGE: {:#?}", package);
        package
    }
}

// Enum to define all packages known to the publisher code.
#[derive(Clone, Debug)]
pub enum Package {
    Simple(Vec<CompiledModule>, PackageMetadata),
}

impl Package {
    pub fn simple() -> Self {
        let (modules, metadata) = module_simple::load_package();
        Self::Simple(modules, metadata)
    }

    // Given an "original" package, updates all modules with the given publisher.
    pub fn update(&self, publisher: AccountAddress, suffix: u64) -> Self {
        match self {
            Self::Simple(modules, metadata) => {
                let (new_modules, metadata) = update(modules, metadata, publisher, suffix);
                Self::Simple(new_modules, metadata)
            },
        }
    }

    // Change package "version"
    pub fn version(&mut self, rng: &mut StdRng) {
        match self {
            Self::Simple(modules, _) => {
                module_simple::version(&mut modules[0], rng);
            },
        }
    }

    // Scrambles the package, passing a function count for the functions that can
    // be duplicated and a `StdRng` to generate random values
    pub fn scramble(&mut self, fn_count: usize, rng: &mut StdRng) {
        match self {
            Self::Simple(modules, _) => {
                module_simple::scramble(&mut modules[0], fn_count, rng);
            },
        }
    }

    // Return a transaction to publish the current package
    pub fn publish_transaction(
        &self,
        publisher: &mut LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        match self {
            Self::Simple(modules, metadata) => {
                publish_transaction(txn_factory, publisher, modules, metadata)
            },
        }
    }

    // Return a transaction to use the current package
    pub fn use_random_transaction(
        &self,
        rng: &mut StdRng,
        account: &mut LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        match self {
            Self::Simple(modules, _) => {
                let module_id = modules[0].self_id();
                // let payload = module_simple::rand_gen_function(rng, module_id);
                let payload = module_simple::rand_simple_function(rng, module_id);
                account.sign_with_transaction_builder(txn_factory.payload(payload))
            },
        }
    }

    pub fn use_specific_transaction(
        &self,
        fun: EntryPoints,
        account: &mut LocalAccount,
        txn_factory: &TransactionFactory,
        rng: Option<&mut StdRng>,
        other: Option<AccountAddress>,
    ) -> SignedTransaction {
        match self {
            Self::Simple(modules, _) => {
                let module_id = modules[0].self_id();
                let payload = fun.create_payload(module_id, rng, other);
                account.sign_with_transaction_builder(txn_factory.payload(payload))
            },
        }
    }
}

fn update(
    modules: &[CompiledModule],
    metadata: &PackageMetadata,
    publisher: AccountAddress,
    suffix: u64,
) -> (Vec<CompiledModule>, PackageMetadata) {
    let mut new_modules = vec![];
    for module in modules {
        let mut new_module = module.clone();
        let module_handle = new_module
            .module_handles
            .get(module.self_handle_idx().0 as usize)
            .expect("ModuleId for self must exists");
        let _ = std::mem::replace(
            &mut new_module.address_identifiers[module_handle.address.0 as usize],
            publisher,
        );
        let mut new_name = new_module.identifiers[module_handle.name.0 as usize].to_string();
        new_name.push_str(suffix.to_string().as_str());
        let _ = std::mem::replace(
            &mut new_module.identifiers[module_handle.name.0 as usize],
            Identifier::new(new_name).expect("Identifier must be legal"),
        );
        new_modules.push(new_module);
    }
    let mut metadata = metadata.clone();
    for module in &mut metadata.modules {
        let mut new_name = module.name.clone();
        new_name.push_str(suffix.to_string().as_str());
        module.name = new_name;
    }
    (new_modules, metadata)
}

fn publish_transaction(
    txn_factory: &TransactionFactory,
    publisher: &mut LocalAccount,
    modules: &[CompiledModule],
    metadata: &PackageMetadata,
) -> SignedTransaction {
    let metadata = bcs::to_bytes(metadata).expect("PackageMetadata must serialize");
    let mut code: Vec<Vec<u8>> = vec![];
    for module in modules {
        let mut module_code: Vec<u8> = vec![];
        module
            .serialize(&mut module_code)
            .expect("Module must serialize");
        code.push(module_code);
    }
    let payload = aptos_stdlib::code_publish_package_txn(metadata, code);
    publisher.sign_with_transaction_builder(txn_factory.payload(payload))
}
