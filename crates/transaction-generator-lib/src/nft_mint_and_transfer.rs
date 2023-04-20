// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::TransactionExecutor;
use crate::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_logger::info;
use aptos_sdk::{
    transaction_builder::{aptos_stdlib, aptos_stdlib::aptos_token_stdlib, TransactionFactory},
    types::{
        account_address::AccountAddress,
        transaction::{
            authenticator::{AuthenticationKey, AuthenticationKeyPreimage},
            SignedTransaction,
        },
        LocalAccount,
    },
};
use rand::{rngs::StdRng, thread_rng, SeedableRng};
use std::collections::HashMap;

const INITIAL_NFT_BALANCE: u64 = 50_000;

pub struct NFTMintAndTransfer {
    txn_factory: TransactionFactory,
    creator_address: AccountAddress,
    distribution_account: LocalAccount,
    collection_name: Vec<u8>,
    token_name: Vec<u8>,
    account_funded: HashMap<AccountAddress, bool>,
}

impl NFTMintAndTransfer {
    pub fn new(
        txn_factory: TransactionFactory,
        creator_address: AccountAddress,
        distribution_account: LocalAccount,
        collection_name: Vec<u8>,
        token_name: Vec<u8>,
    ) -> Self {
        Self {
            txn_factory,
            distribution_account,
            creator_address,
            collection_name,
            token_name,
            account_funded: Default::default(),
        }
    }
}

impl TransactionGenerator for NFTMintAndTransfer {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len() * transactions_per_account);
        for account in accounts {
            let account_funded = self
                .account_funded
                .get(&account.address())
                .cloned()
                .unwrap_or(false);
            for i in 0..transactions_per_account {
                requests.push(
                    if account_funded {
                        create_nft_transfer_request(
                            account,
                            &self.distribution_account,
                            self.creator_address,
                            &self.collection_name,
                            &self.token_name,
                            &self.txn_factory,
                            if i != transactions_per_account - 1 {
                                1
                            } else {
                                INITIAL_NFT_BALANCE + 1 - transactions_per_account as u64
                            },
                        )
                    } else {
                        create_nft_transfer_request(
                            &mut self.distribution_account,
                            account,
                            self.creator_address,
                            &self.collection_name,
                            &self.token_name,
                            &self.txn_factory,
                            if i != transactions_per_account - 1 {
                                1
                            } else {
                                INITIAL_NFT_BALANCE + 1 - transactions_per_account as u64
                            },
                        )
                    },
                );
            }
            self.account_funded
                .insert(account.address(), !account_funded);
        }
        requests
    }
}

pub async fn initialize_nft_collection(
    txn_executor: &dyn TransactionExecutor,
    root_account: &mut LocalAccount,
    creator_account: &mut LocalAccount,
    txn_factory: &TransactionFactory,
    collection_name: &[u8],
    token_name: &[u8],
) {
    // Create and mint the owner account first
    let create_account_txn = create_and_fund_account_request(
        root_account,
        10_000_000,
        creator_account.public_key(),
        txn_factory,
    );

    txn_executor
        .execute_transactions(&[create_account_txn])
        .await
        .unwrap();

    let collection_txn =
        create_nft_collection_request(creator_account, collection_name, txn_factory);

    txn_executor
        .execute_transactions(&[collection_txn])
        .await
        .unwrap();

    let token_txn =
        create_nft_token_request(creator_account, collection_name, token_name, txn_factory);

    txn_executor
        .execute_transactions(&[token_txn])
        .await
        .unwrap();

    info!("initialize_nft_collection complete");
}

pub fn create_nft_collection_request(
    creation_account: &mut LocalAccount,
    collection_name: &[u8],
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    creation_account.sign_with_transaction_builder(txn_factory.payload(
        aptos_token_stdlib::token_create_collection_script(
            collection_name.to_vec(),
            "description".to_owned().into_bytes(),
            "uri".to_owned().into_bytes(),
            u64::MAX,
            vec![false, false, false],
        ),
    ))
}

