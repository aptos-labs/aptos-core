// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    check_aptos_packages_availability, compile_aptos_packages, compile_package, is_aptos_package,
    DataManager, IndexReader, PackageInfo, TxnIndex, APTOS_COMMONS,
};
use anyhow::Result;
use aptos_framework::APTOS_PACKAGES;
use aptos_language_e2e_tests::{data_store::FakeDataStore, executor::FakeExecutor};
use aptos_types::{
    contract_event::ContractEvent,
    on_chain_config::{FeatureFlag, Features, OnChainConfig},
    transaction::{Transaction, TransactionPayload, Version},
    vm_status::VMStatus,
    write_set::WriteSet,
};
use aptos_vm::{data_cache::AsMoveResolver, transaction_metadata::TransactionMetadata};
use clap::ValueEnum;
use itertools::Itertools;
use move_compiler::compiled_unit::CompiledUnitEnum;
use move_core_types::account_address::AccountAddress;
use move_package::{compilation::compiled_package::CompiledPackage, CompilerVersion};
use std::{collections::HashMap, path::PathBuf};

fn load_packages_to_executor(
    executor: &mut FakeExecutor,
    package_info: &PackageInfo,
    compiled_package_cache: &mut HashMap<PackageInfo, CompiledPackage>,
) {
    let compiled_package = compiled_package_cache.get(package_info).unwrap();
    let root_modules = compiled_package.all_modules();
    for compiled_module in root_modules {
        if let CompiledUnitEnum::Module(module) = &compiled_module.unit {
            let module_blob = compiled_module.unit.serialize(None);
            executor.add_module(&module.module.self_id(), module_blob);
        }
    }
}

fn load_aptos_packages_to_executor(
    executor: &mut FakeExecutor,
    compiled_package_map: &mut HashMap<PackageInfo, CompiledPackage>,
) {
    for package in APTOS_PACKAGES {
        let package_info = PackageInfo {
            address: AccountAddress::ONE,
            package_name: package.to_string(),
            upgrade_number: None,
        };
        load_packages_to_executor(executor, &package_info, compiled_package_map);
    }
}

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub enum ExecutionMode {
    V1,
    V2,
    Compare,
}

impl ExecutionMode {
    pub fn is_v1(&self) -> bool {
        *self == Self::V1
    }

    pub fn is_v2(&self) -> bool {
        *self == Self::V2
    }

    pub fn is_compare(&self) -> bool {
        *self == Self::Compare
    }
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::V1
    }
}

pub struct Execution {
    input_path: PathBuf,
    execution_mode: ExecutionMode,
    bytecode_version: u32,
}

impl Execution {
    pub fn new(input_path: PathBuf, execution_mode: ExecutionMode) -> Self {
        Self {
            input_path,
            execution_mode,
            bytecode_version: 6,
        }
    }

    fn set_enable(features: &mut Features, flag: FeatureFlag) {
        let val = flag as u64;
        let byte_index = (val / 8) as usize;
        let bit_mask = 1 << (val % 8);
        if byte_index < features.features.len() {
            features.features[byte_index] |= bit_mask;
        }
    }

