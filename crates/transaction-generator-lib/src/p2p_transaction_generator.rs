// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{chain_id::ChainId, transaction::SignedTransaction, LocalAccount},
};
use rand::{distributions::{Distribution, Standard}, prelude::SliceRandom, rngs::StdRng, Rng, RngCore, SeedableRng};
use std::{cmp::max, sync::Arc};
use std::cmp::min;


/// Specifies how to get a given number of samples from an item pool.
trait Sampler: Send + Sync {
    fn sample(&mut self, rng: &mut StdRng, count: usize) -> Vec<usize>;
}

/// A sampler that samples a random subset of the pool. Samples are replaced immediately.
struct BasicSampler {
    pool_size: usize,
}

impl Sampler for BasicSampler {
    fn sample(&mut self, rng: &mut StdRng, count: usize) -> Vec<usize> {
        rand::seq::index::sample(rng, self.pool_size, count).into_vec()
    }
}


/// A samplers designed for generating a block of P2P transfers without read-write conflicts.
/// The pool is divided into sub-pools, one of the them being the primary:
/// it will keep serving the sample requests *without replacement*, until it's depleted.
/// When the current primary is depleted, another sub-pool takes over and the current one resets.
struct BurnAndRecycleSampler {
    /// We store all sub-pools together in 1 Vec: `item_pool[segment_size * x..segment_size * (x+1)]` being the x-th sub-pool.
    item_pool: Vec<usize>,
    next_index: usize,
    segment_size: usize,
    init_shuffle_done: bool
}

impl BurnAndRecycleSampler {
    fn new(num_items: usize, segment_size: usize) -> Self {
        Self {
            item_pool: (0..num_items).collect(),
            next_index: 0,
            segment_size,
            init_shuffle_done: false,
        }
    }

    fn sample_one(&mut self, rng: &mut StdRng) -> usize {
        if !self.init_shuffle_done {
            self.item_pool.shuffle(rng);
            self.init_shuffle_done = true;
        }
        if self.next_index % self.segment_size == 0 {
            // Switching to a new sub-pool: shuffle it first.
            let segment_end = min(self.item_pool.len(), self.next_index + self.segment_size);
            self.item_pool[self.next_index..segment_end].shuffle(rng);
        }
        let sampled = self.item_pool[self.next_index];
        self.next_index = (self.next_index + 1) % self.item_pool.len();
        sampled
    }
}

impl Sampler for BurnAndRecycleSampler {
    fn sample(&mut self, rng: &mut StdRng, count: usize) -> Vec<usize> {
        (0..count).map(|_| self.sample_one(rng)).collect()
    }
}

#[test]
fn test_burn_and_recycle_sampler() {
    let mut rng = StdRng::from_entropy();
    let mut sampler = BurnAndRecycleSampler::new(6, 3);
    for _ in 0..20 {
        let s = sampler.sample_one(&mut rng);
        println!("s={s}");
    }
}


pub struct P2PTransactionGenerator {
    rng: StdRng,
    send_amount: u64,
    txn_factory: TransactionFactory,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    sampler: Box<dyn Sampler>,
    invalid_transaction_ratio: usize,
}


impl P2PTransactionGenerator {
    pub fn new(
        rng: StdRng,
        send_amount: u64,
        txn_factory: TransactionFactory,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        invalid_transaction_ratio: usize,
        sampler: Box<dyn Sampler>,
    ) -> Self {
        Self {
            rng,
            send_amount,
            txn_factory,
            all_addresses,
            sampler,
            invalid_transaction_ratio,
        }
    }

    fn gen_single_txn(
        &self,
        from: &mut LocalAccount,
        to: &AccountAddress,
        num_coins: u64,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        from.sign_with_transaction_builder(
            txn_factory.payload(aptos_stdlib::aptos_coin_transfer(*to, num_coins)),
        )
    }

