// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::{CoreContext, Result, TestReport};
use diem_sdk::{
    client::BlockingClient,
    crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform},
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{
        account_config::xus_tag, chain_id::ChainId, transaction::authenticator::AuthenticationKey,
        LocalAccount,
    },
};
use diem_transaction_builder::experimental_stdlib;
use rand::{rngs::StdRng, SeedableRng};

/// The testing interface which defines a test written from the perspective of the a public user of
/// the NFT network in a "testnet" like environment where there exists a source for minting NFTs
/// and a means of creating new accounts.
pub trait NFTPublicUsageTest: Test {
    /// Executes the test against the given context.
    fn run<'t>(&self, ctx: &mut NFTPublicUsageContext<'t>) -> Result<()>;
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

    pub fn client(&self) -> BlockingClient {
        BlockingClient::new(&self.public_info.json_rpc_url)
    }

    pub fn url(&self) -> &str {
        &self.public_info.json_rpc_url
    }

    pub fn rest_api_url(&self) -> &str {
        &self.public_info.rest_api_url
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

    pub fn mint_bars(&mut self, address: AccountAddress, amount: u64) -> Result<()> {
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
        self.public_info.json_rpc_client.submit(&mint_nft_txn)?;
        self.public_info
            .json_rpc_client
            .wait_for_signed_transaction(&mint_nft_txn, None, None)?;
        Ok(())
    }

    pub fn create_user_account(&mut self, auth_key: AuthenticationKey) -> Result<()> {
        let create_account_txn = self.public_info.bars_account.sign_with_transaction_builder(
            self.transaction_factory().payload(
                experimental_stdlib::encode_create_account_script_function(
                    xus_tag(),
                    auth_key.derived_address(),
                    auth_key.prefix().to_vec(),
                ),
            ),
        );
        self.public_info
            .json_rpc_client
            .submit(&create_account_txn)?;
        self.public_info
            .json_rpc_client
            .wait_for_signed_transaction(&create_account_txn, None, None)?;
        Ok(())
    }
}

pub struct NFTPublicInfo<'t> {
    json_rpc_url: String,
    chain_id: ChainId,
    rest_api_url: String,
    json_rpc_client: BlockingClient,
    root_account: &'t mut LocalAccount,
    bars_account: LocalAccount,
}

impl<'t> NFTPublicInfo<'t> {
    pub fn new(
        json_rpc_url: String,
        chain_id: ChainId,
        rest_api_url: String,
        root_account: &'t mut LocalAccount,
    ) -> Self {
        let bars_key = Ed25519PrivateKey::generate(&mut StdRng::from_seed([1; 32]));
        let bars_address = AuthenticationKey::ed25519(&bars_key.public_key()).derived_address();
        Self {
            json_rpc_url: json_rpc_url.clone(),
            chain_id,
            rest_api_url,
            json_rpc_client: BlockingClient::new(json_rpc_url),
            root_account,
            bars_account: LocalAccount::new(bars_address, bars_key, 0),
        }
    }

    pub fn init_nft_environment(&mut self) -> Result<()> {
        // send a transaction to initialize nft
        let init_nft_txn = self.root_account.sign_with_transaction_builder(
            TransactionFactory::new(self.chain_id)
                .payload(experimental_stdlib::encode_nft_initialize_script_function()),
        );
        self.json_rpc_client.submit(&init_nft_txn)?;
        self.json_rpc_client
            .wait_for_signed_transaction(&init_nft_txn, None, None)?;

        // create an account for bars
        let bars_account_creation_txn = self.root_account.sign_with_transaction_builder(
            TransactionFactory::new(self.chain_id).payload(
                experimental_stdlib::encode_create_account_script_function(
                    xus_tag(),
                    self.bars_account.address(),
                    self.bars_account.authentication_key().prefix().to_vec(),
                ),
            ),
        );
        self.json_rpc_client.submit(&bars_account_creation_txn)?;
        self.json_rpc_client
            .wait_for_signed_transaction(&bars_account_creation_txn, None, None)?;

        Ok(())
    }
}
