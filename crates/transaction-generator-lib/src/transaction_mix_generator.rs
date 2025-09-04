// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{TransactionGenerator, TransactionGeneratorCreator};
use velor_sdk::types::{transaction::SignedTransaction, LocalAccount};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub struct PhasedTxnMixGenerator {
    rng: StdRng,
    // for each phase, list of transaction mixes.
    txn_mix_per_phase: Vec<Vec<(Box<dyn TransactionGenerator>, usize)>>,
    total_weight_per_phase: Vec<usize>,
    phase: Arc<AtomicUsize>,
}

impl PhasedTxnMixGenerator {
    pub fn new(
        rng: StdRng,
        txn_mix_per_phase: Vec<Vec<(Box<dyn TransactionGenerator>, usize)>>,
        phase: Arc<AtomicUsize>,
    ) -> Self {
        let total_weight_per_phase = txn_mix_per_phase
            .iter()
            .map(|txn_mix| txn_mix.iter().map(|(_, weight)| weight).sum())
            .collect();
        Self {
            rng,
            txn_mix_per_phase,
            total_weight_per_phase,
            phase,
        }
    }
}

impl TransactionGenerator for PhasedTxnMixGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let phase = if self.txn_mix_per_phase.len() == 1 {
            // when only single txn_mix is passed, use it for all phases, for simplicity
            0
        } else {
            self.phase.load(Ordering::Relaxed)
        };

        let mut picked = self.rng.gen_range(0, self.total_weight_per_phase[phase]);
        for (gen, weight) in &mut self.txn_mix_per_phase[phase] {
            if picked < *weight {
                return gen.generate_transactions(account, num_to_create);
            }
            picked -= *weight;
        }
        panic!(
            "Picked {} out of {}, at phase {}, couldn't find correct generator",
            picked, self.total_weight_per_phase[phase], phase,
        );
    }
}

pub struct PhasedTxnMixGeneratorCreator {
    txn_mix_per_phase_creators: Vec<Vec<(Box<dyn TransactionGeneratorCreator>, usize)>>,
    phase: Arc<AtomicUsize>,
}

impl PhasedTxnMixGeneratorCreator {
    pub fn new(
        txn_mix_per_phase_creators: Vec<Vec<(Box<dyn TransactionGeneratorCreator>, usize)>>,
        phase: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            txn_mix_per_phase_creators,
            phase,
        }
    }
}

impl TransactionGeneratorCreator for PhasedTxnMixGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        let mut txn_mix_per_phase = Vec::<Vec<(Box<dyn TransactionGenerator>, usize)>>::new();
        for txn_mix_creators in self.txn_mix_per_phase_creators.iter() {
            let mut txn_mix = Vec::<(Box<dyn TransactionGenerator>, usize)>::new();
            for (generator_creator, weight) in txn_mix_creators.iter() {
                txn_mix.push((generator_creator.create_transaction_generator(), *weight));
            }
            txn_mix_per_phase.push(txn_mix);
        }

        Box::new(PhasedTxnMixGenerator::new(
            StdRng::from_entropy(),
            txn_mix_per_phase,
            self.phase.clone(),
        ))
    }
}
