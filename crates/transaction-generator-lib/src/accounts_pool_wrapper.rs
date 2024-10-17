// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ObjectPool, TransactionGenerator, TransactionGeneratorCreator};
use aptos_sdk::types::{
    transaction::{SignedTransaction, TransactionPayload},
    LocalAccount,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
};

/// Wrapper that allows inner transaction generator to have unique accounts
/// for all transactions (instead of having 5-20 transactions per account, as default)
/// This is achieved via using accounts from the pool that account creatin can fill,
/// and burning (removing accounts from the pool) them - basically using them only once.
/// (we cannot use more as sequence number is not updated on failure)
pub struct AccountsPoolWrapperGenerator {
    rng: StdRng,
    generator: Box<dyn TransactionGenerator>,
    source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
    destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
}

impl AccountsPoolWrapperGenerator {
    pub fn new(
        rng: StdRng,
        generator: Box<dyn TransactionGenerator>,
        source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
        destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
    ) -> Self {
        Self {
            rng,
            generator,
            source_accounts_pool,
            destination_accounts_pool,
        }
    }
}

impl TransactionGenerator for AccountsPoolWrapperGenerator {
    fn generate_transactions(
        &mut self,
        _account: &LocalAccount,
        num_to_create: usize,
        _history: &[String],
        _market_maker: bool,
    ) -> Vec<SignedTransaction> {
        let mut accounts_to_use =
            self.source_accounts_pool
                .take_from_pool(num_to_create, true, &mut self.rng);
        if accounts_to_use.is_empty() {
            return Vec::new();
        }
        let txns = accounts_to_use
            .iter_mut()
            .flat_map(|account| {
                self.generator
                    .generate_transactions(account, 1, &Vec::new(), false)
            })
            .collect();
        if let Some(destination_accounts_pool) = &self.destination_accounts_pool {
            destination_accounts_pool.add_to_pool(accounts_to_use);
        }
        txns
    }
}

pub struct AccountsPoolWrapperCreator {
    creator: Box<dyn TransactionGeneratorCreator>,
    source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
    destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
}

impl AccountsPoolWrapperCreator {
    pub fn new(
        creator: Box<dyn TransactionGeneratorCreator>,
        source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
        destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
    ) -> Self {
        Self {
            creator,
            source_accounts_pool,
            destination_accounts_pool,
        }
    }
}

impl TransactionGeneratorCreator for AccountsPoolWrapperCreator {
    fn create_transaction_generator(
        &self,
        txn_counter: Arc<AtomicU64>,
    ) -> Box<dyn TransactionGenerator> {
        Box::new(AccountsPoolWrapperGenerator::new(
            StdRng::from_entropy(),
            self.creator.create_transaction_generator(txn_counter),
            self.source_accounts_pool.clone(),
            self.destination_accounts_pool.clone(),
        ))
    }
}

pub struct AddHistoryWrapperGenerator {
    rng: StdRng,
    source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
    destination_accounts_pool: Arc<ObjectPool<(LocalAccount, Vec<String>)>>,
}

impl AddHistoryWrapperGenerator {
    pub fn new(
        rng: StdRng,
        source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
        destination_accounts_pool: Arc<ObjectPool<(LocalAccount, Vec<String>)>>,
    ) -> Self {
        Self {
            rng,
            source_accounts_pool,
            destination_accounts_pool,
        }
    }
}

impl TransactionGenerator for AddHistoryWrapperGenerator {
    fn generate_transactions(
        &mut self,
        _account: &LocalAccount,
        _num_to_create: usize,
        _history: &[String],
        _market_maker: bool,
    ) -> Vec<SignedTransaction> {
        let length = self.source_accounts_pool.len();
        let all_source_accounts =
            self.source_accounts_pool
                .take_from_pool(length, true, &mut self.rng);
        self.destination_accounts_pool.add_to_pool(
            all_source_accounts
                .into_iter()
                .map(|account| (account, Vec::new()))
                .collect(),
        );
        vec![]
    }
}

