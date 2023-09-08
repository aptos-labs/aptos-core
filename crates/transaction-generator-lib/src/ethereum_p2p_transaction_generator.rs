// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};
use rand::{
    distributions::{Distribution, Standard},
    prelude::SliceRandom,
    rngs::StdRng,
    Rng, RngCore, SeedableRng,
};
use secp256k1::{rand::rngs, PublicKey, SecretKey};
use std::sync::Arc;
use aptos_types::transaction::{EntryFunction, TransactionPayload};

use ethereum_types::H160;
use ethereum_tx_sign::{LegacyTransaction, Transaction};
use std::time::{SystemTime, UNIX_EPOCH};
use tiny_keccak::keccak256;
use crate::p2p_transaction_generator::{Sampler, SamplingMode, BasicSampler, BurnAndRecycleSampler};
use move_core_types::{
    ident_str,
    language_storage::ModuleId,
};

pub fn get_nstime() -> u64 {
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    // The correct way to calculate the current time is
    // `dur.as_secs() * 1_000_000_000 + dur.subsec_nanos() as u64`
    // But this is faster, and the difference in terms of entropy is
    // negligible (log2(10^9) == 29.9).
    dur.as_secs() << 30 | dur.subsec_nanos() as u64
}

pub type EthereumAddress = H160;

pub fn generate_keypair() -> (SecretKey, PublicKey) {
    let secp = secp256k1::Secp256k1::new();
    let mut rng = rngs::JitterRng::new_with_timer(get_nstime);
    secp.generate_keypair(&mut rng)
}

pub fn public_key_address(public_key: &PublicKey) -> EthereumAddress {
    let public_key = public_key.serialize_uncompressed();
    debug_assert_eq!(public_key[0], 0x04);
    let hash = keccak256(&public_key[1..]);

    EthereumAddress::from_slice(&hash[12..])
}

#[derive(Debug, Clone)]
pub struct EthereumWallet {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
    pub public_address: EthereumAddress,
}

impl EthereumWallet {
    pub fn new(secret_key: &SecretKey, public_key: &PublicKey) -> Self {
        let public_address: EthereumAddress = public_key_address(&public_key);
        EthereumWallet {
            secret_key: secret_key.clone(),
            public_key: public_key.clone(),
            public_address
        }
    }
}

/// Transfers `amount` of coins `CoinType` from `from` to `to`.
pub fn ethereum_coin_transfer(from: EthereumWallet, to: EthereumWallet, amount: u128) -> TransactionPayload {
    let eth_txn = LegacyTransaction {
        chain: 1,
        nonce: 0,
        to: Some(to.public_address.into()),
        value: amount,
        gas_price: 250,
        gas: 21000,
        data: vec![],
    };
    let ecdsa = eth_txn.ecdsa(from.secret_key.into()).unwrap();
    let eth_txn_bytes = eth_txn.sign(&ecdsa);
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("evm").to_owned(),
        ),
        ident_str!("call").to_owned(),
        vec![],
        vec![eth_txn_bytes],
    ))
}

pub struct EthereumP2PTransactionGenerator {
    rng: StdRng,
    send_amount: u64,
    txn_factory: TransactionFactory,
    aptos_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    ethereum_wallets: Arc<RwLock<Vec<EthereumWallet>>>,
    sampler: Box<dyn Sampler<EthereumWallet>>,
    invalid_transaction_ratio: usize,
}

impl EthereumP2PTransactionGenerator {
    pub fn new(
        mut rng: StdRng,
        send_amount: u64,
        txn_factory: TransactionFactory,
        aptos_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        invalid_transaction_ratio: usize,
        sampler: Box<dyn Sampler<EthereumWallet>>,
        ethereum_wallets: Arc<RwLock<Vec<EthereumWallet>>>,
    ) -> Self {
        aptos_addresses.write().shuffle(&mut rng);
        ethereum_wallets.write().shuffle(&mut rng);
        Self {
            rng,
            send_amount,
            txn_factory,
            aptos_addresses,
            sampler,
            invalid_transaction_ratio,
            ethereum_wallets
        }
    }

    fn gen_single_txn(
        &self,
        aptos_signer: &mut LocalAccount,
        from: &EthereumWallet,
        to: &EthereumWallet,
        num_coins: u64,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        aptos_signer.sign_with_transaction_builder(
            txn_factory.payload(aptos_stdlib::aptos_coin_transfer(*to, num_coins)),
        )
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

impl TransactionGenerator for EthereumP2PTransactionGenerator {
    fn generate_transactions(
        &mut self,
        account: &mut LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);
        let mut num_valid_tx = num_to_create;
        
        // [0... num_to_create) are senders    [num_to_create,..., 2*num_to_create) are receivers
        let sampled_wallets: Vec<EthereumWallet> = {
            let mut ethereum_wallets = self.ethereum_wallets.write();
            self.sampler.sample_from_pool(&mut self.rng, ethereum_wallets.as_mut(), 2*num_to_create)
        };

        assert!(
            sampled_wallets.len() >= 2*num_to_create,
            "failed: {} >= {}",
            sampled_wallets.len(),
            2*num_to_create
        );
        for i in 0..num_to_create {
            num_valid_tx -= 1;
            let sender = sampled_wallets.get(i).expect("ethereum_wallets can't be empty");
            let receiver = sampled_wallets.get(i+num_to_create).expect("ethereum_wallets can't be empty");
            let request = self.gen_single_txn(account, sender, receiver, self.send_amount, &self.txn_factory);
            requests.push(request);
        }
        requests
    }
}

pub struct EthereumP2PTransactionGeneratorCreator {
    txn_factory: TransactionFactory,
    amount: u64,
    aptos_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    invalid_transaction_ratio: usize,
    sampling_mode: SamplingMode,
    num_ethereum_accounts: usize,
}

impl EthereumP2PTransactionGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        amount: u64,
        aptos_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        invalid_transaction_ratio: usize,
        sampling_mode: SamplingMode,
        num_ethereum_accounts: usize,
    ) -> Self {
        Self {
            txn_factory,
            amount,
            aptos_addresses,
            invalid_transaction_ratio,
            sampling_mode,
            num_ethereum_accounts,
        }
    }
}

impl TransactionGeneratorCreator for EthereumP2PTransactionGeneratorCreator {
    fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        let rng = StdRng::from_entropy();
        let sampler: Box<dyn Sampler<EthereumWallet>> = match self.sampling_mode {
            SamplingMode::Basic => Box::new(BasicSampler::new()),
            SamplingMode::BurnAndRecycle(recycle_batch_size) => {
                Box::new(BurnAndRecycleSampler::new(recycle_batch_size))
            },
        };
        let ethereum_wallets: Arc<RwLock<Vec<EthereumWallet>>> = Arc::new(RwLock::new(
            (0..self.num_ethereum_accounts)
                .map(|_| {
                    let (secret_key, public_key) = generate_keypair();
                    EthereumWallet::new(&secret_key, &public_key)
                })
                .collect()
        ));
        Box::new(EthereumP2PTransactionGenerator::new(
            rng,
            self.amount,
            self.txn_factory.clone(),
            self.aptos_addresses.clone(),
            self.invalid_transaction_ratio,
            sampler,
            ethereum_wallets
        ))
    }
}
