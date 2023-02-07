// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::AptosValidatorInterface;
use anyhow::{anyhow, bail, Result};
use aptos_api_types::MoveStructTag;
use aptos_rest_client::Client;
use aptos_types::{
    access_path::Path,
    account_address::AccountAddress,
    account_state::AccountState,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, TransactionInfo, Version},
};
use std::collections::BTreeMap;

pub struct RestDebuggerInterface(Client);

impl RestDebuggerInterface {
    pub fn new(client: Client) -> Self {
        Self(client)
    }
}

#[async_trait::async_trait]
impl AptosValidatorInterface for RestDebuggerInterface {
    async fn get_account_state_by_version(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<AccountState>> {
        let resource = self
            .0
            .get_account_resources_at_version_bcs(account, version)
            .await
            .map_err(|err| anyhow!("Failed to get account states: {:?}", err))?
            .into_inner()
            .into_iter()
            .map(|(key, value)| (key.access_vector(), value))
            .collect::<BTreeMap<_, _>>();

        Ok(Some(AccountState::new(account, resource)))
    }

    async fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        match state_key {
            StateKey::AccessPath(path) => match path.get_path() {
                Path::Code(module_id) => Ok(Some(StateValue::new(
                    self.0
                        .get_account_module_bcs_at_version(
                            *module_id.address(),
                            module_id.name().as_str(),
                            version,
                        )
                        .await
                        .map_err(|err| anyhow!("Failed to get account states: {:?}", err))?
                        .into_inner()
                        .to_vec(),
                ))),
                Path::Resource(tag) | Path::ResourceGroup(tag) => Ok(self
                    .0
                    .get_account_resource_at_version_bytes(
                        path.address,
                        MoveStructTag::from(tag).to_string().as_str(),
                        version,
                    )
                    .await
                    .ok()
                    .map(|inner| StateValue::new(inner.into_inner()))),
            },
            StateKey::TableItem { handle, key } => Ok(Some(StateValue::new(
                self.0
                    .get_raw_table_item(handle.0, key, version)
                    .await
                    .map_err(|err| anyhow!("Failed to get account states: {:?}", err))?
                    .into_inner(),
            ))),
            StateKey::Raw(_) => bail!("Unexpected key type"),
        }
    }

    async fn get_committed_transactions(
        &self,
        start: Version,
        limit: u64,
    ) -> Result<(Vec<Transaction>, Vec<TransactionInfo>)> {
        let mut txns = Vec::with_capacity(limit as usize);
        let mut txn_infos = Vec::with_capacity(limit as usize);

        while txns.len() < limit as usize {
            self.0
                .get_transactions_bcs(
                    Some(start + txns.len() as u64),
                    Some(limit as u16 - txns.len() as u16),
                )
                .await?
                .into_inner()
                .into_iter()
                .for_each(|txn| {
                    txns.push(txn.transaction);
                    txn_infos.push(txn.info);
                });
            println!("Got {}/{} txns from RestApi.", txns.len(), limit);
        }

        Ok((txns, txn_infos))
    }

    async fn get_latest_version(&self) -> Result<Version> {
        Ok(self.0.get_ledger_information().await?.into_inner().version)
    }

    async fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>> {
        Ok(Some(
            self.0
                .get_account_transactions_bcs(account, Some(seq), None)
                .await?
                .into_inner()[0]
                .version,
        ))
    }
}