pub struct AddHistoryWrapperCreator {
    source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
    destination_accounts_pool: Arc<ObjectPool<(LocalAccount, Vec<String>)>>,
}

impl AddHistoryWrapperCreator {
    pub fn new(
        source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
        destination_accounts_pool: Arc<ObjectPool<(LocalAccount, Vec<String>)>>,
    ) -> Self {
        Self {
            source_accounts_pool,
            destination_accounts_pool,
        }
    }
}

impl TransactionGeneratorCreator for AddHistoryWrapperCreator {
    fn create_transaction_generator(
        &self,
        _txn_counter: Arc<AtomicU64>,
    ) -> Box<dyn TransactionGenerator> {
        Box::new(AddHistoryWrapperGenerator::new(
            StdRng::from_entropy(),
            self.source_accounts_pool.clone(),
            self.destination_accounts_pool.clone(),
        ))
    }
}

pub struct MarketMakerPoolWrapperGenerator {
    rng: StdRng,
    generator: Box<dyn TransactionGenerator>,
    source_accounts_pool: Arc<ObjectPool<(LocalAccount, Vec<String>)>>,
    market_makers: Vec<(LocalAccount, Vec<String>)>,
}

impl MarketMakerPoolWrapperGenerator {
    pub fn new(
        rng: StdRng,
        generator: Box<dyn TransactionGenerator>,
        source_accounts_pool: Arc<ObjectPool<(LocalAccount, Vec<String>)>>,
    ) -> Self {
        Self {
            rng,
            generator,
            source_accounts_pool,
            market_makers: vec![],
        }
    }
}

