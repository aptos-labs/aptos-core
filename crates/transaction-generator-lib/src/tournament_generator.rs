// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use move_core_types::{
    ident_str,
    language_storage::ModuleId,
};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use rand::{
    prelude::SliceRandom,
    rngs::StdRng,
    SeedableRng,
};
use std::sync::Arc;

/// Starts new round in the tournament and divides all the players into games.
pub fn setup_new_round(
    player_accounts: Arc<RwLock<Vec<AccountAddress>>>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            // TODO: Need to get the module id for the aptos-tournament
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("rps_utils").to_owned(),
        ),
        ident_str!("setup_new_rund").to_owned(),
        vec![],
        // TODO: Need to format these arguments properly. Expected Vec<u8>
        vec![player_accounts.read()],
    ))
}

pub struct TournamentTransactionGenerator {
    rng: StdRng,
    num_tournaments: usize,
    txn_factory: TransactionFactory,
    admin_account: Arc<RwLock<LocalAccount>>,
    player_addresses: Arc<RwLock<Vec<AccountAddress>>>,
}

impl TournamentTransactionGenerator {
    pub fn new(
        mut rng: StdRng,
        txn_factory: TransactionFactory,
        num_tournaments: usize,
        admin_account: Arc<RwLock<LocalAccount>>,
        player_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    ) -> Self {
        player_addresses.write().shuffle(&mut rng);
        Self {
            rng,
            txn_factory,
            num_tournaments,
            admin_account,
            player_addresses
        }
    }

    fn gen_single_txn(
        &mut self,
        admin_account: &mut LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        admin_account.sign_with_transaction_builder(
            txn_factory.payload(setup_new_round(self.player_addresses.clone())),
        )
    }
}

impl TransactionGenerator for TournamentTransactionGenerator {
    fn generate_transactions(
        &mut self,
        // TODO: Is this admin account?
        _account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        vec![self.gen_single_txn(&mut self.admin_account.write(), &self.txn_factory)]
    }
}


pub struct TournamentTransactionGeneratorCreator {
    txn_factory: TransactionFactory,
    num_tournaments: usize,
    admin_account: Arc<RwLock<LocalAccount>>,
    player_addresses: Arc<RwLock<Vec<AccountAddress>>>,
}


impl TournamentTransactionGeneratorCreator {
    pub async fn new(
        txn_factory: TransactionFactory,
        num_tournaments: usize,
        admin_account: Arc<RwLock<LocalAccount>>,
        player_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    ) -> Self {
        Self {
            txn_factory,
            num_tournaments,
            admin_account,
            player_addresses
        }
    }
}

impl TransactionGeneratorCreator for TournamentTransactionGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        let rng = StdRng::from_entropy();

        // Create tournaments for each admin
        Box::new(TournamentTransactionGenerator::new(
            rng,
            self.txn_factory.clone(),
            self.num_tournaments,
            self.admin_account.clone(),
            self.player_addresses.clone()
        ))
    }
}
