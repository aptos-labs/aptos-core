// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{TransactionFeedback, TransactionGenerator, TransactionGeneratorCreator};
use aptos_sdk::types::{transaction::SignedTransaction, LocalAccount};
use aptos_types::transaction::TransactionOutput;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Dispatches feedback events to the handler for the currently active phase.
struct PhasedFeedback {
    feedbacks: Vec<Option<Arc<dyn TransactionFeedback>>>,
    phase: Arc<AtomicUsize>,
}

impl TransactionFeedback for PhasedFeedback {
    fn on_block_committed(&self, outputs: &[TransactionOutput]) {
        let phase = self.phase.load(Ordering::Relaxed);
        if let Some(Some(fb)) = self.feedbacks.get(phase) {
            fb.on_block_committed(outputs);
        }
    }

    fn wait_until_ready(&self) {
        let phase = self.phase.load(Ordering::Relaxed);
        if let Some(Some(fb)) = self.feedbacks.get(phase) {
            fb.wait_until_ready();
        }
    }
}

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
        for (r#gen, weight) in &mut self.txn_mix_per_phase[phase] {
            if picked < *weight {
                return r#gen.generate_transactions(account, num_to_create);
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

    fn transaction_feedback(&self) -> Option<Arc<dyn TransactionFeedback>> {
        let feedbacks_per_phase: Vec<Option<Arc<dyn TransactionFeedback>>> = self
            .txn_mix_per_phase_creators
            .iter()
            .map(|phase| {
                let feedbacks: Vec<_> = phase
                    .iter()
                    .filter_map(|(creator, _)| creator.transaction_feedback())
                    .collect();
                // TODO: support multiple feedbacks per phase via a broadcast wrapper
                assert!(
                    feedbacks.len() <= 1,
                    "At most one creator per phase may provide TransactionFeedback"
                );
                feedbacks.into_iter().next()
            })
            .collect();

        if feedbacks_per_phase.iter().any(|f| f.is_some()) {
            Some(Arc::new(PhasedFeedback {
                feedbacks: feedbacks_per_phase,
                phase: self.phase.clone(),
            }))
        } else {
            None
        }
    }
}
