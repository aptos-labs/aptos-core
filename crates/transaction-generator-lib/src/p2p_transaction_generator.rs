// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ObjectPool, TransactionGenerator, TransactionGeneratorCreator};
use velor_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{velor_stdlib, TransactionFactory},
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

pub enum SamplingMode {
    /// See `BasicSampler`.
    Basic,
    /// See `BurnAndRecycleSampler`.
    BurnAndRecycle(usize),
}

/// Specifies how to get a given number of samples from an item pool.
pub trait Sampler<T>: Send + Sync {
    fn sample_from_pool(
        &mut self,
        rng: &mut StdRng,
        pool: &mut Vec<T>,
        num_samples: usize,
    ) -> Vec<T>;
}

/// A sampler that samples a random subset of the pool. Samples are replaced immediately.
pub struct BasicSampler {}

impl BasicSampler {
    fn new() -> Self {
        Self {}
    }
}

impl<T: Clone + Send + Sync> Sampler<T> for BasicSampler {
    fn sample_from_pool(
        &mut self,
        rng: &mut StdRng,
        pool: &mut Vec<T>,
        num_samples: usize,
    ) -> Vec<T> {
        let mut samples = Vec::with_capacity(num_samples);
        let num_available = pool.len();
        for _ in 0..num_samples {
            let idx = rng.gen_range(0, num_available);
            samples.push(pool[idx].clone());
        }
        samples
    }
}

/// A samplers that samples from a pool but do not replace items until the pool is depleted.
/// The pool is divided into sub-pools. Replacement is done with with each sub-pool shuffled internally.
///
/// Here is an example. Say the initial pool is `[I, J, K, X, Y, Z]`.
/// A `BurnAndRecycleSampler` is created with `replace_batch_size=3` to sample from the pool.
/// The first 6 samples are guaranteed to be `Z`, `Y`, `X`, `K`, `J`, `I`.
/// Then at the beginning of the 7-th sampling,
/// sub-pools `{I, J, K}`, `{X, Y, Z}` are shuffled and replaced.
/// A possible state of the pool is `[K, I, J, Y, X, Z]`.
///
/// This behavior helps generate a block of non-conflicting coin transfer transactions,
/// when there are 2+ sub-pools of size larger than or equal to the block size.
pub struct BurnAndRecycleSampler<T> {
    /// We store all sub-pools together in 1 Vec: `item_pool[segment_size * x..segment_size * (x+1)]` being the x-th sub-pool.
    to_be_replaced: Vec<T>,
    sub_pool_size: usize,
}

impl<T: Clone + Send + Sync> BurnAndRecycleSampler<T> {
    fn new(replace_batch_size: usize) -> Self {
        Self {
            to_be_replaced: vec![],
            sub_pool_size: replace_batch_size,
        }
    }

    fn sample_one_from_pool(&mut self, rng: &mut StdRng, pool: &mut Vec<T>) -> T {
        if pool.is_empty() {
            let num_addresses = self.to_be_replaced.len();
            for replace_batch_start in (0..num_addresses).step_by(self.sub_pool_size) {
                let end = min(replace_batch_start + self.sub_pool_size, num_addresses);
                self.to_be_replaced[replace_batch_start..end].shuffle(rng);
            }
            for _ in 0..num_addresses {
                pool.push(self.to_be_replaced.pop().unwrap());
            }
        }
        let sample = pool.pop().unwrap();
        self.to_be_replaced.push(sample.clone());
        sample
    }
}

impl<T: Clone + Send + Sync> Sampler<T> for BurnAndRecycleSampler<T> {
    fn sample_from_pool(
        &mut self,
        rng: &mut StdRng,
        pool: &mut Vec<T>,
        num_samples: usize,
    ) -> Vec<T> {
        (0..num_samples)
            .map(|_| self.sample_one_from_pool(rng, pool))
            .collect()
    }
}

