// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use super::ReliableTransactionSubmitter;
use crate::{
    p2p_transaction_generator::{BasicSampler, BurnAndRecycleSampler, Sampler, SamplingMode},
    TransactionGenerator, TransactionGeneratorCreator,
};
use aptos_infallible::RwLock;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use aptos_storage_interface::state_view::LatestDbStateCheckpointView;
use aptos_storage_interface::{DbReaderWriter};
use aptos_state_view::{StateView};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use ethereum_tx_sign::{LegacyTransaction, Transaction};
use ethereum_types::H160;
use move_core_types::{ident_str, language_storage::ModuleId};
use rand::{prelude::SliceRandom, rngs::StdRng, SeedableRng};
use secp256k1::{PublicKey, SecretKey};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use tiny_keccak::keccak256;
use aptos_types::access_path::AccessPath;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::table::TableHandle;
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use move_core_types::parser::parse_struct_tag;

pub type EthereumAddress = H160;

pub fn generate_keypair() -> (SecretKey, PublicKey) {
    let secp = secp256k1::Secp256k1::new();
    secp.generate_keypair(&mut secp256k1::rand::thread_rng())
}

pub fn public_key_address(public_key: &PublicKey) -> EthereumAddress {
    let public_key = public_key.serialize_uncompressed();
    debug_assert_eq!(public_key[0], 0x04);
    let hash = keccak256(&public_key[1..]);

    EthereumAddress::from_slice(&hash[12..])
}

// fn u128_to_bytes(u: u128) -> Vec<u8> {
//     let mut result = Vec::new();

//     for i in (0..16).rev() {
//         let byte = ((u >> (i * 8)) & 0xFF) as u8;
//         result.push(byte);
//     }

//     result
// }


#[derive(Deserialize, Serialize)]
struct EvmStore {
    nonce: TableHandle,
    balance: TableHandle,
    code: TableHandle,
    storage: TableHandle,
    pub_keys: TableHandle,
}

#[derive(Debug, Clone)]
pub struct EthereumWallet {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
    pub public_address: EthereumAddress,
}

impl EthereumWallet {
    pub fn new(secret_key: &SecretKey, public_key: &PublicKey) -> Self {
        let public_address: EthereumAddress = public_key_address(public_key);
        EthereumWallet {
            secret_key: *secret_key,
            public_key: *public_key,
            public_address,
        }
    }
}

/// Transfers `amount` of coins `CoinType` from `from` to `to`.
pub fn _ethereum_coin_transfer(
    from: &EthereumWallet,
    to: &EthereumWallet,
    amount: u128,
) -> TransactionPayload {
    let eth_txn = LegacyTransaction {
        chain: 1,
        nonce: 0,
        to: Some(to.public_address.into()),
        value: amount,
        gas_price: 250,
        gas: 21000,
        data: vec![],
    };
    let ecdsa = eth_txn.ecdsa(&from.secret_key.secret_bytes()).unwrap();
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

/// Transfers `amount` of coins `CoinType` from `from` to `to`.
pub fn ethereum_direct_coin_transfer(
    from: &EthereumWallet,
    to: &EthereumWallet,
    _amount: u128,
) -> TransactionPayload {
    // let addr: Vec<u8> = vec![
    //     147, 139, 107, 200, 81, 82, 65, 97, 55, 231, 218, 108, 56, 9, 146, 20, 74, 222, 241, 104,
    // ];
    let amount: Vec<u8> = vec![2];
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("evm").to_owned(),
        ),
        ident_str!("call2").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&from.public_address.as_bytes().to_vec()).unwrap(),
            bcs::to_bytes(&to.public_address.as_bytes().to_vec()).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
            bcs::to_bytes::<Vec<u8>>(&vec![]).unwrap(),
            bcs::to_bytes::<u64>(&100000).unwrap(),
        ],
    ))
}

pub fn ethereum_iniatialize_account(account: &EthereumWallet) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("evm").to_owned(),
        ),
        ident_str!("initialize_account").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&account.public_address.as_bytes().to_vec()).unwrap()
        ],
    ))
}

pub struct EthereumP2PTransactionGenerator {
    rng: StdRng,
    send_amount: u128,
    txn_factory: TransactionFactory,
    ethereum_wallets: Arc<RwLock<Vec<EthereumWallet>>>,
    sampler: Box<dyn Sampler<EthereumWallet>>,
}

impl EthereumP2PTransactionGenerator {
    pub fn new(
        mut rng: StdRng,
        send_amount: u128,
        txn_factory: TransactionFactory,
        sampler: Box<dyn Sampler<EthereumWallet>>,
        ethereum_wallets: Arc<RwLock<Vec<EthereumWallet>>>,
    ) -> Self {
        ethereum_wallets.write().shuffle(&mut rng);
        Self {
            rng,
            send_amount,
            txn_factory,
            sampler,
            ethereum_wallets,
        }
    }

    fn gen_single_txn(
        &self,
        aptos_signer: &LocalAccount,
        from: &EthereumWallet,
        to: &EthereumWallet,
        num_coins: u128,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        aptos_signer.sign_with_transaction_builder(
            txn_factory.payload(ethereum_direct_coin_transfer(from, to, num_coins)), // txn_factory.payload(aptos_stdlib::aptos_coin_transfer(*to, num_coins)),
        )
    }

