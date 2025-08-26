// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosValidatorInterface, FilterCondition};
use anyhow::{anyhow, Result};
use aptos_api_types::{AptosError, AptosErrorCode};
use aptos_framework::{
    natives::code::{PackageMetadata, PackageRegistry},
    APTOS_PACKAGES,
};
use aptos_rest_client::{
    error::{AptosErrorResponse, RestError},
    Client,
};
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        EntryFunction, ExecutionStatus::MiscellaneousError, PersistedAuxiliaryInfo, Transaction,
        TransactionExecutableRef, TransactionInfo, TransactionPayload, Version,
    },
};
use async_recursion::async_recursion;
use move_core_types::language_storage::ModuleId;
use std::collections::HashMap;

pub struct RestDebuggerInterface(Client);

impl RestDebuggerInterface {
    pub fn new(client: Client) -> Self {
        Self(client)
    }
}

#[async_recursion]
async fn retrieve_available_src(
    client: &Client,
    version: u64,
    package: &PackageMetadata,
    account_address: AccountAddress,
    data: &mut HashMap<(AccountAddress, String), PackageMetadata>,
    package_registry_cache: &mut HashMap<AccountAddress, PackageRegistry>,
) -> Result<()> {
    if package.modules.is_empty() || package.modules[0].source.is_empty() {
        Err(anyhow::anyhow!("source code is not available"))
    } else {
        let package_name = package.clone().name;
        if let std::collections::hash_map::Entry::Vacant(e) =
            data.entry((account_address, package_name.clone()))
        {
            e.insert(package.clone());
            retrieve_dep_packages_with_src(client, version, package, data, package_registry_cache)
                .await
        } else {
            Ok(())
        }
    }
}

#[async_recursion]
async fn get_or_update_package_registry<'a>(
    client: &Client,
    version: u64,
    addr: &AccountAddress,
    package_registry_cache: &'a mut HashMap<AccountAddress, PackageRegistry>,
) -> Result<&'a PackageRegistry> {
    if package_registry_cache.contains_key(addr) {
        Ok(package_registry_cache.get(addr).unwrap())
    } else {
        let packages = client
            .get_account_resource_at_version_bcs::<PackageRegistry>(
                *addr,
                "0x1::code::PackageRegistry",
                version,
            )
            .await?
            .into_inner();
        package_registry_cache.insert(*addr, packages);
        Ok(package_registry_cache.get(addr).unwrap())
    }
}

#[async_recursion]
async fn retrieve_dep_packages_with_src(
    client: &Client,
    version: u64,
    root_package: &PackageMetadata,
    data: &mut HashMap<(AccountAddress, String), PackageMetadata>,
    package_registry_cache: &mut HashMap<AccountAddress, PackageRegistry>,
) -> Result<()> {
    for dep in &root_package.deps {
        let package_registry =
            get_or_update_package_registry(client, version, &dep.account, package_registry_cache)
                .await?;
        for package in &package_registry.packages {
            if package.name == dep.package_name {
                retrieve_available_src(
                    client,
                    version,
                    &package.clone(),
                    dep.account,
                    data,
                    package_registry_cache,
                )
                .await?;
                break;
            }
        }
    }
    Ok(())
}

async fn check_and_obtain_source_code(
    client: &Client,
    m: &ModuleId,
    addr: &AccountAddress,
    version: Version,
    transaction: &Transaction,
    package_cache: &mut HashMap<
        ModuleId,
        (
            AccountAddress,
            String,
            HashMap<(AccountAddress, String), PackageMetadata>,
        ),
    >,
    txns: &mut Vec<(
        u64,
        Transaction,
        Option<(
            AccountAddress,
            String,
            HashMap<(AccountAddress, String), PackageMetadata>,
        )>,
    )>,
) -> Result<()> {
    let locate_package_with_src =
        |module: &ModuleId, packages: &[PackageMetadata]| -> Option<PackageMetadata> {
            for package in packages {
                for module_metadata in &package.modules {
                    if module_metadata.name == module.name().as_str() {
                        if module_metadata.source.is_empty() || package.upgrade_policy.policy == 0 {
                            return None;
                        } else {
                            return Some(package.clone());
                        }
                    }
                }
            }
            None
        };
    let mut package_registry_cache: HashMap<AccountAddress, PackageRegistry> = HashMap::new();
    let package_registry =
        get_or_update_package_registry(client, version, addr, &mut package_registry_cache).await?;
    let target_package_opt = locate_package_with_src(m, &package_registry.packages);
    if let Some(target_package) = target_package_opt {
        let mut map = HashMap::new();
        if APTOS_PACKAGES.contains(&target_package.name.as_str()) {
            package_cache.insert(
                m.clone(),
                (
                    AccountAddress::ONE,
                    target_package.name.clone(), // all aptos packages are stored under 0x1
                    HashMap::new(),
                ),
            );
            txns.push((
                version,
                transaction.clone(),
                Some((
                    AccountAddress::ONE,
                    target_package.name, // all aptos packages are stored under 0x1
                    HashMap::new(),
                )), // do not need to store the package registry for aptos packages
            ));
        } else if let Ok(()) = retrieve_dep_packages_with_src(
            client,
            version,
            &target_package,
            &mut map,
            &mut package_registry_cache,
        )
        .await
        {
            map.insert((*addr, target_package.clone().name), target_package.clone());
            package_cache.insert(m.clone(), (*addr, target_package.name.clone(), map.clone()));
            txns.push((
                version,
                transaction.clone(),
                Some((*addr, target_package.name, map)),
            ));
        }
    }
    Ok(())
}

