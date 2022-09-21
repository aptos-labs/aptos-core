// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::{CoreContext, Result, TestReport};
use aptos_logger::info;
use aptos_rest_client::{Client as RestClient, PendingTransaction, State, Transaction};
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
use cached_packages::aptos_stdlib;
use rand::{rngs::OsRng, Rng, SeedableRng};
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
        let unit_price = std::cmp::max(aptos_global_constants::GAS_UNIT_PRICE, 1);
        TransactionFactory::new(self.chain_id()).with_gas_unit_price(unit_price)
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
    rng: ::rand::rngs::StdRng,
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
            rng: ::rand::rngs::StdRng::from_seed(OsRng.gen()),
        }
    }

    pub fn client(&self) -> &RestClient {
        &self.rest_client
    }

    pub fn url(&self) -> &str {
        self.rest_api_url.as_str()
    }

    pub fn root_account(&mut self) -> &mut LocalAccount {
        &mut self.root_account
    }

    pub async fn create_user_account(&mut self, pubkey: &Ed25519PublicKey) -> Result<()> {
        let preimage = AuthenticationKeyPreimage::ed25519(pubkey);
        let auth_key = AuthenticationKey::from_preimage(&preimage);
        let create_account_txn =
            self.root_account
                .sign_with_transaction_builder(self.transaction_factory().payload(
                    aptos_stdlib::aptos_account_create_account(auth_key.derived_address()),
                ));
        self.rest_client
            .submit_and_wait(&create_account_txn)
            .await?;
        Ok(())
    }

    pub async fn mint(&mut self, addr: AccountAddress, amount: u64) -> Result<()> {
        let mint_txn = self.root_account.sign_with_transaction_builder(
            self.transaction_factory()
                .payload(aptos_stdlib::aptos_coin_mint(addr, amount)),
        );
        self.rest_client.submit_and_wait(&mint_txn).await?;
        Ok(())
    }

    pub async fn transfer_non_blocking(
        &self,
        from_account: &mut LocalAccount,
        to_account: &LocalAccount,
        amount: u64,
    ) -> Result<PendingTransaction> {
        let tx = from_account.sign_with_transaction_builder(self.transaction_factory().payload(
            aptos_stdlib::aptos_coin_transfer(to_account.address(), amount),
        ));
        let pending_txn = self.rest_client.submit(&tx).await?.into_inner();
        Ok(pending_txn)
    }

    pub async fn transfer(
        &self,
        from_account: &mut LocalAccount,
        to_account: &LocalAccount,
        amount: u64,
    ) -> Result<PendingTransaction> {
        let pending_txn = self
            .transfer_non_blocking(from_account, to_account, amount)
            .await?;
        self.rest_client.wait_for_transaction(&pending_txn).await?;
        Ok(pending_txn)
    }

    pub fn transaction_factory(&self) -> TransactionFactory {
        let unit_price = std::cmp::max(aptos_global_constants::GAS_UNIT_PRICE, 1);
        TransactionFactory::new(self.chain_id).with_gas_unit_price(unit_price)
    }

    pub async fn get_balance(&self, address: AccountAddress) -> Option<u64> {
        let module = Identifier::new("coin".to_string()).unwrap();
        let name = Identifier::new("CoinStore".to_string()).unwrap();
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

    pub fn random_account(&mut self) -> LocalAccount {
        LocalAccount::generate(&mut self.rng)
    }

    pub async fn create_and_fund_user_account(&mut self, amount: u64) -> Result<LocalAccount> {
        let account = self.random_account();
        self.create_user_account(account.public_key()).await?;
        self.mint(account.address(), amount).await?;
        Ok(account)
    }

    pub async fn reconfig(&mut self) -> State {
        // dedupe with smoke-test::test_utils::reconfig
        reconfig(
            &self.rest_client,
            &self.transaction_factory(),
            self.root_account,
        )
        .await
    }
}

pub async fn reconfig(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    root_account: &mut LocalAccount,
) -> State {
    let aptos_version = client.get_aptos_version().await.unwrap();
    let (current, state) = aptos_version.into_parts();
    let current_version = *current.major.inner();
    let txn = root_account.sign_with_transaction_builder(
        transaction_factory
            .clone()
            .payload(aptos_stdlib::version_set_version(current_version + 1)),
    );
    let result = client.submit_and_wait(&txn).await;
    if let Err(e) = result {
        let last_transactions = client
            .get_account_transactions(root_account.address(), None, None)
            .await
            .map(|result| {
                result
                    .into_inner()
                    .iter()
                    .map(|t| {
                        if let Transaction::UserTransaction(ut) = t {
                            format!(
                                "user seq={}, payload={:?}",
                                ut.request.sequence_number, ut.request.payload
                            )
                        } else {
                            t.type_str().to_string()
                        }
                    })
                    .collect::<Vec<_>>()
            });

        panic!(
            "Couldn't execute {:?}, for account {:?}, error {:?}, last account transactions: {:?}",
            txn,
            root_account,
            e,
            last_transactions.unwrap_or_default()
        )
    }

    let transaction = result.unwrap();
    // Next transaction after reconfig should be a new epoch.
    let new_state = client
        .wait_for_version(transaction.inner().version().unwrap() + 1)
        .await
        .unwrap();

    info!(
        "Changed aptos version from {} (epoch={}, ledger_v={}), to {}, (epoch={}, ledger_v={})",
        current_version,
        state.epoch,
        state.version,
        current_version + 1,
        new_state.epoch,
        new_state.version
    );
    assert_ne!(state.epoch, new_state.epoch);

    new_state
}