    fn get_value<T: DeserializeOwned>(
        state_key: &StateKey,
        state_view: &impl StateView,
    ) -> anyhow::Result<Option<T>> {
        let value = state_view
            .get_state_value_bytes(state_key)?
            .map(move |value| bcs::from_bytes(value.as_slice()));
        //println!("value: {:?}", value);
        value.transpose().map_err(anyhow::Error::msg)
    }

    fn get_eth_balance(&self, db: &impl StateView, address: &EthereumAddress) -> anyhow::Result<move_core_types::u256::U256> {
        let evm_store_path =
            StateKey::access_path(AccessPath::resource_access_path(CORE_CODE_ADDRESS, parse_struct_tag("0x1::evm::EvmData").unwrap()).unwrap());
        let evm_store: EvmStore =  Self::get_value(&evm_store_path, db).unwrap().unwrap();
        let evm_store_balance_table = evm_store.balance;
        let state_key = &StateKey::table_item(
            evm_store_balance_table,
            bcs::to_bytes(&address.as_bytes().to_vec()).unwrap(),
        );
        let state_value = Self::get_value(state_key, db).unwrap().unwrap();
        //let state_value = db.get_state_value(state_key).unwrap().map(StateValue::into_bytes).unwrap();
        //println!("state_value: {:?}", state_value);
        Ok(state_value)
    }

    fn get_balance_summary(&self, db: DbReaderWriter) {
        // print out the balance of 10 accounts and the total balance of all accounts
        let db_state_view = db.reader.latest_state_checkpoint_view().unwrap();
        let mut total_balance: move_core_types::u256::U256 = move_core_types::u256::U256::from_str_radix("0", 10).unwrap();
        for i in 0..10 {
            let address = self.ethereum_wallets.read()[i].public_address;
            let balance = self.get_eth_balance(&db_state_view, &address).unwrap();
            println!("{}: {:?}", address, balance);
        }

        for address in self.ethereum_wallets.read().iter() {
            let balance = self.get_eth_balance(&db_state_view, &address.public_address).unwrap();
            total_balance += balance;
        }
        println!("Total balance: {}", total_balance);
    }
}

impl TransactionGenerator for EthereumP2PTransactionGenerator {

    fn pre_generate(&self, db: DbReaderWriter) {
        self.get_balance_summary(db);
    }

    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);

        // [0... num_to_create) are senders    [num_to_create,..., 2*num_to_create) are receivers
        let sampled_wallets: Vec<EthereumWallet> = {
            let mut ethereum_wallets = self.ethereum_wallets.write();
            self.sampler.sample_from_pool(
                &mut self.rng,
                ethereum_wallets.as_mut(),
                2 * num_to_create,
            )
        };

        assert!(
            sampled_wallets.len() >= 2 * num_to_create,
            "failed: {} >= {}",
            sampled_wallets.len(),
            2 * num_to_create
        );
        for i in 0..num_to_create {
            let sender = sampled_wallets
                .get(i)
                .expect("ethereum_wallets can't be empty");
            let receiver = sampled_wallets
                .get(i + num_to_create)
                .expect("ethereum_wallets can't be empty");
            let request = self.gen_single_txn(
                account,
                sender,
                receiver,
                self.send_amount,
                &self.txn_factory,
            );
            requests.push(request);
        }
        requests
    }

    fn post_generate(&self, db: DbReaderWriter) {
        self.get_balance_summary(db);
    }
}

pub struct EthereumP2PTransactionGeneratorCreator {
    txn_factory: TransactionFactory,
    amount: u128,
    sampling_mode: SamplingMode,
    ethereum_wallets: Arc<RwLock<Vec<EthereumWallet>>>,
}

impl EthereumP2PTransactionGeneratorCreator {
    pub async fn new(
        txn_factory: TransactionFactory,
        amount: u128,
        aptos_accounts: &mut [LocalAccount],
        sampling_mode: SamplingMode,
        num_ethereum_accounts: usize,
        txn_executor: &dyn ReliableTransactionSubmitter,
    ) -> Self {
        println!("Generating ethereum wallets");
        let ethereum_wallets: Arc<RwLock<Vec<EthereumWallet>>> = Arc::new(RwLock::new(
            (0..num_ethereum_accounts)
                .map(|_| {
                    let (secret_key, public_key) = generate_keypair();
                    EthereumWallet::new(&secret_key, &public_key)
                })
                .collect(),
        ));
        println!(
            "Done generating ethereum wallets {}",
            ethereum_wallets.read().len()
        );
        // Initialize each ethereum account
        let txns = ethereum_wallets
            .read()
            .iter()
            .map(|ethereum_account| {
                aptos_accounts[0].sign_with_transaction_builder(
                    txn_factory.payload(ethereum_iniatialize_account(ethereum_account)),
                )
            })
            .collect::<Vec<SignedTransaction>>();
        txn_executor.execute_transactions(&txns).await.unwrap();
        println!("Initialized ethereum wallets");
        Self {
            txn_factory,
            amount,
            sampling_mode,
            ethereum_wallets,
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

        Box::new(EthereumP2PTransactionGenerator::new(
            rng,
            self.amount,
            self.txn_factory.clone(),
            sampler,
            self.ethereum_wallets.clone(),
        ))
    }
}
