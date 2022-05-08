// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{TransactionContext, TransactionStatus};
use aptos_management::error::Error;
use aptos_rest_client::Client;
use aptos_types::{
    account_address::AccountAddress,
    account_config,
    account_config::AccountResource,
    on_chain_config::{access_path_for_config, OnChainConfig},
    transaction::SignedTransaction,
    validator_config::ValidatorConfig,
    validator_info::ValidatorInfo,
};
use move_deps::move_core_types::move_resource::MoveStructType;
use serde::de::DeserializeOwned;

/// A wrapper around JSON RPC for error handling
pub struct RestClient {
    client: Client,
}

impl RestClient {
    pub fn new(host: String) -> RestClient {
        RestClient {
            client: Client::new(url::Url::parse(&host).unwrap()),
        }
    }

    pub async fn submit_transaction(
        &self,
        transaction: SignedTransaction,
    ) -> Result<TransactionContext, Error> {
        let result = self.client.submit(&transaction).await;
        result.map_err(|e| Error::RestWriteError("transaction", e.to_string()))?;
        Ok(TransactionContext::new(
            transaction.sender(),
            transaction.sequence_number(),
        ))
    }

    pub async fn get_resource<T: DeserializeOwned>(
        &self,
        address: AccountAddress,
        resource_type: &str,
    ) -> Result<T, Error> {
        Ok(self
            .client
            .get_resource(address, resource_type)
            .await
            .map_err(|e| Error::RestReadError("get_resource", e.to_string()))?
            .into_inner())
    }

    pub async fn validator_config(
        &self,
        account: AccountAddress,
    ) -> Result<ValidatorConfig, Error> {
        let access_path = ValidatorConfig::struct_tag().access_vector();
        let resource_type = std::str::from_utf8(&access_path)
            .map_err(|e| Error::UnableToParse("Unable to form resource type", e.to_string()))?;

        let validator_config: ValidatorConfig = self.get_resource(account, resource_type).await?;
        resource("validator-config-resource", Ok(Some(validator_config)))
    }

    /// This method returns all validator infos currently registered in the validator set of the
    /// blockchain. If account is specified, only a single validator info is returned: the
    /// one that matches the given account.
    pub async fn validator_set(
        &self,
        account: Option<AccountAddress>,
    ) -> Result<Vec<ValidatorInfo>, Error> {
        let validator_set_account = account_config::validator_set_address();
        let access_path =
            access_path_for_config(aptos_types::on_chain_config::ValidatorSet::CONFIG_ID).path;
        let resource_type = std::str::from_utf8(&access_path)
            .map_err(|e| Error::RestReadError("Unable to form resource type", e.to_string()))?;

        let validator_set: aptos_types::on_chain_config::ValidatorSet = self
            .get_resource(validator_set_account, resource_type)
            .await?;

        let mut validator_infos = vec![];
        for validator_info in validator_set.payload() {
            if let Some(account) = account {
                if validator_info.account_address() == &account {
                    validator_infos.push(validator_info.clone());
                }
            } else {
                validator_infos.push(validator_info.clone());
            }
        }

        if validator_infos.is_empty() {
            return Err(Error::UnexpectedError(
                "No validator sets were found!".to_string(),
            ));
        }
        Ok(validator_infos)
    }

    pub async fn account_resource(
        &self,
        account: AccountAddress,
    ) -> Result<AccountResource, Error> {
        let access_path = AccountResource::struct_tag().access_vector();
        let resource_type = std::str::from_utf8(&access_path)
            .map_err(|e| Error::UnableToParse("Unable to form resource type", e.to_string()))?;

        let account_resource: AccountResource = self.get_resource(account, resource_type).await?;
        resource("account-resource", Ok(Some(account_resource)))
    }

    pub async fn sequence_number(&self, account: AccountAddress) -> Result<u64, Error> {
        Ok(self.account_resource(account).await?.sequence_number())
    }

    pub async fn transaction_status(
        &self,
        account: AccountAddress,
        sequence_number: u64,
    ) -> Result<Option<TransactionStatus>, Error> {
        let result = self
            .client
            .get_account_transactions(account, Some(sequence_number), Some(1))
            .await;
        let txns = result.map_err(|e| Error::RestReadError("transaction-status", e.to_string()))?;
        Ok(txns
            .inner()
            .first()
            .map(|txn| TransactionStatus::new(txn.vm_status(), txn.success())))
    }
}

fn resource<T>(
    resource_name: &'static str,
    maybe_resource: Result<Option<T>, anyhow::Error>,
) -> Result<T, Error> {
    match maybe_resource {
        Ok(Some(resource)) => Ok(resource),
        Ok(None) => Err(Error::RestReadError(
            resource_name,
            "not present".to_string(),
        )),
        Err(e) => Err(Error::RestReadError(resource_name, e.to_string())),
    }
}
