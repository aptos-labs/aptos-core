// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::{CoreContext, Result, TestReport};
use aptos_rest_client::{Client as RestClient, PendingTransaction};
use aptos_sdk::{
    crypto::ed25519::Ed25519PublicKey,
    move_types::identifier::Identifier,
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
        TransactionFactory::new(self.chain_id()).with_gas_unit_price(1)
    }

    pub fn aptos_transaction_factory(&self) -> TransactionFactory {
        self.public_info.transaction_factory()
    }

    pub async fn create_user_account(&mut self, pubkey: &Ed25519PublicKey) -> Result<()> {
        self.public_info.create_user_account(pubkey).await
    }

    pub async fn mint(&mut self, addr: AccountAddress, amount: u64) -> Result<()> {
        self.public_info.mint(addr, amount).await
    }

    pub async fn create_and_fund_user_account(&mut self, amount: u64) -> Result<LocalAccount> {
        let account = self.random_account();
        self.create_user_account(account.public_key()).await?;
        self.mint(account.address(), amount).await?;
        Ok(account)
    }

    pub async fn transfer(
        &self,
        from_account: &mut LocalAccount,
        to_account: &LocalAccount,
        amount: u64,
    ) -> Result<PendingTransaction> {
        self.public_info
            .transfer(from_account, to_account, amount)
            .await
    }

    pub async fn get_balance(&self, address: AccountAddress) -> Option<u64> {
        self.public_info.get_balance(address).await
    }

    pub fn root_account(&mut self) -> &mut LocalAccount {
        self.public_info.root_account
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

    pub async fn create_user_account(&mut self, pubkey: &Ed25519PublicKey) -> Result<()> {
        let preimage = AuthenticationKeyPreimage::ed25519(pubkey);
        let auth_key = AuthenticationKey::from_preimage(&preimage);
        let create_account_txn =
            self.root_account
                .sign_with_transaction_builder(self.transaction_factory().payload(
                    aptos_stdlib::encode_account_create_account(auth_key.derived_address()),
                ));
        self.rest_client
            .submit_and_wait(&create_account_txn)
            .await?;
        Ok(())
    }

    pub async fn mint(&mut self, addr: AccountAddress, amount: u64) -> Result<()> {
        let mint_txn = self.root_account.sign_with_transaction_builder(
            self.transaction_factory()
                .payload(aptos_stdlib::encode_test_coin_mint(addr, amount)),
        );
        self.rest_client.submit_and_wait(&mint_txn).await?;
        Ok(())
    }

    pub async fn transfer(
        &self,
        from_account: &mut LocalAccount,
        to_account: &LocalAccount,
        amount: u64,
    ) -> Result<PendingTransaction> {
        let tx = from_account.sign_with_transaction_builder(self.transaction_factory().payload(
            aptos_stdlib::encode_test_coin_transfer(to_account.address(), amount),
        ));
        let pending_txn = self.rest_client.submit(&tx).await?.into_inner();
        self.rest_client.wait_for_transaction(&pending_txn).await?;
        Ok(pending_txn)
    }

    pub fn transaction_factory(&self) -> TransactionFactory {
        TransactionFactory::new(self.chain_id)
            .with_gas_unit_price(1)
            .with_max_gas_amount(1000)
    }

    pub async fn get_balance(&self, address: AccountAddress) -> Option<u64> {
        let module = Identifier::new("TestCoin".to_string()).unwrap();
        let name = Identifier::new("Balance".to_string()).unwrap();
        self.rest_client
            .get_account_resources(address)
            .await
            .unwrap()
            .into_inner()
            .into_iter()
            .find(|r| r.resource_type.name == name && r.resource_type.module == module)
            .and_then(|coin| {
                coin.data
                    .get("coin")
                    .unwrap()
                    .get("value")
                    .unwrap()
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
            })
    }
}