    fn generate_invalid_transaction(
        &mut self,
        rng: &mut StdRng,
        sender: &mut LocalAccount,
        receiver: &AccountAddress,
        reqs: &[SignedTransaction],
    ) -> SignedTransaction {
        let mut invalid_account = LocalAccount::generate(rng);
        let invalid_address = invalid_account.address();
        match Standard.sample(rng) {
            InvalidTransactionType::ChainId => {
                let txn_factory = &self.txn_factory.clone().with_chain_id(ChainId::new(255));
                self.gen_single_txn(sender, receiver, self.send_amount, txn_factory)
            },
            InvalidTransactionType::Sender => self.gen_single_txn(
                &mut invalid_account,
                receiver,
                self.send_amount,
                &self.txn_factory,
            ),
            InvalidTransactionType::Receiver => self.gen_single_txn(
                sender,
                &invalid_address,
                self.send_amount,
                &self.txn_factory,
            ),
            InvalidTransactionType::Duplication => {
                // if this is the first tx, default to generate invalid tx with wrong chain id
                // otherwise, make a duplication of an exist valid tx
                if reqs.is_empty() {
                    let txn_factory = &self.txn_factory.clone().with_chain_id(ChainId::new(255));
                    self.gen_single_txn(sender, receiver, self.send_amount, txn_factory)
                } else {
                    let random_index = rng.gen_range(0, reqs.len());
                    reqs[random_index].clone()
                }
            },
        }
    }
}

#[derive(Debug)]
enum InvalidTransactionType {
    /// invalid tx with wrong chain id
    ChainId,
    /// invalid tx with sender not on chain
    Sender,
    /// invalid tx with receiver not on chain
    Receiver,
    /// duplicate an exist tx
    Duplication,
}

impl Distribution<InvalidTransactionType> for Standard {
    fn sample<R: RngCore + ?Sized>(&self, rng: &mut R) -> InvalidTransactionType {
        match rng.gen_range(0, 4) {
            0 => InvalidTransactionType::ChainId,
            1 => InvalidTransactionType::Sender,
            2 => InvalidTransactionType::Receiver,
            _ => InvalidTransactionType::Duplication,
        }
    }
}

impl TransactionGenerator for P2PTransactionGenerator {
    fn generate_transactions(
        &mut self,
        account: &mut LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);
        let invalid_size = if self.invalid_transaction_ratio != 0 {
            // if enable mix invalid tx, at least 1 invalid tx per batch
            max(1, self.invalid_transaction_ratio / 100)
        } else {
            0
        };
        let mut num_valid_tx = num_to_create * (1 - invalid_size);

        let receivers: Vec<AccountAddress> = {
            let all_addrs = self.all_addresses.read();
            self.sampler.sample(&mut self.rng, num_to_create)
                .into_iter()
                .map(|i| all_addrs[i])
                .collect()
        };

        assert!(
            receivers.len() >= num_to_create,
            "failed: {} >= {}",
            receivers.len(),
            num_to_create
        );
        for i in 0..num_to_create {
            let receiver = receivers.get(i).expect("all_addresses can't be empty");
            let request = if num_valid_tx > 0 {
                num_valid_tx -= 1;
                self.gen_single_txn(account, receiver, self.send_amount, &self.txn_factory)
            } else {
                self.generate_invalid_transaction(
                    &mut self.rng.clone(),
                    account,
                    receiver,
                    &requests,
                )
            };
            requests.push(request);
        }
        requests
    }
}

pub struct P2PTransactionGeneratorCreator {
    txn_factory: TransactionFactory,
    amount: u64,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    invalid_transaction_ratio: usize,
}

impl P2PTransactionGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        amount: u64,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        invalid_transaction_ratio: usize,
    ) -> Self {
        Self {
            txn_factory,
            amount,
            all_addresses,
            invalid_transaction_ratio,
        }
    }
}

impl TransactionGeneratorCreator for P2PTransactionGeneratorCreator {
    fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        let mut rng = StdRng::from_entropy();
        let num_addresses = self.all_addresses.read().len();
        let sampler = Box::new(BurnAndRecycleSampler::new(num_addresses, (num_addresses + 1) / 2));
        Box::new(P2PTransactionGenerator::new(
            rng,
            self.amount,
            self.txn_factory.clone(),
            self.all_addresses.clone(),
            self.invalid_transaction_ratio,
            sampler,
        ))
    }
}