impl TransactionGenerator for MarketMakerPoolWrapperGenerator {
    fn generate_transactions(
        &mut self,
        _account: &LocalAccount,
        _num_to_create: usize,
        _history: &[String],
        _market_maker: bool,
    ) -> Vec<SignedTransaction> {
        if self.market_makers.len() < 7 {
            self.market_makers =
                self.source_accounts_pool
                    .take_from_pool(7, true, &mut StdRng::from_entropy());
        }
        if self.rng.gen_range(0, 1000) < 939 {
            let rand = self.rng.gen_range(0, 1000);
            if rand < 476 {
                let txns = self.generator.generate_transactions(
                    &self.market_makers[0].0,
                    1,
                    &self.market_makers[0].1,
                    true,
                );
                for txn in txns.iter() {
                    if let TransactionPayload::EntryFunction(entry_function) = txn.payload() {
                        let function_name = entry_function.function().as_str();
                        self.market_makers[0].1.push(function_name.to_string());
                    }
                }
                return txns;
            } else if rand < 733 {
                let txns = self.generator.generate_transactions(
                    &self.market_makers[1].0,
                    1,
                    &self.market_makers[1].1,
                    true,
                );
                for txn in txns.iter() {
                    if let TransactionPayload::EntryFunction(entry_function) = txn.payload() {
                        let function_name = entry_function.function().as_str();
                        self.market_makers[1].1.push(function_name.to_string());
                    }
                }
                return txns;
            } else if rand < 818 {
                let txns = self.generator.generate_transactions(
                    &self.market_makers[2].0,
                    1,
                    &self.market_makers[2].1,
                    true,
                );
                for txn in txns.iter() {
                    if let TransactionPayload::EntryFunction(entry_function) = txn.payload() {
                        let function_name = entry_function.function().as_str();
                        self.market_makers[2].1.push(function_name.to_string());
                    }
                }
                return txns;
            } else if rand < 871 {
                let txns = self.generator.generate_transactions(
                    &self.market_makers[3].0,
                    1,
                    &self.market_makers[3].1,
                    true,
                );
                for txn in txns.iter() {
                    if let TransactionPayload::EntryFunction(entry_function) = txn.payload() {
                        let function_name = entry_function.function().as_str();
                        self.market_makers[3].1.push(function_name.to_string());
                    }
                }
                return txns;
            } else if rand < 920 {
                let txns = self.generator.generate_transactions(
                    &self.market_makers[4].0,
                    1,
                    &self.market_makers[4].1,
                    true,
                );
                for txn in txns.iter() {
                    if let TransactionPayload::EntryFunction(entry_function) = txn.payload() {
                        let function_name = entry_function.function().as_str();
                        self.market_makers[4].1.push(function_name.to_string());
                    }
                }
                return txns;
            } else if rand < 962 {
                let txns = self.generator.generate_transactions(
                    &self.market_makers[5].0,
                    1,
                    &self.market_makers[5].1,
                    true,
                );
                for txn in txns.iter() {
                    if let TransactionPayload::EntryFunction(entry_function) = txn.payload() {
                        let function_name = entry_function.function().as_str();
                        self.market_makers[5].1.push(function_name.to_string());
                    }
                }
                return txns;
            } else if rand < 991 {
                let txns = self.generator.generate_transactions(
                    &self.market_makers[6].0,
                    1,
                    &self.market_makers[6].1,
                    true,
                );
                for txn in txns.iter() {
                    if let TransactionPayload::EntryFunction(entry_function) = txn.payload() {
                        let function_name = entry_function.function().as_str();
                        self.market_makers[6].1.push(function_name.to_string());
                    }
                }
                return txns;
            }
        }
        println!("MarketMakerPoolWrapperGenerator::generate_transactions: non-market-maker");
        let mut accounts_to_use = self
            .source_accounts_pool
            .take_from_pool(1, true, &mut self.rng);
        if accounts_to_use.is_empty() {
            println!(
                "MarketMakerPoolWrapperGenerator::generate_transactions: accounts_to_use is empty"
            );
            return Vec::new();
        }
        let txns: Vec<SignedTransaction> = accounts_to_use
            .iter_mut()
            .flat_map(|(account, history)| {
                self.generator
                    .generate_transactions(account, 1, history, false)
            })
            .collect();

        let mut function_calls = HashMap::new();
        for txn in txns.iter() {
            if let TransactionPayload::EntryFunction(entry_function) = txn.payload() {
                let function_name = entry_function.function().as_str();
                function_calls.insert(txn.sender(), function_name.to_string());
            }
        }
        accounts_to_use = accounts_to_use
            .into_iter()
            .map(|(account, history)| {
                if let Some(function_name) = function_calls.get(&account.address()) {
                    let mut history = history.clone();
                    history.push(function_name.clone());
                    (account, history)
                } else {
                    (account, history)
                }
            })
            .collect();

        self.source_accounts_pool.add_to_pool(accounts_to_use);
        println!(
            "MarketMakerPoolWrapperGenerator::source_pool_len {}",
            self.source_accounts_pool.len()
        );
        txns
    }
}

pub struct MarketMakerPoolWrapperCreator {
    creator: Box<dyn TransactionGeneratorCreator>,
    source_accounts_pool: Arc<ObjectPool<(LocalAccount, Vec<String>)>>,
}

impl MarketMakerPoolWrapperCreator {
    pub fn new(
        creator: Box<dyn TransactionGeneratorCreator>,
        source_accounts_pool: Arc<ObjectPool<(LocalAccount, Vec<String>)>>,
    ) -> Self {
        Self {
            creator,
            source_accounts_pool,
        }
    }
}

impl TransactionGeneratorCreator for MarketMakerPoolWrapperCreator {
    fn create_transaction_generator(
        &self,
        txn_counter: Arc<AtomicU64>,
    ) -> Box<dyn TransactionGenerator> {
        Box::new(MarketMakerPoolWrapperGenerator::new(
            StdRng::from_entropy(),
            self.creator.create_transaction_generator(txn_counter),
            self.source_accounts_pool.clone(),
        ))
    }
}

pub struct ReuseAccountsPoolWrapperGenerator {
    rng: StdRng,
    generator: Box<dyn TransactionGenerator>,
    source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
}

impl ReuseAccountsPoolWrapperGenerator {
    pub fn new(
        rng: StdRng,
        generator: Box<dyn TransactionGenerator>,
        source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
    ) -> Self {
        Self {
            rng,
            generator,
            source_accounts_pool,
        }
    }
}

