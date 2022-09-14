// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{
    transaction_generator::{TransactionGenerator, TransactionGeneratorCreator},
    TransactionType,
};
use aptos_sdk::types::{transaction::SignedTransaction, LocalAccount};
use rand::prelude::StdRng;
use rand::Rng;
use rand_core::{OsRng, SeedableRng};

pub struct TxnMixGenerator {
    rng: StdRng,
    txn_mix: Vec<(TransactionType, Box<dyn TransactionGenerator>, usize)>,
    total_weight: usize,
}

impl TxnMixGenerator {
    pub fn new(
        rng: StdRng,
        txn_mix: Vec<(TransactionType, Box<dyn TransactionGenerator>, usize)>,
    ) -> Self {
        let total_weight = txn_mix.iter().map(|(_, _, weight)| weight).sum();
        Self {
            rng,
            txn_mix,
            total_weight,
        }
    }

    pub fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> (TransactionType, Vec<SignedTransaction>) {
        let mut picked = self.rng.gen_range(0, self.total_weight);
        for (txn_type, gen, weight) in &mut self.txn_mix {
            if picked < *weight {
                return (
                    *txn_type,
                    gen.generate_transactions(accounts, transactions_per_account),
                );
            }
            picked -= *weight;
        }
        panic!(
            "Picked {} out of {}, couldn't find correct generator",
            picked, self.total_weight
        );
    }
}

pub struct TxnMixGeneratorCreator {
    txn_mix_creators: Vec<(TransactionType, Box<dyn TransactionGeneratorCreator>, usize)>,
}

impl TxnMixGeneratorCreator {
    pub fn new(
        txn_mix_creators: Vec<(TransactionType, Box<dyn TransactionGeneratorCreator>, usize)>,
    ) -> Self {
        Self { txn_mix_creators }
    }

    pub fn create_transaction_mix_generator(&self) -> TxnMixGenerator {
        TxnMixGenerator::new(
            StdRng::from_seed(OsRng.gen()),
            self.txn_mix_creators
                .iter()
                .map(|(txn_type, generator_creator, weight)| {
                    (
                        *txn_type,
                        generator_creator.create_transaction_generator(),
                        *weight,
                    )
                })
                .collect(),
        )
    }
}
