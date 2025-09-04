// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    check_velor_packages_availability, compile_velor_packages, compile_package,
    data_state_view::DataStateView, generate_compiled_blob, is_velor_package, CompilationCache,
    DataManager, IndexReader, PackageInfo, TxnIndex, VELOR_COMMONS,
};
use anyhow::Result;
use velor_framework::VELOR_PACKAGES;
use velor_language_e2e_tests::executor::FakeExecutor;
use velor_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
use velor_types::{
    contract_event::ContractEvent,
    on_chain_config::{FeatureFlag, Features, OnChainConfig},
    transaction::{Transaction, Version},
    vm_status::VMStatus,
    write_set::WriteSet,
};
use velor_validator_interface::VelorValidatorInterface;
use clap::ValueEnum;
use itertools::Itertools;
use move_binary_format::file_format_common::VERSION_6;
use move_core_types::{account_address::AccountAddress, language_storage::ModuleId};
use move_model::metadata::CompilerVersion;
use std::{cmp, collections::HashMap, path::PathBuf, sync::Arc};

fn add_packages_to_state_store(
    state_store: &impl SimulationStateStore,
    package_info: &PackageInfo,
    compiled_package_cache: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
) {
    if !compiled_package_cache.contains_key(package_info) {
        return;
    }
    let compiled_package = compiled_package_cache.get(package_info).unwrap();
    for (module_id, module_blob) in compiled_package {
        state_store
            .add_module_blob(module_id, module_blob.clone())
            .expect("failed to add module blob, this should not happen");
    }
}

fn add_velor_packages_to_state_store(
    state_store: &impl SimulationStateStore,
    compiled_package_map: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
) {
    for package in VELOR_PACKAGES {
        let package_info = PackageInfo {
            address: AccountAddress::ONE,
            package_name: package.to_string(),
            upgrade_number: None,
        };
        add_packages_to_state_store(state_store, &package_info, compiled_package_map);
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

    pub fn is_v1_or_compare(&self) -> bool {
        self.is_v1() || self.is_compare()
    }

    pub fn is_v2_or_compare(&self) -> bool {
        self.is_v2() || self.is_compare()
    }
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::V1
    }
}

pub struct Execution {
    input_path: PathBuf,
    pub execution_mode: ExecutionMode,
    pub bytecode_version: u32,
}

impl Execution {
    pub fn output_result_str(&self, msg: String) {
        eprintln!("{}", msg);
    }

    pub fn new(input_path: PathBuf, execution_mode: ExecutionMode) -> Self {
        Self {
            input_path,
            execution_mode,
            bytecode_version: VERSION_6,
        }
    }