impl TransactionGenerator for ReuseAccountsPoolWrapperGenerator {
    fn generate_transactions(
        &mut self,
        _account: &LocalAccount,
        num_to_create: usize,
        _history: &[String],
        _market_maker: bool,
    ) -> Vec<SignedTransaction> {
        let mut accounts_to_use =
            self.source_accounts_pool
                .take_from_pool(num_to_create, true, &mut self.rng);
        if accounts_to_use.is_empty() {
            return Vec::new();
        }
        let txns = accounts_to_use
            .iter_mut()
            .flat_map(|account| {
                self.generator
                    .generate_transactions(account, 1, &Vec::new(), false)
            })
            .collect();

        self.source_accounts_pool.add_to_pool(accounts_to_use);
        txns
    }
}

pub struct ReuseAccountsPoolWrapperCreator {
    creator: Box<dyn TransactionGeneratorCreator>,
    source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
}

impl ReuseAccountsPoolWrapperCreator {
    pub fn new(
        creator: Box<dyn TransactionGeneratorCreator>,
        source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
    ) -> Self {
        Self {
            creator,
            source_accounts_pool,
        }
    }
}

impl TransactionGeneratorCreator for ReuseAccountsPoolWrapperCreator {
    fn create_transaction_generator(
        &self,
        txn_counter: Arc<AtomicU64>,
    ) -> Box<dyn TransactionGenerator> {
        Box::new(ReuseAccountsPoolWrapperGenerator::new(
            StdRng::from_entropy(),
            self.creator.create_transaction_generator(txn_counter),
            self.source_accounts_pool.clone(),
        ))
    }
}

// pub struct BypassAccountsPoolWrapperGenerator {
//     rng: StdRng,
//     generator: Box<dyn TransactionGenerator>,
//     source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
//     destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
// }

// impl BypassAccountsPoolWrapperGenerator {
//     pub fn new(
//         rng: StdRng,
//         generator: Box<dyn TransactionGenerator>,
//         source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
//         destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
//     ) -> Self {
//         Self {
//             rng,
//             generator,
//             source_accounts_pool,
//             destination_accounts_pool,
//         }
//     }
// }

// impl TransactionGenerator for BypassAccountsPoolWrapperGenerator {
//     fn generate_transactions(
//         &mut self,
//         account: &LocalAccount,
//         _num_to_create: usize,
//     ) -> Vec<SignedTransaction> {
//         let accounts_to_use =
//             self.source_accounts_pool
//                 .take_from_pool(self.source_accounts_pool.len(), true, &mut self.rng);
//         if accounts_to_use.is_empty() {
//             return Vec::new();
//         }
//         if let Some(destination_accounts_pool) = &self.destination_accounts_pool {
//             destination_accounts_pool.add_to_pool(accounts_to_use);
//         }
//         let txns = self.generator.generate_transactions(account, 1);
//         txns
//     }
// }

// pub struct BypassAccountsPoolWrapperCreator {
//     creator: Box<dyn TransactionGeneratorCreator>,
//     source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
//     destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
// }

// impl BypassAccountsPoolWrapperCreator {
//     pub fn new(
//         creator: Box<dyn TransactionGeneratorCreator>,
//         source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
//         destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
//     ) -> Self {
//         Self {
//             creator,
//             source_accounts_pool,
//             destination_accounts_pool,
//         }
//     }
// }

// impl TransactionGeneratorCreator for BypassAccountsPoolWrapperCreator {
//     fn create_transaction_generator(&self, txn_counter: Arc<AtomicU64>) -> Box<dyn TransactionGenerator> {
//         Box::new(BypassAccountsPoolWrapperGenerator::new(
//             StdRng::from_entropy(),
//             self.creator.create_transaction_generator(txn_counter),
//             self.source_accounts_pool.clone(),
//             self.destination_accounts_pool.clone(),
//         ))
//     }
// }