#[async_trait::async_trait]
impl AptosValidatorInterface for RestDebuggerInterface {
    async fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        match self.0.get_raw_state_value(state_key, version).await {
            Ok(resp) => Ok(Some(bcs::from_bytes(&resp.into_inner())?)),
            Err(err) => match err {
                RestError::Api(AptosErrorResponse {
                    error:
                        AptosError {
                            error_code:
                                AptosErrorCode::StateValueNotFound | AptosErrorCode::TableItemNotFound, /* bug in pre 1.9 nodes */
                            ..
                        },
                    ..
                }) => Ok(None),
                _ => Err(anyhow!(err)),
            },
        }
    }

    async fn get_committed_transactions(
        &self,
        start: Version,
        limit: u64,
    ) -> Result<(
        Vec<Transaction>,
        Vec<TransactionInfo>,
        Vec<PersistedAuxiliaryInfo>,
    )> {
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

        // REST API doesn't provide auxiliary info, so return None for all transactions
        let auxiliary_infos = vec![PersistedAuxiliaryInfo::None; txns.len()];

        Ok((txns, txn_infos, auxiliary_infos))
    }

    async fn get_and_filter_committed_transactions(
        &self,
        start: Version,
        limit: u64,
        filter_condition: FilterCondition,
        package_cache: &mut HashMap<
            ModuleId,
            (
                AccountAddress,
                String,
                HashMap<(AccountAddress, String), PackageMetadata>,
            ),
        >,
    ) -> Result<
        Vec<(
            u64,
            Transaction,
            Option<(
                AccountAddress,
                String,
                HashMap<(AccountAddress, String), PackageMetadata>,
            )>,
        )>,
    > {
        let mut txns = Vec::with_capacity(limit as usize);
        let (tns, infos, _auxiliary_infos) = self.get_committed_transactions(start, limit).await?;
        let temp_txns = tns
            .iter()
            .zip(infos)
            .enumerate()
            .map(|(idx, (txn, txn_info))| {
                let version = start + idx as u64;
                (version, txn, txn_info)
            });
        let extract_entry_fun = |payload: &TransactionPayload| -> Option<EntryFunction> {
            match payload.executable_ref() {
                Ok(TransactionExecutableRef::EntryFunction(e)) => Some(e.clone()),
                _ => None,
            }
        };
        for (version, txn, txn_info) in temp_txns {
            if filter_condition.skip_failed_txns && !txn_info.status().is_success() {
                continue;
            }
            if let MiscellaneousError(_) = txn_info.status() {
                continue;
            }
            if let Transaction::UserTransaction(signed_trans) = txn.clone() {
                let payload = signed_trans.payload();
                if let Some(entry_function) = extract_entry_fun(payload) {
                    let m = entry_function.module();
                    let addr = m.address();
                    if filter_condition.target_account.is_some()
                        && filter_condition.target_account.unwrap() != *addr
                    {
                        continue;
                    }
                    if entry_function.function().as_str() == "publish_package_txn" {
                        if filter_condition.skip_publish_txns {
                            continue;
                        }
                        // For publish txn, we remove all items in the package_cache where module_id.address is the sender of this txn
                        // to update the new package in the cache.
                        package_cache.retain(|k, _| k.address != signed_trans.sender());
                    }
                    if !filter_condition.check_source_code {
                        txns.push((version, txn.clone(), None));
                    } else if package_cache.contains_key(m) {
                        txns.push((
                            version,
                            txn.clone(),
                            Some(package_cache.get(m).unwrap().clone()),
                        ));
                    } else {
                        check_and_obtain_source_code(
                            &self.0,
                            m,
                            addr,
                            version,
                            txn,
                            package_cache,
                            &mut txns,
                        )
                        .await?;
                    }
                }
            }
        }
        return Ok(txns);
    }

    async fn get_latest_ledger_info_version(&self) -> Result<Version> {
        Ok(self.0.get_ledger_information().await?.into_inner().version)
    }

    async fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>> {
        Ok(Some(
            self.0
                .get_account_ordered_transactions_bcs(account, Some(seq), None)
                .await?
                .into_inner()[0]
                .version,
        ))
    }

    async fn get_persisted_auxiliary_infos(
        &self,
        _start: Version,
        _limit: u64,
    ) -> Result<Vec<PersistedAuxiliaryInfo>> {
        Err(anyhow::anyhow!(
            "Getting persisted auxiliary infos is not supported via REST API. Use DB interface instead."
        ))
    }
}