    pub async fn execute_txns(&self, begin: Version, limit: u64) -> Result<()> {
        let aptos_commons_path = self.input_path.join(APTOS_COMMONS);
        if !check_aptos_packages_availability(aptos_commons_path.clone()) {
            return Err(anyhow::Error::msg("aptos packages are missing"));
        }

        let mut compiled_package_cache: HashMap<PackageInfo, CompiledPackage> = HashMap::new();
        let mut compiled_package_cache_v2: HashMap<PackageInfo, CompiledPackage> = HashMap::new();
        if self.execution_mode.is_v1() || self.execution_mode.is_compare() {
            compile_aptos_packages(&aptos_commons_path, &mut compiled_package_cache, false)?;
        }
        if self.execution_mode.is_v2() || self.execution_mode.is_compare() {
            compile_aptos_packages(&aptos_commons_path, &mut compiled_package_cache_v2, true)?;
        }

        // prepare data
        let data_manager = DataManager::new(&self.input_path);
        if !data_manager.check_dir_availability() {
            return Err(anyhow::Error::msg("data is missing"));
        }
        if !IndexReader::check_availability(&self.input_path) {
            return Err(anyhow::Error::msg("index file is missing"));
        }
        let mut index_reader = IndexReader::new(&self.input_path);

        // get the first idx from the version_index file
        let ver = index_reader.get_next_version_ge(begin);
        if ver.is_none() {
            return Err(anyhow::Error::msg(
                "cannot find a version greater than or equal to the specified begin version",
            ));
        }
        let mut cur_version = ver.unwrap();
        while cur_version < begin + limit {
            self.execute_one_txn(
                cur_version,
                &data_manager,
                &mut compiled_package_cache,
                &mut compiled_package_cache_v2,
            )?;
            if let Some(ver) = index_reader.get_next_version() {
                cur_version = ver;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn compile_code(
        &self,
        txn_idx: &TxnIndex,
        compiled_package_cache: &mut HashMap<PackageInfo, CompiledPackage>,
        compiled_package_cache_v2: &mut HashMap<PackageInfo, CompiledPackage>,
    ) -> Result<()> {
        if !txn_idx.package_info.is_compilable() {
            return Err(anyhow::Error::msg("not compilable"));
        }
        let package_info = txn_idx.package_info.clone();
        let package_dir = self.input_path.join(format!("{}", package_info));
        if !package_dir.exists() {
            return Err(anyhow::Error::msg("source code is not available"));
        }
        if (self.execution_mode.is_compare() || self.execution_mode.is_v1())
            && !compiled_package_cache.contains_key(&package_info)
        {
            let compiled_res = compile_package(package_dir.clone(), &package_info, None)?;
            compiled_package_cache.insert(package_info.clone(), compiled_res);
        }
        if (self.execution_mode.is_compare() || self.execution_mode.is_v2())
            && !compiled_package_cache_v2.contains_key(&package_info)
        {
            let compiled_res =
                compile_package(package_dir, &package_info, Some(CompilerVersion::V2))?;
            compiled_package_cache_v2.insert(package_info.clone(), compiled_res);
        }
        Ok(())
    }

    fn execute_one_txn(
        &self,
        cur_version: Version,
        data_manager: &DataManager,
        compiled_package_cache: &mut HashMap<PackageInfo, CompiledPackage>,
        compiled_package_cache_v2: &mut HashMap<PackageInfo, CompiledPackage>,
    ) -> Result<()> {
        if let Some(txn_idx) = data_manager.get_txn_index(cur_version) {
            // compile the code if the source code is available
            if txn_idx.package_info.is_compilable()
                && !is_aptos_package(&txn_idx.package_info.package_name)
            {
                self.compile_code(&txn_idx, compiled_package_cache, compiled_package_cache_v2)?;
            }
            // read the state data;
            let state = data_manager.get_state(cur_version);
            let state_view = state.as_move_resolver();
            let mut features = Features::fetch_config(&state_view).unwrap_or_default();
            if self.bytecode_version == 6 {
                Self::set_enable(&mut features, FeatureFlag::VM_BINARY_FORMAT_V6);
            }
            // execute and compare
            self.execute_and_compare(
                cur_version,
                &state,
                &features,
                &txn_idx,
                compiled_package_cache,
                compiled_package_cache_v2,
            );
        }
        Ok(())
    }

    fn execute_and_compare(
        &self,
        cur_version: Version,
        state: &FakeDataStore,
        features: &Features,
        txn_idx: &TxnIndex,
        compiled_package_cache: &mut HashMap<PackageInfo, CompiledPackage>,
        compiled_package_cache_v2: &mut HashMap<PackageInfo, CompiledPackage>,
    ) {
        let mut res_1_opt = None;
        let mut res_2_opt = None;
        if self.execution_mode.is_v1() || self.execution_mode.is_compare() {
            res_1_opt = self.execute_code(
                state,
                features,
                &txn_idx.package_info,
                &txn_idx.txn,
                compiled_package_cache,
            );
        }
        if self.execution_mode.is_v2() || self.execution_mode.is_compare() {
            res_2_opt = self.execute_code(
                state,
                features,
                &txn_idx.package_info,
                &txn_idx.txn,
                compiled_package_cache_v2,
            );
        }
        if self.execution_mode.is_compare() {
            Self::print_mismatches(cur_version, &res_1_opt.unwrap(), &res_2_opt.unwrap());
        } else {
            let res = if let Some(res_1) = res_1_opt {
                res_1
            } else {
                res_2_opt.unwrap()
            };
            if let Ok(res_ok) = res {
                println!(
                    "version:{}\nwrite set:{:?}\n events:{:?}\n",
                    cur_version, res_ok.0, res_ok.1
                );
            } else {
                println!(
                    "execution error {} at version: {}, error",
                    res.unwrap_err(),
                    cur_version
                );
            }
        }
    }

    fn execute_code(
        &self,
        state: &FakeDataStore,
        features: &Features,
        package_info: &PackageInfo,
        txn: &Transaction,
        compiled_package_cache: &mut HashMap<PackageInfo, CompiledPackage>,
    ) -> Option<Result<(WriteSet, Vec<ContractEvent>), VMStatus>> {
        let executor = FakeExecutor::no_genesis();
        let mut executor = executor.set_not_parallel();
        *executor.data_store_mut() = state.clone();
        if let Transaction::UserTransaction(signed_trans) = txn {
            let sender = signed_trans.sender();
            let payload = signed_trans.payload();
            if let TransactionPayload::EntryFunction(entry_function) = payload {
                // always load 0x1 modules
                load_aptos_packages_to_executor(&mut executor, compiled_package_cache);
                // Load other modules
                if package_info.is_compilable() {
                    load_packages_to_executor(&mut executor, package_info, compiled_package_cache);
                }
                let mut senders = vec![sender];
                senders.extend(TransactionMetadata::new(signed_trans).secondary_signers);
                return Some(executor.try_exec_entry_with_features(
                    senders,
                    entry_function,
                    features,
                ));
            } else if let TransactionPayload::Multisig(multi_sig) = payload {
                assert!(multi_sig.transaction_payload.is_some());
                println!("Multisig transaction is not supported yet");
            }
        }
        None
    }

    fn print_mismatches(
        cur_version: u64,
        res_1: &Result<(WriteSet, Vec<ContractEvent>), VMStatus>,
        res_2: &Result<(WriteSet, Vec<ContractEvent>), VMStatus>,
    ) {
        if res_1.is_err() && res_2.is_err() {
            let res_1_err = res_1.as_ref().unwrap_err();
            let res_2_err = res_2.as_ref().unwrap_err();
            if res_1_err != res_2_err {
                println!("error is different at {}", cur_version);
                println!("error {} is raised from V1", res_1_err);
                println!("error {} is raised from V2", res_2_err);
            }
        } else if res_1.is_err() && res_2.is_ok() {
            println!(
                "error {} is raised from V1 at {}",
                res_1.as_ref().unwrap_err(),
                cur_version
            );
            let res_2_unwrapped = res_2.as_ref().unwrap();
            println!(
                "output from V2 at version:{}\nwrite set:{:?}\n events:{:?}\n",
                cur_version, res_2_unwrapped.0, res_2_unwrapped.1
            );
        } else if res_1.is_ok() && res_2.is_err() {
            println!(
                "error {} is raised from V2 at {}",
                res_2.as_ref().unwrap_err(),
                cur_version
            );
            let res_1_unwrapped = res_1.as_ref().unwrap();
            println!(
                "output from V1 at version:{}\nwrite set:{:?}\n events:{:?}\n",
                cur_version, res_1_unwrapped.0, res_1_unwrapped.1
            );
        } else {
            let res_1 = res_1.as_ref().unwrap();
            let res_2 = res_2.as_ref().unwrap();
            // compare events
            for idx in 0..res_1.1.len() {
                let event_1 = &res_1.1[idx];
                let event_2 = &res_2.1[idx];
                if event_1 != event_2 {
                    println!("event is different at version {}", cur_version);
                    println!("event raised from V1: {} at index:{}", event_1, idx);
                    println!("event raised from V2: {} at index:{}", event_2, idx);
                }
            }
            // compare write set
            let res_1_write_set_vec = res_1.0.iter().collect_vec();
            let res_2_write_set_vec = res_2.0.iter().collect_vec();
            for idx in 0..res_1_write_set_vec.len() {
                let write_set_1 = res_1_write_set_vec[0];
                let write_set_2 = res_2_write_set_vec[0];
                if write_set_1.0 != write_set_2.0 {
                    println!("write set key is different at version {}", cur_version);
                    println!("state key at V1: {:?} at index:{}", write_set_1.0, idx);
                    println!("state key at V2: {:?} at index:{}", write_set_2.0, idx);
                }
                if write_set_1.1 != write_set_2.1 {
                    println!("write set value is different at version {}", cur_version);
                    println!("state value at V1: {:?} at index {}", write_set_1.1, idx);
                    println!("state value at V2: {:?} at index {}", write_set_2.1, idx);
                }
            }
        }
    }
}
