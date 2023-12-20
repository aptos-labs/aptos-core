// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ObjectPool, call_custom_modules::{UserModuleTransactionGenerator, TransactionGeneratorWorker}, publishing::publish_util::Package, ReliableTransactionSubmitter};
use aptos_sdk::{
    bcs,
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use move_core_types::{
    ident_str,
    language_storage::ModuleId,
};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use rand::rngs::StdRng;
use std::sync::Arc;

/// Starts new round in the tournament and divides all the players into games.
pub fn start_new_round(
    module_id: ModuleId,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("start_new_round").to_owned(),
        vec![],
        vec![],
    ))
}

pub fn move_players_to_round(
    module_id: ModuleId,
    player_accounts: Vec<AccountAddress>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("move_players_to_round").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&player_accounts).unwrap()
        ],
    ))
}

pub struct TournamentStartNewRoundTransactionGenerator {
    to_setup: Arc<ObjectPool<LocalAccount>>,
    done: Arc<ObjectPool<LocalAccount>>,
    batch_size: usize,
}

impl TournamentStartNewRoundTransactionGenerator {
    pub fn new(
        to_setup: Arc<ObjectPool<LocalAccount>>,
        done: Arc<ObjectPool<LocalAccount>>,
        batch_size: usize,
    ) -> Self {
        Self {
            to_setup,
            done,
            batch_size,
        }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for TournamentStartNewRoundTransactionGenerator {
    fn initialize_package(
        &mut self,
        _package: &Package,
        _publisher: &mut LocalAccount,
        _txn_factory: &TransactionFactory,
        _rng: &mut StdRng,
    ) -> Vec<SignedTransaction> {
        vec![]
    }

    async fn create_generator_fn(
        &self,
        _root_account: &mut LocalAccount,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        _rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        let batch_size = self.batch_size;
        let to_setup = self.to_setup.clone();
        let done = self.done.clone();
        Arc::new(move |account, package, publisher, txn_factory, rng| {
            let batch = to_setup.take_from_pool(usize::MAX, true, rng);

            if batch.is_empty() {
                return None;
            }
            let addresses: Vec<_> = batch.iter().map(|a| a.address()).collect();
            println!("Tournament Generator: submitting transaction: start_new_round: {:?}", addresses);
            let builder = txn_factory.payload(start_new_round(package.get_module_id("rps_utils")));
            done.add_to_pool(batch);
            Some(account.sign_multi_agent_with_transaction_builder(vec![publisher], builder))
        })
    }
}

pub struct TournamentMovePlayersToRoundTransactionGenerator {
    to_join: Arc<ObjectPool<LocalAccount>>,
    joined: Arc<ObjectPool<LocalAccount>>,
    batch_size: usize,
}

impl TournamentMovePlayersToRoundTransactionGenerator {
    pub fn new(
        to_join: Arc<ObjectPool<LocalAccount>>,
        joined: Arc<ObjectPool<LocalAccount>>,
        batch_size: usize,
    ) -> Self {
        Self {
            to_join,
            joined,
            batch_size,
        }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for TournamentMovePlayersToRoundTransactionGenerator {
    fn initialize_package(
        &mut self,
        _package: &Package,
        _publisher: &mut LocalAccount,
        _txn_factory: &TransactionFactory,
        _rng: &mut StdRng,
    ) -> Vec<SignedTransaction> {
        vec![]
    }

    async fn create_generator_fn(
        &self,
        _root_account: &mut LocalAccount,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        _rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        let batch_size = self.batch_size;
        let to_join = self.to_join.clone();
        let joined = self.joined.clone();
        Arc::new(move |account, package, publisher, txn_factory, rng| {
            let batch = to_join.take_from_pool(batch_size, true, rng);

            if batch.is_empty() {
                return None;
            }
            let addresses = batch.iter().map(|a| a.address()).collect();
            println!("Tournament Generator: submitting transaction: move_players_to_round: {:?}", addresses);
            let builder = txn_factory.payload(move_players_to_round(package.get_module_id("rps_utils"), addresses));
            joined.add_to_pool(batch);
            Some(account.sign_multi_agent_with_transaction_builder(vec![publisher], builder))
        })
    }
}