    pub async fn execute_txns(&self, begin: Version, num_txns_to_execute: u64) -> Result<()> {
        let velor_commons_path = self.input_path.join(VELOR_COMMONS);
        if !check_velor_packages_availability(velor_commons_path.clone()) {
            return Err(anyhow::Error::msg("velor packages are missing"));
        }

        let mut compiled_cache = CompilationCache::default();
        if self.execution_mode.is_v1_or_compare() {
            compile_velor_packages(
                &velor_commons_path,
                &mut compiled_cache.compiled_package_cache_v1,
                false,
            )?;
        }
        if self.execution_mode.is_v2_or_compare() {
            compile_velor_packages(
                &velor_commons_path,
                &mut compiled_cache.compiled_package_cache_v2,
                true,
            )?;
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
        let mut i = 0;
        while i < num_txns_to_execute {
            let res = self.execute_one_txn(cur_version, &data_manager, &mut compiled_cache);
            if res.is_err() {
                self.output_result_str(format!(
                    "execution at version:{} failed, skip to the next txn",
                    cur_version
                ));
            }
            let mut ver_res = index_reader.get_next_version();
            while ver_res.is_err() {
                ver_res = index_reader.get_next_version();
            }
            if ver_res.is_ok() {
                if let Some(ver) = ver_res.unwrap() {
                    cur_version = ver;
                } else {
                    break;
                }
            }
            i += 1;
        }
        Ok(())
    }

    fn compile_code(
        &self,
        txn_idx: &TxnIndex,
        compiled_cache: &mut CompilationCache,
    ) -> Result<()> {
        if !txn_idx.package_info.is_compilable() {
            return Err(anyhow::Error::msg("not compilable"));
        }
        let package_info = txn_idx.package_info.clone();
        let package_dir = self.input_path.join(format!("{}", package_info));
        if !package_dir.exists() {
            return Err(anyhow::Error::msg("source code is not available"));
        }
        let mut v1_failed = false;
        let mut v2_failed = false;
        if self.execution_mode.is_v1_or_compare()
            && !compiled_cache
                .compiled_package_cache_v1
                .contains_key(&package_info)
        {
            if compiled_cache.failed_packages_v1.contains(&package_info) {
                v1_failed = true;
            } else {
                let compiled_res_v1 = compile_package(
                    package_dir.clone(),
                    &package_info,
                    Some(CompilerVersion::V1),
                );
                if let Ok(compiled_res) = compiled_res_v1 {
                    generate_compiled_blob(
                        &package_info,
                        &compiled_res,
                        &mut compiled_cache.compiled_package_cache_v1,
                    );
                } else {
                    v1_failed = true;
                    compiled_cache
                        .failed_packages_v1
                        .insert(package_info.clone());
                }
            }
        }
        if self.execution_mode.is_v2_or_compare()
            && !compiled_cache
                .compiled_package_cache_v2
                .contains_key(&package_info)
        {
            if compiled_cache.failed_packages_v2.contains(&package_info) {
                v2_failed = true;
            } else {
                let compiled_res_v2 = compile_package(
                    package_dir,
                    &package_info,
                    Some(CompilerVersion::latest_stable()),
                );
                if let Ok(compiled_res) = compiled_res_v2 {
                    generate_compiled_blob(
                        &package_info,
                        &compiled_res,
                        &mut compiled_cache.compiled_package_cache_v2,
                    );
                } else {
                    v2_failed = true;
                    compiled_cache
                        .failed_packages_v2
                        .insert(package_info.clone());
                }
            }
        }
        if v1_failed || v2_failed {
            let mut err_msg = "compilation failed at ".to_string();
            if v1_failed {
                err_msg = format!("{} v1", err_msg);
            }
            if v2_failed {
                err_msg = format!("{} v2", err_msg);
            }
            return Err(anyhow::Error::msg(err_msg));
        }
        Ok(())
    }

    fn execute_one_txn(
        &self,
        cur_version: Version,
        data_manager: &DataManager,
        compiled_cache: &mut CompilationCache,
    ) -> Result<()> {
        if let Some(txn_idx) = data_manager.get_txn_index(cur_version) {
            // compile the code if the source code is available
            if txn_idx.package_info.is_compilable()
                && !is_velor_package(&txn_idx.package_info.package_name)
            {
                let compiled_result = self.compile_code(&txn_idx, compiled_cache);
                if compiled_result.is_err() {
                    self.output_result_str(format!(
                        "compilation failed for the package:{} at version:{}",
                        txn_idx.package_info.package_name, cur_version
                    ));
                    return compiled_result;
                }
            }
            // read the state data
            let state = data_manager.get_state(cur_version);
            self.execute_and_compare(
                cur_version,
                state,
                &txn_idx,
                &compiled_cache.compiled_package_cache_v1,
                &compiled_cache.compiled_package_cache_v2,
                None,
            );
        }
        Ok(())
    }

    pub(crate) fn execute_and_compare(
        &self,
        cur_version: Version,
        state: InMemoryStateStore,
        txn_idx: &TxnIndex,
        compiled_package_cache: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
        compiled_package_cache_v2: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
        debugger: Option<Arc<dyn VelorValidatorInterface + Send>>,
    ) {
        let mut package_cache_main = compiled_package_cache;
        let package_cache_other = compiled_package_cache_v2;
        let mut v2_flag = false;
        if self.execution_mode.is_v2() {
            package_cache_main = compiled_package_cache_v2;
            v2_flag = true;
        }
        let res_main = self.execute_code(
            cur_version,
            state.clone(),
            &txn_idx.package_info,
            &txn_idx.txn,
            package_cache_main,
            debugger.clone(),
            v2_flag,
        );
        if self.execution_mode.is_compare() {
            let res_other = self.execute_code(
                cur_version,
                state,
                &txn_idx.package_info,
                &txn_idx.txn,
                package_cache_other,
                debugger.clone(),
                true,
            );
            self.print_mismatches(cur_version, &res_main, &res_other);
        } else {
            match res_main {
                Ok((write_set, events)) => {
                    self.output_result_str(format!(
                        "version:{}\nwrite set:{:?}\n events:{:?}\n",
                        cur_version, write_set, events
                    ));
                },
                Err(vm_status) => {
                    self.output_result_str(format!(
                        "execution error {} at version: {}, error",
                        vm_status, cur_version
                    ));
                },
            }
        }
    }

    fn execute_code(
        &self,
        version: Version,
        state: InMemoryStateStore,
        package_info: &PackageInfo,
        txn: &Transaction,
        compiled_package_cache: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
        debugger_opt: Option<Arc<dyn VelorValidatorInterface + Send>>,
        v2_flag: bool,
    ) -> Result<(WriteSet, Vec<ContractEvent>), VMStatus> {
        // Always add Velor (0x1) packages.
        add_velor_packages_to_state_store(&state, compiled_package_cache);

        // Add other modules.
        if package_info.is_compilable() {
            add_packages_to_state_store(&state, package_info, compiled_package_cache);
        }

        // Update features if needed to the correct binary format used by V2 compiler.
        let mut features = Features::fetch_config(&state).unwrap_or_default();
        if v2_flag {
            features.enable(FeatureFlag::VM_BINARY_FORMAT_V8);
        } else {
            features.enable(FeatureFlag::VM_BINARY_FORMAT_V6);
        }
        state
            .set_features(features)
            .expect("failed to set features, this should not happen");

        // We use executor only to get access to block executor and avoid some of
        // the initializations, but ignore its internal state.
        let executor = FakeExecutor::no_genesis();
        let txns = vec![txn.clone()];

        if let Some(debugger) = debugger_opt {
            let data_view = DataStateView::new(debugger, version, state);
            executor
                .execute_transaction_block_with_state_view(txns, &data_view)
                .map(|mut res| res.pop().unwrap().into())
        } else {
            executor
                .execute_transaction_block_with_state_view(txns, &state)
                .map(|mut res| res.pop().unwrap().into())
        }
    }

    fn print_mismatches(
        &self,
        cur_version: u64,
        res_1: &Result<(WriteSet, Vec<ContractEvent>), VMStatus>,
        res_2: &Result<(WriteSet, Vec<ContractEvent>), VMStatus>,
    ) {
        match (res_1, res_2) {
            (Err(e1), Err(e2)) => {
                if e1.message() != e2.message() || e1.status_code() != e2.status_code() {
                    self.output_result_str(format!(
                        "error is different at version: {}",
                        cur_version
                    ));
                    self.output_result_str(format!("error {} is raised from V1", e1));
                    self.output_result_str(format!("error {} is raised from V2", e2));
                }
            },
            (Err(_), Ok(_)) => {
                self.output_result_str(format!(
                    "V1 returns error while V2 does not at version: {}",
                    cur_version
                ));
            },
            (Ok(_), Err(_)) => {
                self.output_result_str(format!(
                    "V2 returns error while V1 does not at version: {}",
                    cur_version
                ));
            },
            (Ok(res_1), Ok(res_2)) => {
                // compare events
                let mut event_error = false;
                if res_1.1.len() != res_2.1.len() {
                    event_error = true;
                }
                for idx in 0..cmp::min(res_1.1.len(), res_2.1.len()) {
                    let event_1 = &res_1.1[idx];
                    let event_2 = &res_2.1[idx];
                    if event_1 != event_2 {
                        event_error = true;
                        self.output_result_str(format!(
                            "event raised from V1: {} at index: {}",
                            event_1, idx
                        ));
                        self.output_result_str(format!(
                            "event raised from V2: {} at index: {}",
                            event_2, idx
                        ));
                    }
                }
                if event_error {
                    self.output_result_str(format!(
                        "event is different at version: {}",
                        cur_version
                    ));
                }
                // compare write set
                let mut write_set_error = false;
                let res_1_write_set_vec = res_1.0.write_op_iter().collect_vec();
                let res_2_write_set_vec = res_2.0.write_op_iter().collect_vec();
                if res_1_write_set_vec.len() != res_2_write_set_vec.len() {
                    write_set_error = true;
                }
                for idx in 0..cmp::min(res_1_write_set_vec.len(), res_2_write_set_vec.len()) {
                    let write_set_1 = res_1_write_set_vec[idx];
                    let write_set_2 = res_2_write_set_vec[idx];
                    if write_set_1.0 != write_set_2.0 {
                        write_set_error = true;
                        self.output_result_str(format!(
                            "write set key is different at version: {}, index: {}",
                            cur_version, idx
                        ));
                        self.output_result_str(format!(
                            "state key at V1: {:?} at index: {}",
                            write_set_1.0, idx
                        ));
                        self.output_result_str(format!(
                            "state key at V2: {:?} at index: {}",
                            write_set_2.0, idx
                        ));
                    }
                    if write_set_1.1 != write_set_2.1 {
                        write_set_error = true;
                        self.output_result_str(format!(
                            "write set value is different at version: {}, index: {}",
                            cur_version, idx
                        ));
                        self.output_result_str(format!(
                            "state value at V1: {:?} at index: {}",
                            write_set_1.1, idx
                        ));
                        self.output_result_str(format!(
                            "state value at V2: {:?} at index: {}",
                            write_set_2.1, idx
                        ));
                    }
                }
                if write_set_error {
                    self.output_result_str(format!(
                        "write set is different at version: {}",
                        cur_version
                    ));
                }
            },
        }
    }
}
