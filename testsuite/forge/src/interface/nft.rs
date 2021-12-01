// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::{CoreContext, Result, TestReport};
use diem_rest_client::Client as RestClient;
use diem_sdk::{
    crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform},
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey, LocalAccount},
};
use diem_transaction_builder::experimental_stdlib;
use rand::{rngs::StdRng, SeedableRng};
use reqwest::Url;

/// The testing interface which defines a test written from the perspective of the a public user of
/// the NFT network in a "testnet" like environment where there exists a source for minting NFTs
/// and a means of creating new accounts.
#[async_trait::async_trait]
pub trait NFTPublicUsageTest: Test {
    /// Executes the test against the given context.
    async fn run<'t>(&self, ctx: &mut NFTPublicUsageContext<'t>) -> Result<()>;
}

pub struct NFTPublicUsageContext<'t> {
    core: CoreContext,
    public_info: NFTPublicInfo<'t>,
    pub report: &'t mut TestReport,
}

impl<'t> NFTPublicUsageContext<'t> {
    pub fn new(
        core: CoreContext,
        public_info: NFTPublicInfo<'t>,
        report: &'t mut TestReport,
    ) -> Self {
        Self {
            core,
            public_info,
            report,
        }
    }

    pub fn client(&self) -> RestClient {
        RestClient::new(self.public_info.rest_api_url.clone())
    }

    pub fn url(&self) -> &str {
        self.public_info.rest_api_url.as_str()
    }

    pub fn core(&self) -> &CoreContext {
        &self.core
    }

    pub fn rng(&mut self) -> &mut ::rand::rngs::StdRng {
        self.core.rng()
    }

    pub fn random_account(&mut self) -> LocalAccount {
        LocalAccount::generate(self.core.rng())
    }

    pub fn chain_id(&self) -> ChainId {
        self.public_info.chain_id
    }

    pub fn transaction_factory(&self) -> TransactionFactory {
        TransactionFactory::new(self.chain_id())
    }

    pub async fn mint_bars(&mut self, address: AccountAddress, amount: u64) -> Result<()> {
        let mint_nft_txn = self.public_info.bars_account.sign_with_transaction_builder(
            self.transaction_factory().payload(
                experimental_stdlib::encode_mint_bars_script_function(
                    address,
                    "test_artist".as_bytes().to_vec(),
                    "test_url".as_bytes().to_vec(),
                    amount,
                ),
            ),
        );
        self.public_info
            .rest_client
            .submit_and_wait(&mint_nft_txn)
            .await?;
        Ok(())
    }

    pub async fn create_user_account(&mut self, auth_key: AuthenticationKey) -> Result<()> {
        let create_account_txn = self.public_info.bars_account.sign_with_transaction_builder(
            self.transaction_factory().payload(
                experimental_stdlib::encode_create_account_script_function(
                    auth_key.derived_address(),
                    auth_key.prefix().to_vec(),
                ),
            ),
        );
        self.public_info
            .rest_client
            .submit_and_wait(&create_account_txn)
            .await?;
        Ok(())
    }
}

pub struct NFTPublicInfo<'t> {
    chain_id: ChainId,
    rest_api_url: Url,
    rest_client: RestClient,
    root_account: &'t mut LocalAccount,
    bars_account: LocalAccount,
}

impl<'t> NFTPublicInfo<'t> {
    pub fn new(
        chain_id: ChainId,
        rest_api_url_str: String,
        root_account: &'t mut LocalAccount,
    ) -> Self {
        let bars_key = Ed25519PrivateKey::generate(&mut StdRng::from_seed([1; 32]));
        let bars_address = AuthenticationKey::ed25519(&bars_key.public_key()).derived_address();
        let rest_api_url = Url::parse(&rest_api_url_str).unwrap();
        Self {
            rest_client: RestClient::new(rest_api_url.clone()),
            rest_api_url,
            chain_id,
            root_account,
            bars_account: LocalAccount::new(bars_address, bars_key, 0),
        }
    }

    pub async fn init_nft_environment(&mut self) -> Result<()> {
        // send a transaction to initialize nft
        let init_nft_txn = self.root_account.sign_with_transaction_builder(
            TransactionFactory::new(self.chain_id)
                .payload(experimental_stdlib::encode_nft_initialize_script_function()),
        );
        self.rest_client.submit_and_wait(&init_nft_txn).await?;

        // create an account for bars
        let bars_account_creation_txn = self.root_account.sign_with_transaction_builder(
            TransactionFactory::new(self.chain_id).payload(
                experimental_stdlib::encode_create_account_script_function(
                    self.bars_account.address(),
                    self.bars_account.authentication_key().prefix().to_vec(),
                ),
            ),
        );
        self.rest_client
            .submit_and_wait(&bars_account_creation_txn)
            .await?;

        Ok(())
    }
}
