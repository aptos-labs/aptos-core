// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{TransactionContext, TransactionStatus};
use diem_management::error::Error;
use diem_rest_client::Client;
use diem_types::{
    account_address::AccountAddress, account_config, account_config::AccountResource,
    account_state::AccountState, account_state_blob::AccountStateBlob,
    transaction::SignedTransaction, validator_config::ValidatorConfigResource,
    validator_info::ValidatorInfo,
};
use std::convert::TryFrom;

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

    pub async fn account_state(&self, account: AccountAddress) -> Result<AccountState, Error> {
        let result = self.client.get_account_state_blob(account).await;
        let account_state_blob: AccountStateBlob = result
            .map_err(|e| Error::RestReadError("account-state", e.to_string()))?
            .inner()
            .clone()
            .into();

        AccountState::try_from(&account_state_blob)
            .map_err(|e| Error::RestReadError("account-state", e.to_string()))
    }

    pub async fn validator_config(
        &self,
        account: AccountAddress,
    ) -> Result<ValidatorConfigResource, Error> {
        resource(
            "validator-config-resource",
            self.account_state(account)
                .await?
                .get_validator_config_resource(),
        )
    }

    /// This method returns all validator infos currently registered in the validator set of the
    /// Diem blockchain. If account is specified, only a single validator info is returned: the
    /// one that matches the given account.
    pub async fn validator_set(
        &self,
        account: Option<AccountAddress>,
    ) -> Result<Vec<ValidatorInfo>, Error> {
        let validator_set_account = account_config::validator_set_address();
        let validator_set = self
            .account_state(validator_set_account)
            .await?
            .get_validator_set();

        match validator_set {
            Ok(Some(validator_set)) => {
                let mut validator_infos = vec![];
                for validator_info in validator_set.payload().iter() {
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
            Ok(None) => Err(Error::RestReadError(
                "validator-set",
                "not present".to_string(),
            )),
            Err(e) => Err(Error::RestReadError("validator-set", e.to_string())),
        }
    }

    pub async fn account_resource(
        &self,
        account: AccountAddress,
    ) -> Result<AccountResource, Error> {
        let account_state = self.account_state(account).await?;
        resource("account-resource", account_state.get_account_resource())
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
