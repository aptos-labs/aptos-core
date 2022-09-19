// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_generator::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    transaction_builder::{aptos_stdlib::aptos_token_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};

use crate::emitter::{account_minter::create_and_fund_account_request, RETRY_POLICY};
use aptos_infallible::Mutex;
use aptos_logger::{info, warn};
use aptos_sdk::types::account_address::AccountAddress;
use async_trait::async_trait;
use rand::rngs::StdRng;
use rand::thread_rng;
use std::sync::Arc;

const INITIAL_NFT_BALANCE: u64 = 50_000;

pub struct NFTMintAndTransfer {
    txn_factory: TransactionFactory,
    creator_account: Arc<Mutex<LocalAccount>>,
    faucet_account: LocalAccount,
    collection_name: Vec<u8>,
    token_name: Vec<u8>,
}

impl NFTMintAndTransfer {
    pub fn new(
        txn_factory: TransactionFactory,
        creator_account: Arc<Mutex<LocalAccount>>,
        collection_name: Vec<u8>,
        token_name: Vec<u8>,
    ) -> Self {
        let faucet_account = LocalAccount::generate(&mut thread_rng());
        Self {
            txn_factory,
            faucet_account,
            creator_account,
            collection_name,
            token_name,
        }
    }
}

#[async_trait]
impl TransactionGenerator for NFTMintAndTransfer {
    async fn setup(&mut self, rest_client: &RestClient, accounts: &[LocalAccount]) {
        let creator_address = self.creator_account.lock().address();
        let fund_faucet_account_txn = create_nft_transfer_request(
            &mut self.creator_account.lock(),
            &self.faucet_account,
            creator_address,
            &self.collection_name,
            &self.token_name,
            &self.txn_factory,
            1_000_000_000,
        );
        submit_retry_and_wait(rest_client, &fund_faucet_account_txn).await;

        for account in accounts {
            let initial_nft_funding_txn = create_nft_transfer_request(
                &mut self.faucet_account,
                account,
                creator_address,
                &self.collection_name,
                &self.token_name,
                &self.txn_factory,
                INITIAL_NFT_BALANCE,
            );
            submit_retry_and_wait(rest_client, &initial_nft_funding_txn).await;
        }
    }

    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len() * transactions_per_account);
        for account in accounts {
            for i in 0..transactions_per_account {
                requests.push(create_nft_transfer_request(
                    account,
                    &self.faucet_account,
                    self.creator_account.lock().address(),
                    &self.collection_name,
                    &self.token_name,
                    &self.txn_factory,
                    if i != transactions_per_account - 1 {
                        1
                    } else {
                        INITIAL_NFT_BALANCE + 1 - transactions_per_account as u64
                    },
                ));
            }
        }
        requests
    }
}

async fn submit_retry_and_wait(rest_client: &RestClient, txn: &SignedTransaction) {
    let submit_result = RETRY_POLICY
        .retry(move || rest_client.submit_bcs(txn))
        .await;
    if let Err(e) = submit_result {
        warn!("Failed submitting transaction {:?} with {:?}", txn, e);
    }
    // if submission timeouts, it might still get committed:
    RETRY_POLICY
        .retry(move || rest_client.wait_for_signed_transaction_bcs(txn))
        .await
        .unwrap();
}

pub async fn initialize_nft_collection(
    rest_client: &RestClient,
    root_account: &mut LocalAccount,
    creator_account: &mut LocalAccount,
    txn_factory: &TransactionFactory,
    collection_name: &[u8],
    token_name: &[u8],
) {
    // resync root account sequence number
    match rest_client.get_account(root_account.address()).await {
        Ok(result) => {
            let account = result.into_inner();
            if root_account.sequence_number() < account.sequence_number {
                warn!(
                    "Root account sequence number got out of sync: remotely {}, locally {}",
                    account.sequence_number,
                    root_account.sequence_number_mut()
                );
                *root_account.sequence_number_mut() = account.sequence_number;
            }
        }
        Err(e) => warn!(
            "[{}] Couldn't check account sequence number due to {:?}",
            rest_client.path_prefix_string(),
            e
        ),
    }

    // Create and mint the owner account first
    let create_account_txn = create_and_fund_account_request(
        root_account,
        10_000_000,
        creator_account.public_key(),
        txn_factory,
    );

    submit_retry_and_wait(rest_client, &create_account_txn).await;

    info!("create_account_txn complete");

    let collection_txn =
        create_nft_collection_request(creator_account, collection_name, txn_factory);

    submit_retry_and_wait(rest_client, &collection_txn).await;

    info!("collection_txn complete");

    let token_txn =
        create_nft_token_request(creator_account, collection_name, token_name, txn_factory);

    submit_retry_and_wait(rest_client, &token_txn).await;

    info!("token_txn complete");
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
            0,
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
    creator_account: Arc<Mutex<LocalAccount>>,
    collection_name: Vec<u8>,
    token_name: Vec<u8>,
}

impl NFTMintAndTransferGeneratorCreator {
    pub async fn new(
        mut rng: StdRng,
        txn_factory: TransactionFactory,
        root_account: &mut LocalAccount,
        rest_client: RestClient,
    ) -> Self {
        let mut creator_account = LocalAccount::generate(&mut rng);
        let collection_name = "collection name".to_owned().into_bytes();
        let token_name = "token name".to_owned().into_bytes();
        initialize_nft_collection(
            &rest_client,
            root_account,
            &mut creator_account,
            &txn_factory,
            &collection_name,
            &token_name,
        )
        .await;
        Self {
            txn_factory,
            creator_account: Arc::new(Mutex::new(creator_account)),
            collection_name,
            token_name,
        }
    }
}

impl TransactionGeneratorCreator for NFTMintAndTransferGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(NFTMintAndTransfer::new(
            self.txn_factory.clone(),
            self.creator_account.clone(),
            self.collection_name.clone(),
            self.token_name.clone(),
        ))
    }
}
