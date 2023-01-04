use std::sync::Arc;

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{transaction_generator::{TransactionGenerator, TransactionGeneratorCreator}, emitter::stats::DynamicStatsTracking};
use aptos_sdk::types::{transaction::SignedTransaction, LocalAccount};
use async_trait::async_trait;
use rand::{prelude::StdRng, Rng};
use rand_core::{OsRng, SeedableRng};

pub struct TxnMixPhasedGenerator {
    rng: StdRng,
    // for each phase, list of transaction mixes.
    txn_mix_per_phase: Vec<Vec<(Box<dyn TransactionGenerator>, usize)>>,
    total_weight_per_phase: Vec<usize>,
    phase: Arc<DynamicStatsTracking>,
}

impl TxnMixPhasedGenerator {
    pub fn new(rng: StdRng, txn_mix_per_phase: Vec<Vec<(Box<dyn TransactionGenerator>, usize)>>, phase: Arc<DynamicStatsTracking>) -> Self {
        let total_weight_per_phase = txn_mix_per_phase.iter().map(|txn_mix| txn_mix.iter().map(|(_, weight)| weight).sum()).collect();
        Self {
            rng,
            txn_mix_per_phase,
            total_weight_per_phase,
            phase,
        }
    }
}

impl TransactionGenerator for TxnMixPhasedGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let phase = self.phase.get_cur_phase() % self.txn_mix_per_phase.len();

        let mut picked = self.rng.gen_range(0, self.total_weight_per_phase[phase]);
        for (gen, weight) in &mut self.txn_mix_per_phase[phase] {
            if picked < *weight {
                return gen.generate_transactions(accounts, transactions_per_account);
            }
            picked -= *weight;
        }
        panic!(
            "Picked {} out of {}, at phase {}, couldn't find correct generator",
            picked, self.total_weight_per_phase[phase], phase,
        );
    }
}

pub struct TxnMixPhasedGeneratorCreator {
    txn_mix_per_phase_creators: Vec<Vec<(Box<dyn TransactionGeneratorCreator>, usize)>>,
    phase: Arc<DynamicStatsTracking>,
}

impl TxnMixPhasedGeneratorCreator {
    pub fn new(txn_mix_per_phase_creators: Vec<Vec<(Box<dyn TransactionGeneratorCreator>, usize)>>, phase: Arc<DynamicStatsTracking>) -> Self {
        Self { txn_mix_per_phase_creators, phase }
    }
}

#[async_trait]
impl TransactionGeneratorCreator for TxnMixPhasedGeneratorCreator {
    async fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        let mut txn_mix_per_phase = Vec::<Vec<(Box<dyn TransactionGenerator>, usize)>>::new();
        for txn_mix_creators in self.txn_mix_per_phase_creators.iter_mut() {
            let mut txn_mix = Vec::<(Box<dyn TransactionGenerator>, usize)>::new();
            for (generator_creator, weight) in txn_mix_creators.iter_mut() {
                txn_mix.push((
                    generator_creator.create_transaction_generator().await,
                    *weight,
                ));
            }
            txn_mix_per_phase.push(txn_mix);
        }

        Box::new(TxnMixPhasedGenerator::new(
            StdRng::from_seed(OsRng.gen()),
            txn_mix_per_phase,
            self.phase.clone(),
        ))
    }
}
