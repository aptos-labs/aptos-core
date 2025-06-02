// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compile_aptos_packages, dump_and_compile_from_package_metadata, is_aptos_package,
    CompilationCache, ExecutionMode, IndexWriter, PackageInfo, TxnIndex, APTOS_COMMONS,
};
use anyhow::Result;
use aptos_framework::natives::code::PackageMetadata;
use aptos_rest_client::Client;
use aptos_transaction_simulation::InMemoryStateStore;
use aptos_types::transaction::Version;
use aptos_validator_interface::{AptosValidatorInterface, FilterCondition, RestDebuggerInterface};
use move_core_types::account_address::AccountAddress;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use url::Url;

pub struct OnlineExecutor {
    debugger: Arc<dyn AptosValidatorInterface + Send>,
    current_dir: PathBuf,
    batch_size: u64,
    filter_condition: FilterCondition,
    execution_mode: ExecutionMode,
    endpoint: String,
}

impl OnlineExecutor {
    pub fn new(
        debugger: Arc<dyn AptosValidatorInterface + Send>,
        current_dir: PathBuf,
        batch_size: u64,
        skip_failed_txns: bool,
        skip_publish_txns: bool,
        execution_mode: ExecutionMode,
        endpoint: String,
    ) -> Self {
        Self {
            debugger,
            current_dir,
            batch_size,
            filter_condition: FilterCondition {
                skip_failed_txns,
                skip_publish_txns,
                check_source_code: true,
                target_account: None,
            },
            execution_mode,
            endpoint,
        }
    }

    pub fn new_with_rest_client(
        rest_client: Client,
        current_dir: PathBuf,
        batch_size: u64,
        skip_failed_txns: bool,
        skip_publish_txns: bool,
        execution_mode: ExecutionMode,
        endpoint: String,
    ) -> Result<Self> {
        Ok(Self::new(
            Arc::new(RestDebuggerInterface::new(rest_client)),
            current_dir,
            batch_size,
            skip_failed_txns,
            skip_publish_txns,
            execution_mode,
            endpoint,
        ))
    }

    fn dump_and_check_src(
        version: Version,
        address: AccountAddress,
        package_name: String,
        map: HashMap<(AccountAddress, String), PackageMetadata>,
        compilation_cache: &mut CompilationCache,
        execution_mode: Option<ExecutionMode>,
        current_dir: PathBuf,
    ) -> Option<PackageInfo> {
        let upgrade_number = if is_aptos_package(&package_name) {
            None
        } else {
            let package = map.get(&(address, package_name.clone())).unwrap();
            Some(package.upgrade_number)
        };

        let package_info = PackageInfo {
            address,
            package_name: package_name.clone(),
            upgrade_number,
        };
        if compilation_cache.failed_packages_v1.contains(&package_info) {
            return None;
        }
        if !is_aptos_package(&package_name)
            && !compilation_cache
                .compiled_package_map
                .contains_key(&package_info)
        {
            let res = dump_and_compile_from_package_metadata(
                package_info.clone(),
                current_dir,
                &map,
                compilation_cache,
                execution_mode,
            );
            if res.is_err() {
                eprintln!("{} at:{}", res.unwrap_err(), version);
                return None;
            }
        }
        Some(package_info)
    }

    pub async fn execute(&self, begin: Version, limit: u64) -> Result<()> {
        println!("begin executing events");
        let compilation_cache = Arc::new(Mutex::new(CompilationCache::default()));
        let index_writer = Arc::new(Mutex::new(IndexWriter::new(&self.current_dir)));

        let aptos_commons_path = self.current_dir.join(APTOS_COMMONS);
        if self.execution_mode.is_v1_or_compare() {
            compile_aptos_packages(
                &aptos_commons_path,
                &mut compilation_cache.lock().unwrap().compiled_package_cache_v1,
                false,
            )?;
        }
        if self.execution_mode.is_v2_or_compare() {
            compile_aptos_packages(
                &aptos_commons_path,
                &mut compilation_cache.lock().unwrap().compiled_package_cache_v2,
                true,
            )?;
        }

        let mut cur_version = begin;
        let mut module_registry_map = HashMap::new();
        while cur_version < begin + limit {
            let batch = if cur_version + self.batch_size <= begin + limit {
                self.batch_size
            } else {
                begin + limit - cur_version
            };
            let res_txns = self
                .debugger
                .get_and_filter_committed_transactions(
                    cur_version,
                    batch,
                    self.filter_condition,
                    &mut module_registry_map,
                )
                .await;
            // if error happens when collecting txns, log the version range
            if res_txns.is_err() {
                index_writer.lock().unwrap().write_err(&format!(
                    "{}:{}:{:?}",
                    cur_version,
                    batch,
                    res_txns.unwrap_err()
                ));
                cur_version += batch;
                continue;
            }
            let txns = res_txns.unwrap_or_default();
            if !txns.is_empty() {
                let mut txn_execution_ths = vec![];
                for (version, txn, source_code_data) in txns {
                    println!("get txn at version:{}", version);

                    let compilation_cache = compilation_cache.clone();
                    let current_dir = self.current_dir.clone();
                    let execution_mode = self.execution_mode;
                    let endpoint = self.endpoint.clone();

                    let txn_execution_thread = tokio::task::spawn_blocking(move || {
                        let executor = crate::Execution::new(current_dir.clone(), execution_mode);

                        let mut version_idx = TxnIndex {
                            version,
                            txn: txn.clone(),
                            package_info: PackageInfo::non_compilable_info(),
                        };

                        // handle source code
                        if let Some((address, package_name, map)) = source_code_data {
                            let execution_mode_opt = Some(execution_mode);
                            let package_info_opt = Self::dump_and_check_src(
                                version,
                                address,
                                package_name,
                                map,
                                &mut compilation_cache.lock().unwrap(),
                                execution_mode_opt,
                                current_dir.clone(),
                            );
                            if package_info_opt.is_none() {
                                return;
                            }

                            version_idx.package_info = package_info_opt.unwrap();

                            let state_store = InMemoryStateStore::new();

                            let cache_v1 = compilation_cache
                                .lock()
                                .unwrap()
                                .compiled_package_cache_v1
                                .clone();
                            let cache_v2 = compilation_cache
                                .lock()
                                .unwrap()
                                .compiled_package_cache_v2
                                .clone();

                            let client = Client::new(Url::parse(&endpoint).unwrap());
                            let debugger = Arc::new(RestDebuggerInterface::new(client));
                            executor.execute_and_compare(
                                version,
                                state_store,
                                &version_idx,
                                &cache_v1,
                                &cache_v2,
                                Some(debugger),
                            );
                        }
                    });
                    txn_execution_ths.push(txn_execution_thread);
                }
                futures::future::join_all(txn_execution_ths).await;
            }
            cur_version += batch;
        }
        Ok(())
    }
}