#[test]
fn test_burn_and_recycle_sampler() {
    use std::collections::HashSet;
    let mut rng = StdRng::from_entropy();
    let mut sampler = BurnAndRecycleSampler::new(3);
    let mut pool: Vec<u8> = (0..8).collect();
    let samples = (0..16)
        .map(|_| sampler.sample_one_from_pool(&mut rng, &mut pool))
        .collect::<Vec<_>>();
    // `samples[0..3]` and `samples[8..11]` are 2 permutations of sub-pool 0.
    assert_eq!(
        samples[0..3].iter().collect::<HashSet<_>>(),
        samples[8..11].iter().collect::<HashSet<_>>()
    );
    // `samples[3..6]` and `samples[11..14]` are 2 permutations of sub-pool 1.
    assert_eq!(
        samples[3..6].iter().collect::<HashSet<_>>(),
        samples[11..14].iter().collect::<HashSet<_>>()
    );
    // `samples[6..8]` and `samples[14..16]` are 2 permutations of sub-pool 1.
    assert_eq!(
        samples[6..8].iter().collect::<HashSet<_>>(),
        samples[14..16].iter().collect::<HashSet<_>>()
    );
}

pub struct P2PTransactionGenerator {
    rng: StdRng,
    send_amount: u64,
    txn_factory: TransactionFactory,
    all_addresses: Arc<ObjectPool<AccountAddress>>,
    sampler: Box<dyn Sampler<AccountAddress>>,
    invalid_transaction_ratio: usize,
    use_fa_transfer: bool,
}

impl P2PTransactionGenerator {
    pub fn new(
        rng: StdRng,
        send_amount: u64,
        txn_factory: TransactionFactory,
        all_addresses: Arc<ObjectPool<AccountAddress>>,
        invalid_transaction_ratio: usize,
        use_fa_transfer: bool,
        sampler: Box<dyn Sampler<AccountAddress>>,
    ) -> Self {
        Self {
            rng,
            send_amount,
            txn_factory,
            all_addresses,
            sampler,
            invalid_transaction_ratio,
            use_fa_transfer,
        }
    }

    fn gen_single_txn(
        &self,
        from: &LocalAccount,
        to: &AccountAddress,
        num_coins: u64,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        from.sign_with_transaction_builder(
            if self.use_fa_transfer {
                txn_factory.payload(velor_stdlib::velor_account_fungible_transfer_only(
                    *to, num_coins,
                ))
            } else {
                txn_factory.payload(velor_stdlib::velor_coin_transfer(*to, num_coins))
            },
        )
    }

    fn generate_invalid_transaction(
        &mut self,
        rng: &mut StdRng,
        sender: &LocalAccount,
        receiver: &AccountAddress,
        reqs: &[SignedTransaction],
    ) -> SignedTransaction {
        let invalid_account = LocalAccount::generate(rng);
        let invalid_address = invalid_account.address();
        match Standard.sample(rng) {
            InvalidTransactionType::ChainId => {
                let txn_factory = &self.txn_factory.clone().with_chain_id(ChainId::new(255));
                self.gen_single_txn(sender, receiver, self.send_amount, txn_factory)
            },
            InvalidTransactionType::Sender => self.gen_single_txn(
                &invalid_account,
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
        account: &LocalAccount,
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
            let mut all_addrs = self.all_addresses.write_view();
            self.sampler
                .sample_from_pool(&mut self.rng, all_addrs.as_mut(), num_to_create)
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
    all_addresses: Arc<ObjectPool<AccountAddress>>,
    invalid_transaction_ratio: usize,
    use_fa_transfer: bool,
    sampling_mode: SamplingMode,
}

impl P2PTransactionGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        amount: u64,
        all_addresses: Arc<ObjectPool<AccountAddress>>,
        invalid_transaction_ratio: usize,
        use_fa_transfer: bool,
        sampling_mode: SamplingMode,
    ) -> Self {
        let mut rng = StdRng::from_entropy();
        all_addresses.shuffle(&mut rng);

        Self {
            txn_factory,
            amount,
            all_addresses,
            invalid_transaction_ratio,
            use_fa_transfer,
            sampling_mode,
        }
    }
}

impl TransactionGeneratorCreator for P2PTransactionGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        let rng = StdRng::from_entropy();
        let sampler: Box<dyn Sampler<AccountAddress>> = match self.sampling_mode {
            SamplingMode::Basic => Box::new(BasicSampler::new()),
            SamplingMode::BurnAndRecycle(recycle_batch_size) => {
                Box::new(BurnAndRecycleSampler::new(recycle_batch_size))
            },
        };
        Box::new(P2PTransactionGenerator::new(
            rng,
            self.amount,
            self.txn_factory.clone(),
            self.all_addresses.clone(),
            self.invalid_transaction_ratio,
            self.use_fa_transfer,
            sampler,
        ))
    }
}
