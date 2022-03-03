// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::{CoreContext, Result, TestReport};
use diem_rest_client::Client as RestClient;
use diem_sdk::{
    transaction_builder::TransactionFactory,
    types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey, LocalAccount},
};
use diem_transaction_builder::aptos_stdlib;
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

    pub async fn create_user_account(&mut self, auth_key: AuthenticationKey) -> Result<()> {
        let create_account_txn = self.public_info.root_account.sign_with_transaction_builder(
            self.transaction_factory().payload(
                aptos_stdlib::encode_create_account_script_function(
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
