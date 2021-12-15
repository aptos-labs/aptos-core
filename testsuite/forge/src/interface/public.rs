// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::{CoreContext, Result, TestReport};
use diem_rest_client::Client as RestClient;
use diem_sdk::{
    client::{BlockingClient, FaucetClient},
    move_types::account_address::AccountAddress,
    transaction_builder::{Currency, TransactionFactory},
    types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey, LocalAccount},
};
use reqwest::Url;

/// The testing interface which defines a test written from the perspective of the a public user of
/// the network in a "testnet" like environment where there exists a funding source and a means of
/// creating new accounts.
pub trait PublicUsageTest: Test {
    /// Executes the test against the given context.
    fn run<'t>(&self, ctx: &mut PublicUsageContext<'t>) -> Result<()>;
}

pub struct PublicUsageContext<'t> {
    core: CoreContext,

    public_info: PublicInfo<'t>,
    pub report: &'t mut TestReport,
}

impl<'t> PublicUsageContext<'t> {
    pub fn new(core: CoreContext, public_info: PublicInfo<'t>, report: &'t mut TestReport) -> Self {
        Self {
            core,
            public_info,
            report,
        }
    }

    pub fn client(&self) -> BlockingClient {
        BlockingClient::new(&self.public_info.json_rpc_url)
    }

    pub fn rest_client(&self) -> RestClient {
        RestClient::new(Url::parse(self.rest_api_url()).unwrap())
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

    pub async fn fund(&mut self, address: AccountAddress, amount: u64) -> Result<()> {
        self.public_info
            .coffer
            .fund(Currency::XUS, address, amount)
            .await
    }

    pub async fn create_parent_vasp_account(&mut self, auth_key: AuthenticationKey) -> Result<()> {
        self.public_info
            .coffer
            .create_parent_vasp_account(Currency::XUS, auth_key)
            .await
    }

    pub async fn create_designated_dealer_account(
        &mut self,
        auth_key: AuthenticationKey,
    ) -> Result<()> {
        self.public_info
            .coffer
            .create_designated_dealer_account(Currency::XUS, auth_key)
            .await
    }

    pub async fn transfer_coins(
        &mut self,
        currency: Currency,
        sender: &mut LocalAccount,
        payee: AccountAddress,
        amount: u64,
    ) -> Result<()> {
        let client = self.rest_client();
        let tx = sender.sign_with_transaction_builder(
            self.transaction_factory()
                .peer_to_peer(currency, payee, amount),
        );
        client.submit_and_wait(&tx).await?;

        Ok(())
    }
}

pub enum Coffer<'t> {
    TreasuryCompliance {
        transaction_factory: TransactionFactory,
        rest_client: RestClient,
        treasury_compliance_account: &'t mut LocalAccount,
        designated_dealer_account: &'t mut LocalAccount,
    },
    Faucet(FaucetClient),
}

#[async_trait::async_trait]
pub trait Fund {
    async fn fund(
        &mut self,
        currency: Currency,
        address: AccountAddress,
        amount: u64,
    ) -> Result<()>;
    async fn create_parent_vasp_account(
        &mut self,
        currency: Currency,
        auth_key: AuthenticationKey,
    ) -> Result<()>;
    async fn create_designated_dealer_account(
        &mut self,
        currency: Currency,
        auth_key: AuthenticationKey,
    ) -> Result<()>;
}

#[async_trait::async_trait]
impl Fund for Coffer<'_> {
    async fn fund(
        &mut self,
        currency: Currency,
        address: AccountAddress,
        amount: u64,
    ) -> Result<()> {
        match self {
            Coffer::Faucet(_) => todo!(),
            Coffer::TreasuryCompliance {
                transaction_factory,
                rest_client,
                treasury_compliance_account: _,
                designated_dealer_account,
            } => {
                let fund_account_txn = designated_dealer_account.sign_with_transaction_builder(
                    transaction_factory.peer_to_peer(currency, address, amount),
                );
                rest_client.submit_and_wait(&fund_account_txn).await?;
                Ok(())
            }
        }
    }

    async fn create_parent_vasp_account(
        &mut self,
        currency: Currency,
        auth_key: AuthenticationKey,
    ) -> Result<()> {
        match self {
            Coffer::Faucet(_) => todo!(),
            Coffer::TreasuryCompliance {
                transaction_factory,
                rest_client,
                treasury_compliance_account,
                ..
            } => {
                let create_account_txn = treasury_compliance_account.sign_with_transaction_builder(
                    transaction_factory.create_parent_vasp_account(
                        currency,
                        0,
                        auth_key,
                        &format!("No. {} VASP", treasury_compliance_account.sequence_number()),
                        false,
                    ),
                );
                rest_client.submit_and_wait(&create_account_txn).await?;
                Ok(())
            }
        }
    }

    async fn create_designated_dealer_account(
        &mut self,
        currency: Currency,
        auth_key: AuthenticationKey,
    ) -> Result<()> {
        match self {
            Coffer::Faucet(_) => todo!(),
            Coffer::TreasuryCompliance {
                transaction_factory,
                rest_client,
                treasury_compliance_account,
                ..
            } => {
                let create_account_txn = treasury_compliance_account.sign_with_transaction_builder(
                    transaction_factory.create_designated_dealer(
                        currency,
                        0, // sliding_nonce
                        auth_key,
                        &format!("No. {} DD", treasury_compliance_account.sequence_number()),
                        false, // add all currencies
                    ),
                );
                rest_client.submit_and_wait(&create_account_txn).await?;
                Ok(())
            }
        }
    }
}

pub struct PublicInfo<'t> {
    json_rpc_url: String,
    chain_id: ChainId,
    coffer: Coffer<'t>,
    rest_api_url: String,
}

impl<'t> PublicInfo<'t> {
    pub fn new(
        json_rpc_url: String,
        chain_id: ChainId,
        coffer: Coffer<'t>,
        rest_api_url: String,
    ) -> Self {
        Self {
            json_rpc_url,
            chain_id,
            coffer,
            rest_api_url,
        }
    }
}