pub fn create_nft_token_request(
    creation_account: &mut LocalAccount,
    collection_name: &[u8],
    token_name: &[u8],
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    creation_account.sign_with_transaction_builder(txn_factory.payload(
        aptos_token_stdlib::token_create_token_script(
            collection_name.to_vec(),
            token_name.to_vec(),
            "collection description".to_owned().into_bytes(),
            100_000_000_000,
            u64::MAX,
            "uri".to_owned().into_bytes(),
            creation_account.address(),
            1,
            0,
            vec![false, false, false, false, false],
            vec![Vec::new()],
            vec![Vec::new()],
            vec![Vec::new()],
        ),
    ))
}

pub fn create_nft_transfer_request(
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    creation_address: AccountAddress,
    collection_name: &[u8],
    token_name: &[u8],
    txn_factory: &TransactionFactory,
    amount: u64,
) -> SignedTransaction {
    sender.sign_multi_agent_with_transaction_builder(
        vec![receiver],
        txn_factory.payload(aptos_token_stdlib::token_direct_transfer_script(
            creation_address,
            collection_name.to_vec(),
            token_name.to_vec(),
            0,
            amount,
        )),
    )
}

pub struct NFTMintAndTransferGeneratorCreator {
    txn_factory: TransactionFactory,
    creator_address: AccountAddress,
    distribution_accounts: Vec<LocalAccount>,
    collection_name: Vec<u8>,
    token_name: Vec<u8>,
}

impl NFTMintAndTransferGeneratorCreator {
    pub async fn new(
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &mut LocalAccount,
        txn_executor: &dyn TransactionExecutor,
        num_workers: usize,
    ) -> Self {
        let mut rng = StdRng::from_entropy();
        let mut creator_account = LocalAccount::generate(&mut rng);
        let creator_address = creator_account.address();
        let collection_name = "collection name".to_owned().into_bytes();
        let token_name = "token name".to_owned().into_bytes();
        initialize_nft_collection(
            txn_executor,
            root_account,
            &mut creator_account,
            &init_txn_factory,
            &collection_name,
            &token_name,
        )
        .await;

        let mut distribution_accounts = Vec::new();
        let mut txns = Vec::new();

        for _ in 0..num_workers {
            let distribution_account = LocalAccount::generate(&mut thread_rng());
            txns.push(create_nft_transfer_request(
                &mut creator_account,
                &distribution_account,
                creator_address,
                &collection_name,
                &token_name,
                &init_txn_factory,
                1_000_000_000,
            ));
            distribution_accounts.push(distribution_account);
        }

        info!("Creating {} NFTs", txns.len());
        // per account limit is 100
        for chunk in txns.chunks(100) {
            txn_executor.execute_transactions(chunk).await.unwrap();
        }
        info!("Done creating {} NFTs", txns.len());

        Self {
            txn_factory,
            creator_address,
            distribution_accounts,
            collection_name,
            token_name,
        }
    }
}

impl TransactionGeneratorCreator for NFTMintAndTransferGeneratorCreator {
    fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        Box::new(NFTMintAndTransfer::new(
            self.txn_factory.clone(),
            self.creator_address,
            self.distribution_accounts.pop().unwrap(),
            self.collection_name.clone(),
            self.token_name.clone(),
        ))
    }
}

fn create_and_fund_account_request(
    creation_account: &mut LocalAccount,
    amount: u64,
    pubkey: &Ed25519PublicKey,
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    let preimage = AuthenticationKeyPreimage::ed25519(pubkey);
    let auth_key = AuthenticationKey::from_preimage(&preimage);
    creation_account.sign_with_transaction_builder(txn_factory.payload(
        aptos_stdlib::aptos_account_transfer(auth_key.derived_address(), amount),
    ))
}
