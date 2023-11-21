// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{chain_id::ChainId, transaction::SignedTransaction, LocalAccount},
};
use rand::{
    distributions::{Distribution, Standard},
    prelude::SliceRandom,
    rngs::StdRng,
    Rng, RngCore, SeedableRng,
};
use std::{
    cmp::{max, min},
    sync::Arc,
};

pub struct TournamentTransactionGenerator {
    rng: StdRng,
    num_tournaments: u64,
    txn_factory: TransactionFactory,
    admin_accounts: Arc<RwLock<Vec<LocalAccount>>>,
    player_accounts: Arc<RwLock<Vec<LocalAccount>>>,
}

pub struct TournamentTransactionGeneratorCreator {
    txn_factory: TransactionFactory,
    num_tournaments: u64,
    all_accounts: Arc<RwLock<Vec<LocalAccount>>>,
}

impl P2PTransactionGenerator {
    pub fn new(
        mut rng: StdRng,
        txn_factory: TransactionFactory,
        num_tournaments: u64,
        admin_accounts: Arc<RwLock<Vec<LocalAccount>>>,
        player_accounts: Arc<RwLock<Vec<LocalAccount>>>,
    ) -> Self {
        player_accounts.write().shuffle(&mut rng);
        Self {
            rng,
            txn_factory,
            num_tournaments,
            admin_accounts,
            player_accounts
        }
    }
}

impl TransactionGenerator for P2PTransactionGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {

    }
}


impl TournamentTransactionGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        num_tournaments: u64,
        all_accounts: Arc<RwLock<Vec<LocalAccount>>>,
    ) -> Self {
        Self {
            txn_factory,
            amount,
            all_accounts,
        }
    }
}

impl TransactionGeneratorCreator for TournamentTransactionGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        let rng = StdRng::from_entropy();
        let sampler: Box<dyn Sampler<AccountAddress>> = match self.sampling_mode {
            SamplingMode::Basic => Box::new(BasicSampler::new()),
            SamplingMode::BurnAndRecycle(recycle_batch_size) => {
                Box::new(BurnAndRecycleSampler::new(recycle_batch_size))
            },
        };
        // Split accounts into admins and players.
        let admin_accounts = Arc::new(RwLock::new());
        let player_accounts = all_accounts;
        Box::new(TournamentTransactionGenerator::new(
            rng,
            self.txn_factory.clone(),
            self.num_tournaments,
            admin_accounts,
            player_accounts.clone()
        ))
    }

}