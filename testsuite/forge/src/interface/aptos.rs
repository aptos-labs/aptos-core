// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::{CoreContext, Result, TestReport};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    crypto::ed25519::Ed25519PublicKey,
    transaction_builder::TransactionFactory,
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::authenticator::{AuthenticationKey, AuthenticationKeyPreimage},
        LocalAccount,
    },
};
use aptos_transaction_builder::aptos_stdlib;
use reqwest::Url;

#[async_trait::async_trait]
pub trait AptosTest: Test {
    /// Executes the test against the given context.
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()>;
}

pub struct AptosContext<'t> {
    core: CoreContext,
    public_info: AptosPublicInfo<'t>,
    pub report: &'t mut TestReport,
}

impl<'t> AptosContext<'t> {
    pub fn new(
        core: CoreContext,
        public_info: AptosPublicInfo<'t>,
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

    pub async fn create_user_account(&mut self, pubkey: &Ed25519PublicKey) -> Result<()> {
        let preimage = AuthenticationKeyPreimage::ed25519(pubkey);
        let auth_key = AuthenticationKey::from_preimage(&preimage);
        let create_account_txn = self.public_info.root_account.sign_with_transaction_builder(
            self.transaction_factory().payload(
                aptos_stdlib::encode_create_account_script_function(
                    auth_key.derived_address(),
                    preimage.into_vec(),
                ),
            ),
        );
        self.public_info
            .rest_client
            .submit_and_wait(&create_account_txn)
            .await?;
        Ok(())
    }

    pub async fn mint(&mut self, addr: AccountAddress, amount: u64) -> Result<()> {
        let mint_txn = self.public_info.root_account.sign_with_transaction_builder(
            self.transaction_factory()
                .payload(aptos_stdlib::encode_mint_script_function(addr, amount)),
        );
        self.public_info
            .rest_client
            .submit_and_wait(&mint_txn)
            .await?;
        Ok(())
    }

    pub fn root_account(&mut self) -> &mut LocalAccount {
        &mut self.public_info.root_account
    }
}

pub struct AptosPublicInfo<'t> {
    chain_id: ChainId,
    rest_api_url: Url,
    rest_client: RestClient,
    root_account: &'t mut LocalAccount,
}

impl<'t> AptosPublicInfo<'t> {
    pub fn new(
        chain_id: ChainId,
        rest_api_url_str: String,
        root_account: &'t mut LocalAccount,
    ) -> Self {
        let rest_api_url = Url::parse(&rest_api_url_str).unwrap();
        Self {
            rest_client: RestClient::new(rest_api_url.clone()),
            rest_api_url,
            chain_id,
            root_account,
        }
    }
}
