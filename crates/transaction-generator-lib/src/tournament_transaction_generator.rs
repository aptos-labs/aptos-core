// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use super::ReliableTransactionSubmitter;
use crate::{
    TransactionGenerator, TransactionGeneratorCreator,
};
use aptos_infallible::RwLock;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use ethereum_tx_sign::{LegacyTransaction, Transaction};
use ethereum_types::H160;
use move_core_types::{ident_str, language_storage::ModuleId};
use rand::{prelude::SliceRandom, rngs::StdRng, SeedableRng};
use std::sync::Arc;


impl TransactionGeneratorCreator for TournamentTransactionGeneratorCreator {
    fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        let rng = StdRng::from_entropy();
        let sampler: Box<dyn Sampler<EthereumWallet>> = match self.sampling_mode {
            SamplingMode::Basic => Box::new(BasicSampler::new()),
            SamplingMode::BurnAndRecycle(recycle_batch_size) => {
                Box::new(BurnAndRecycleSampler::new(recycle_batch_size))
            },
        };

        Box::new(EthereumP2PTransactionGenerator::new(
            rng,
            self.amount,
            self.txn_factory.clone(),
            sampler,
            self.ethereum_wallets.clone(),
        ))
    }
}