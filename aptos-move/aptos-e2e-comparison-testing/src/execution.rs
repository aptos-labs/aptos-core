// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    check_aptos_packages_availability, compile_aptos_packages, compile_package, data_state_view::DataStateView, generate_compiled_blob, is_aptos_package, CompilationCache, DataManager, IndexReader, PackageInfo, TxnIndex, APTOS_COMMONS, APTOS_COMMONS_V2, DISABLE_REF_CHECK, DISABLE_SPEC_CHECK, ENABLE_REF_CHECK
};
use anyhow::Result;
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_framework::APTOS_PACKAGES;
use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
use aptos_types::{
    account_config::FungibleStoreResource,
    access_path::Path, account_config::ObjectGroupResource, block_executor::transaction_slice_metadata::TransactionSliceMetadata, contract_event::ContractEvent, on_chain_config::{FeatureFlag, Features, OnChainConfig}, state_store::state_key::{inner::StateKeyInner, StateKey}, transaction::{signature_verified_transaction::{
        into_signature_verified_block, SignatureVerifiedTransaction,
    }, Transaction, TransactionOutput, TransactionStatus, Version}, vm_status::VMStatus, write_set::{WriteSet, TOTAL_SUPPLY_STATE_KEY}
};
use aptos_validator_interface::AptosValidatorInterface;
use clap::ValueEnum;
use itertools::Itertools;
use move_binary_format::file_format_common::VERSION_DEFAULT;
use move_core_types::{account_address::AccountAddress, language_storage::{StructTag, ModuleId}, move_resource::MoveStructType};
use move_model::metadata::CompilerVersion;
use std::{cmp, collections::HashMap, env, path::PathBuf, sync::Arc};
use std::{
    sync::Mutex,
};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use aptos_types::block_executor::config::BlockExecutorConfigFromOnchain;
use aptos_types::account_config::{WithdrawFAEvent, DepositFAEvent};
use aptos_types::write_set::WriteOp;
use aptos_types::fee_statement::FeeStatement;
use std::collections::BTreeMap;
// use std::cmp::min;

const GAS_DIFF_PERCENTAGE: u64 = 3;
const TXNS_NUMBER: u64 = 1000;

pub fn add_packages_to_state_store(
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

pub fn add_aptos_packages_to_state_store(
    state_store: &impl SimulationStateStore,
    compiled_package_map: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
) {
    for package in APTOS_PACKAGES {
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
    pub skip_ref_packages: Option<String>,
}

impl Execution {
    pub fn check_package_skip(&self, package_name: &str) -> bool {
        println!("package name:{}", package_name);
        if let Some(p) = &self.skip_ref_packages {
            let packages = p.split(',').collect_vec();
            packages.contains(&package_name)
        } else {
            false
        }
    }

    // pub fn check_package_skip_alternative(skip_ref_packages: &Option<String>, package_name: &str) -> bool {
    //     println!("package name:{}", package_name);
    //     if let Some(p) = skip_ref_packages {
    //         let packages = p.split(',').collect_vec();
    //         packages.contains(&package_name)
    //     } else {
    //         false
    //     }
    // }

    pub fn output_result_str(&self, msg: String) {
        eprintln!("{}", msg);
    }

    // pub fn output_result_str_alternative(msg: String) {
    //     eprintln!("{}", msg);
    // }

    pub fn new(
        input_path: PathBuf,
        execution_mode: ExecutionMode,
        skip_ref_packages: Option<String>,
    ) -> Self {
        Self {
            input_path,
            execution_mode,
            bytecode_version: VERSION_DEFAULT,
            skip_ref_packages,
        }
    }

    pub async fn execute_txns(&self, begin: Version, num_txns_to_execute: u64) -> Result<()> {
        let aptos_commons_path = self.input_path.join(APTOS_COMMONS);
        if self.execution_mode.is_v1_or_compare() && !check_aptos_packages_availability(aptos_commons_path.clone()) {
            return Err(anyhow::Error::msg("aptos packages are missing"));
        }
        let aptos_commons_path_v2 = self.input_path.join(APTOS_COMMONS_V2);
        if self.execution_mode.is_v2_or_compare() && !check_aptos_packages_availability(aptos_commons_path_v2.clone()){
            return Err(anyhow::Error::msg("aptos packages are missing for v2"));
        }

        let mut compiled_cache = CompilationCache::default();
        if self.execution_mode.is_v1_or_compare() {
            compile_aptos_packages(
                &aptos_commons_path,
                &mut compiled_cache.compiled_package_cache_v1,
                false,
            )?;
        }
        if self.execution_mode.is_v2_or_compare() {
            compile_aptos_packages(
                &aptos_commons_path_v2,
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
            if let Ok(ver) = ver_res {
                if let Some(ver) = ver {
                    cur_version = ver;
                } else {
                    break;
                }
            }
            i += 1;
        }
        Ok(())
    }
    // pub async fn execute_txns(&self, begin: Version, num_txns_to_execute: u64) -> Result<()> {
    //     let aptos_commons_path = self.input_path.join(APTOS_COMMONS);
    //     if !check_aptos_packages_availability(aptos_commons_path.clone()) {
    //         return Err(anyhow::Error::msg("aptos packages are missing"));
    //     }

    //     let mut compiled_cache = CompilationCache::default();
    //     if self.execution_mode.is_v1_or_compare() {
    //         compile_aptos_packages(
    //             &aptos_commons_path,
    //             &mut compiled_cache.compiled_package_cache_v1,
    //             false,
    //         )?;
    //     }
    //     if self.execution_mode.is_v2_or_compare() {
    //         compile_aptos_packages(
    //             &aptos_commons_path,
    //             &mut compiled_cache.compiled_package_cache_v2,
    //             true,
    //         )?;
    //     }

    //     // prepare data
    //     let data_manager = DataManager::new(&self.input_path);
    //     if !data_manager.check_dir_availability() {
    //         return Err(anyhow::Error::msg("data is missing"));
    //     }
    //     if !IndexReader::check_availability(&self.input_path) {
    //         return Err(anyhow::Error::msg("index file is missing"));
    //     }
    //     let mut index_reader = IndexReader::new(&self.input_path);

    //     // get the first idx from the version_index file
    //     let ver = index_reader.get_next_version_ge(begin);
    //     if ver.is_none() {
    //         return Err(anyhow::Error::msg(
    //             "cannot find a version greater than or equal to the specified begin version",
    //         ));
    //     }
    //     let mut cur_version = ver.unwrap();
    //     let mut i = 0;
    //     if !self.execution_mode.is_compare() {
    //         while i < num_txns_to_execute {
    //             let res: std::result::Result<(), anyhow::Error> = self.execute_one_txn(cur_version, &data_manager, &mut compiled_cache);
    //             if res.is_err() {
    //                 self.output_result_str(format!(
    //                     "execution at version:{} failed, skip to the next txn",
    //                     cur_version
    //                 ));
    //             }
    //             let mut ver_res = index_reader.get_next_version();
    //             while ver_res.is_err() {
    //                 ver_res = index_reader.get_next_version();
    //             }
    //             if ver_res.is_ok() {
    //                 if let Some(ver) = ver_res.unwrap() {
    //                     cur_version = ver;
    //                 } else {
    //                     break;
    //                 }
    //             }
    //             i += 1;
    //         }
    //     } else {
    //         // prepare_data_state
    //         let mut data_state = vec![];
    //         let mut versions = vec![];
    //         let cache_arc: Arc<Mutex<CompilationCache>> = Arc::new(Mutex::new(compiled_cache));
    //         while i < num_txns_to_execute {
    //             let mut j = 0;
    //             let mut finish_early: bool = false;
    //             while j < std::cmp::min(num_txns_to_execute - i, TXNS_NUMBER) {
    //                 Self::prepare_data_state(cur_version, &data_manager, &mut cache_arc.lock().unwrap(), self.input_path.clone(), &mut versions, &mut data_state, self.skip_ref_packages.clone(),
    //                 &self.execution_mode);
    //                 let mut ver_res = index_reader.get_next_version();
    //                 while ver_res.is_err() {
    //                     ver_res = index_reader.get_next_version();
    //                 }
    //                 if let Some(ver) = ver_res.unwrap() {
    //                     cur_version = ver;
    //                 } else {
    //                     finish_early = true;
    //                     break;
    //                 }
    //                 i += 1;
    //                 j += 1;
    //             }
    //             let data_state_copy: Arc<Vec<(u64, TxnIndex, FakeDataStore)>> = Arc::new(data_state);
    //             // let cache_copy= cache_arc.clone();
    //             let cache_copy_v1: HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>= cache_arc.clone().lock().unwrap().compiled_package_cache_v1.clone();
    //             let cache_copy_v2: HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>= cache_arc.clone().lock().unwrap().compiled_package_cache_v2.clone();
    //             let data_state_c: Arc<Vec<(u64, TxnIndex, FakeDataStore)>>= data_state_copy.clone();
    //             let res_1: Arc<Mutex<Vec<(u64, std::result::Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>)>>> = Arc::new(Mutex::new(vec![]));
    //             let res_1_copy: Arc<Mutex<Vec<(u64, std::result::Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>)>>> = res_1.clone();

    //             // let handle_v1 = std::thread::spawn(move || {
    //             //     for (v, txn_index, state) in data_state_c.iter() {
    //             //         let res = Self::execute_one_txn_with_result_alternative(*v, state, txn_index, &cache_copy_v1);
    //             //         res_1_copy.lock().unwrap().push((*v, res));
    //             //     }
    //             // });

    //             for (v, txn_index, state) in data_state_c.iter() {
    //                 let res = Self::execute_one_txn_with_result_alternative(*v, state, txn_index, &cache_copy_v1);
    //                 println!("res:{:?} at version:{}", res, v);
    //                 res_1_copy.lock().unwrap().push((*v, res));
    //             }

    //             // let res_2: Arc<Mutex<Vec<(u64, std::result::Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>)>>> = Arc::new(Mutex::new(vec![]));
    //             // let res_2_copy: Arc<Mutex<Vec<(u64, std::result::Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>)>>> = res_2.clone();
    //             // let handle_v2 = std::thread::spawn(move || {
    //             //     for (v, txn_index, state) in data_state_copy.iter() {
    //             //         //let res = Self::execute_one_txn_with_result_alternative(*v, state, txn_index, &cache_copy_v2);
    //             //         //res_2_copy.lock().unwrap().push((*v, res));
    //             //     }
    //             // });
    //             // handle_v1.join().unwrap();
    //             // handle_v2.join().unwrap();
    //             // for ((v_1, r_1), (v_2, r_2)) in res_1.lock().unwrap().iter().zip(res_2.lock().unwrap().iter()) {
    //             //     if v_1 == v_2 {
    //             //         self.print_mismatches(*v_1, r_1, r_2, None);
    //             //     } else {
    //             //         eprint!("v1:{}, v2:{}", v_1, v_2);
    //             //     }
    //             // }
    //             data_state = vec![];
    //             versions = vec![];
    //             if finish_early {
    //                 break;
    //             }
    //             // let mut ver_res = index_reader.get_next_version();
    //             // while ver_res.is_err() {
    //             //     ver_res = index_reader.get_next_version();
    //             // }
    //             // if ver_res.is_ok() {
    //             //     if let Some(ver) = ver_res.unwrap() {
    //             //         cur_version = ver;
    //             //     } else {
    //             //         break;
    //             //     }
    //             // }
    //             // i += 1;
    //         }
    //         // let cache_copy: Arc<CompilationCache> = Arc::new(compiled_cache);
    //         // let cache_copy_c: Arc<CompilationCache> = cache_copy.clone();
    //         // let data_state_copy: Arc<Vec<(u64, TxnIndex, FakeDataStore)>> = Arc::new(data_state);
    //         // let data_state_c= data_state_copy.clone();
    //         // let res_1: Arc<Mutex<Vec<(u64, std::result::Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>)>>> = Arc::new(Mutex::new(vec![]));
    //         // let res_1_copy: Arc<Mutex<Vec<(u64, std::result::Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>)>>> = res_1.clone();
    //         // let handle_v1 = std::thread::spawn(move || {
    //         //     for (v, txn_index, state) in data_state_c.iter() {
    //         //         let res = Self::execute_one_txn_with_result_alternative(*v, state, txn_index, &cache_copy_c.compiled_package_cache_v1);
    //         //         res_1_copy.lock().unwrap().push((*v, res));
    //         //         //println!("v1 version:{}", v);
    //         //     }
    //         // });

    //         // let res_2: Arc<Mutex<Vec<(u64, std::result::Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>)>>> = Arc::new(Mutex::new(vec![]));
    //         // let res_2_copy: Arc<Mutex<Vec<(u64, std::result::Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>)>>> = res_2.clone();
    //         // let handle_v2 = std::thread::spawn(move || {
    //         //     for (v, txn_index, state) in data_state_copy.iter() {
    //         //         let res = Self::execute_one_txn_with_result_alternative(*v, state, txn_index, &cache_copy.compiled_package_cache_v2);
    //         //         res_2_copy.lock().unwrap().push((*v, res));
    //         //         //println!("v2 version:{}", v);
    //         //     }
    //         // });
    //         // handle_v1.join().unwrap();
    //         // handle_v2.join().unwrap();
    //         // for ((v_1, r_1), (v_2, r_2)) in res_1.lock().unwrap().iter().zip(res_2.lock().unwrap().iter()) {
    //         //     if v_1 == v_2 {
    //         //         self.print_mismatches(*v_1, r_1, r_2, None);
    //         //     } else {
    //         //         eprint!("v1:{}, v2:{}", v_1, v_2);
    //         //     }
    //         // }
    //     }
    //     Ok(())
    // }

    // fn compile_code_alternative(
    //     input_path: PathBuf,
    //     txn_idx: &TxnIndex,
    //     compiled_cache: &mut CompilationCache,
    //     execution_mode: &ExecutionMode,
    //     skip_ref_packages: &Option<String>,
    // ) -> Result<()> {
    //     if !txn_idx.package_info.is_compilable() {
    //         return Err(anyhow::Error::msg("not compilable"));
    //     }
    //     let package_info = txn_idx.package_info.clone();
    //     let package_dir = input_path.join(format!("{}", package_info));
    //     if !package_dir.exists() {
    //         return Err(anyhow::Error::msg("source code is not available"));
    //     }
    //     let mut v1_failed = false;
    //     let mut v2_failed = false;
    //     if execution_mode.is_v1_or_compare()
    //         && !compiled_cache
    //             .compiled_package_cache_v1
    //             .contains_key(&package_info)
    //     {
    //         if compiled_cache.failed_packages_v1.contains(&package_info) {
    //             v1_failed = true;
    //         } else {
    //             let compiled_res_v1 = compile_package(
    //                 package_dir.clone(),
    //                 &package_info,
    //                 Some(CompilerVersion::latest_stable()),
    //                 vec![],
    //             );
    //             if let Ok(compiled_res) = compiled_res_v1 {
    //                 generate_compiled_blob(
    //                     &package_info,
    //                     &compiled_res,
    //                     &mut compiled_cache.compiled_package_cache_v1,
    //                 );
    //             } else {
    //                 v1_failed = true;
    //                 compiled_cache
    //                     .failed_packages_v1
    //                     .insert(package_info.clone());
    //             }
    //         }
    //     }
    //     if execution_mode.is_v2_or_compare()
    //         && !compiled_cache
    //             .compiled_package_cache_v2
    //             .contains_key(&package_info)
    //     {
    //         if compiled_cache.failed_packages_v2.contains(&package_info) {
    //             v2_failed = true;
    //         } else {
    //             // if Self::check_package_skip_alternative(skip_ref_packages, &package_info.package_name) {
    //             //     env::set_var(
    //             //         "MOVE_COMPILER_EXP",
    //             //         format!("{},{}", DISABLE_SPEC_CHECK, DISABLE_REF_CHECK),
    //             //     );
    //             // } else {
    //             //     env::set_var(
    //             //         "MOVE_COMPILER_EXP",
    //             //         format!("{},{}", DISABLE_SPEC_CHECK, ENABLE_REF_CHECK),
    //             //     );
    //             // }
    //             let compiled_res_v2 =
    //                 compile_package(package_dir, &package_info, Some(CompilerVersion::latest()), vec![]);
    //             if let Ok(compiled_res) = compiled_res_v2 {
    //                 generate_compiled_blob(
    //                     &package_info,
    //                     &compiled_res,
    //                     &mut compiled_cache.compiled_package_cache_v2,
    //                 );
    //             } else {
    //                 v2_failed = true;
    //                 compiled_cache
    //                     .failed_packages_v2
    //                     .insert(package_info.clone());
    //             }
    //         }
    //     }
    //     if v1_failed || v2_failed {
    //         let mut err_msg = format!(
    //             "compilation for the package {} failed at",
    //             package_info.package_name
    //         );
    //         if v1_failed {
    //             err_msg = format!("{} v1", err_msg);
    //         }
    //         if v2_failed {
    //             err_msg = format!("{} v2", err_msg);
    //         }
    //         return Err(anyhow::Error::msg(err_msg));
    //     }
    //     Ok(())
    // }

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
                    Some(CompilerVersion::latest_stable()),
                    vec![],
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
                // if self.check_package_skip(&package_info.package_name) {
                //     env::set_var(
                //         "MOVE_COMPILER_EXP",
                //         format!("{},{}", DISABLE_SPEC_CHECK, DISABLE_REF_CHECK),
                //     );
                // } else {
                //     env::set_var(
                //         "MOVE_COMPILER_EXP",
                //         format!("{},{}", DISABLE_SPEC_CHECK, ENABLE_REF_CHECK),
                //     );
                // }
                let compiled_res_v2 =
                    compile_package(package_dir, &package_info, Some(CompilerVersion::latest()), vec![]);
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
            let mut err_msg = format!(
                "compilation for the package {} failed at",
                package_info.package_name
            );
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
        if let Some(mut txn_idx) = data_manager.get_txn_index(cur_version) {
            // compile the code if the source code is available
            if txn_idx.package_info.is_compilable()
                && !is_aptos_package(&txn_idx.package_info.package_name)
            {
                let compiled_result = self.compile_code(&txn_idx, compiled_cache);
                if compiled_result.is_err() {
                    let err = compiled_result.unwrap_err();
                    self.output_result_str(format!("{} at version:{}", err, cur_version));
                    return Err(err);
                }
            }
            // read the state data
            let state = data_manager.get_state(cur_version);
            self.execute_and_compare(
                cur_version,
                state,
                &mut txn_idx,
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
        txn_idx: &mut TxnIndex,
        compiled_package_cache: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
        compiled_package_cache_v2: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
        debugger: Option<Arc<dyn AptosValidatorInterface + Send>>,
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
            &mut txn_idx.txn,
            package_cache_main,
            debugger.clone(),
            v2_flag,
        );
        if self.execution_mode.is_compare() {
            let res_other = self.execute_code(
                cur_version,
                state,
                &txn_idx.package_info,
                &mut txn_idx.txn,
                package_cache_other,
                debugger.clone(),
                true,
            );
            self.print_mismatches(
                cur_version,
                &res_main,
                &res_other,
                Some(txn_idx.package_info.package_name.clone()),
            );
        } else {
            match res_main {
                Ok(((write_set, events), txn_status, gas)) => {
                    self.output_result_str(format!(
                        "version:{}\nwrite set:{:?}\n events:{:?}, txn_status:{:?}, gas:{}\n",
                        cur_version, write_set, events, txn_status, gas
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
        mut state: InMemoryStateStore,
        package_info: &PackageInfo,
        txn: &mut Transaction,
        compiled_package_cache: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
        debugger_opt: Option<Arc<dyn AptosValidatorInterface + Send>>,
        v2_flag: bool
    ) -> Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus> {
        // Always add Aptos (0x1) packages.
        add_aptos_packages_to_state_store(&mut state, compiled_package_cache);

        // Add other modules.
        if package_info.is_compilable() {
            add_packages_to_state_store(&mut state, package_info, compiled_package_cache);
        }

        // Update features if needed to the correct binary format used by V2 compiler.
        let mut features = Features::fetch_config(&state).unwrap_or_default();
        features.enable(FeatureFlag::VM_BINARY_FORMAT_V7);
        features.enable(FeatureFlag::NATIVE_MEMORY_OPERATIONS);
        features.disable(FeatureFlag::ACCOUNT_ABSTRACTION);
        if v2_flag {
            features.enable(FeatureFlag::VM_BINARY_FORMAT_V9);
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
                .execute_transaction_block_with_state_view(txns, &data_view, false)
                .map(|mut res| {
                    let res_i = res.pop().unwrap();
                    (
                        res_i.clone().into(),
                        res_i.status().clone(),
                        res_i.gas_used(),
                    )
                })
        } else {
            let res = executor
                .execute_transaction_block_with_state_view(txns, &state, false)
                .map(|mut res| {
                    let res_i = res.pop().unwrap();
                    // println!(
                    //     "v2 flag:{} gas used:{}, status:{:?}",
                    //     v2_flag,
                    //     res_i.gas_used(),
                    //     res_i.status()
                    // );
                    (
                        res_i.clone().into(),
                        res_i.status().clone(),
                        res_i.gas_used(),
                    )
                });
            res
        }
    }

    fn is_fee_related_stake_key(&self, key: &StateKey) -> bool {
        if let StateKeyInner::AccessPath(p) = key.inner() {
            let path = p.get_path();
            if let Path::Resource(tag) = path {
                if tag.name.as_str() == "CoinStore" && !tag.type_args.is_empty() {
                    let para_type = &tag.type_args[0];
                    if para_type.to_canonical_string() == "0x1::aptos_coin::AptosCoin" {
                        return true;
                    }
                }
            }
        }
        *key == *TOTAL_SUPPLY_STATE_KEY
    }

    fn filter_stake_key_resource_group(&self, key: &StateKey, value_1: &WriteOp, value_2: &WriteOp) -> bool {
        if let StateKeyInner::AccessPath(p) = key.inner() {
            let path = p.get_path();
            if let Path::ResourceGroup(tag) = path {
                let state_value_1_opt = value_1.as_state_value_opt();
                let state_value_2_opt = value_2.as_state_value_opt();
                if let (Some(start_value_1), Some(start_value_2)) = (state_value_1_opt, state_value_2_opt) {
                    if tag == ObjectGroupResource::struct_tag() {
                        let byte_map_1: BTreeMap<StructTag, Vec<u8>> =
                        bcs::from_bytes(&start_value_1.bytes().to_vec()).unwrap();
                        let byte_map_2: BTreeMap<StructTag, Vec<u8>> =
                        bcs::from_bytes(&start_value_1.bytes().to_vec()).unwrap();
                        if byte_map_1.len() != byte_map_2.len() {
                            return false;
                        }
                        for tag_1 in byte_map_1.keys() {
                            if !byte_map_2.contains_key(tag_1) {
                                return false;
                            }
                            if tag_1.name.as_str() != "ConcurrentSupply" || tag_1.name.as_str() != "FungibleStore" {
                                if byte_map_1.get(tag_1).unwrap() != byte_map_1.get(tag_1).unwrap() {
                                    return false;
                                }
                             }
                        }
                        return true;
                    }
                } else {
                    return false;
                }
            }
        }
        false
    }

    fn is_fee_statement_event_key(&self, event: &ContractEvent) -> bool {
        event.type_tag().to_canonical_string() == "0x1::transaction_fee::FeeStatement"
    }

    fn is_withdraw_event_key(&self, event: &ContractEvent) -> bool {
        event.type_tag().to_canonical_string() == "0x1::fungible_asset::Withdraw"
    }

    fn is_deposit_event_key(&self, event: &ContractEvent) -> bool {
        event.type_tag().to_canonical_string() == "0x1::fungible_asset::Deposit"
    }


    fn print_mismatches(
        &self,
        cur_version: u64,
        res_1: &Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>,
        res_2: &Result<((WriteSet, Vec<ContractEvent>), TransactionStatus, u64), VMStatus>,
        package_name: Option<String>,
    ) {
        let gas_diff = |gas_1: u64, gas_2: u64, x: u64| -> (f64, bool, bool) {
            let gas2_ge_gas1: bool = gas_2 > gas_1;
            let gas1_ge_gas2: bool = gas_1 > gas_2;
            let mut denominator = gas_1;
            let mut difference = gas_2 as i64 - gas_1 as i64;
            if !gas2_ge_gas1 {
                difference = gas_1 as i64 - gas_2 as i64;
                denominator = gas_2;
            }
            let percentage_difference = difference as f64 / denominator as f64 * 100.0;
            (
                percentage_difference,
                gas2_ge_gas1,
                gas1_ge_gas2,
            )
        };
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
            (Ok((res_1, txn_status_1, gas_used_1)), Ok((res_2, txn_status_2, gas_used_2))) => {
                // compare txn status
                if txn_status_1 != txn_status_2 {
                    self.output_result_str(format!("txn status is different at version: {}, status from V1:{:?}, gas used:{}, status from V2:{:?}, gas used:{}", cur_version, txn_status_1, gas_used_1, txn_status_2, gas_used_2));
                    return;
                }
                let (diff, gas2_gt_gas1, gas1_gt_gas_2) =
                gas_diff(*gas_used_1, *gas_used_2, GAS_DIFF_PERCENTAGE);
                let greater_version = if gas1_gt_gas_2 { "v1" } else { "v2" };
                let gas_equal = !(gas2_gt_gas1 || gas1_gt_gas_2);
                if !gas_equal {
                    self.output_result_str(format!(
                        "gas v1:{}, gas v2:{}, gas diff: {}'s gas usage is {} percent more than the other at version: {}, v1 status:{:?}, v2 status:{:?} for package:{}",
                        gas_used_1, gas_used_2, greater_version, diff, cur_version, txn_status_1, txn_status_2, package_name.unwrap_or("unknown package".to_string())
                    ));
                } else {
                    self.output_result_str(format!(
                        "v1 and v2 cosumes same amount of gas {} at version: {}, v1 status:{:?}, v2 status:{:?} for package:{}",
                        gas_used_1, cur_version, txn_status_1, txn_status_2, package_name.unwrap_or("unknown package".to_string())
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
                    if write_set_1.1 != write_set_2.1
                        && write_set_1.0 == write_set_2.0
                        && (gas_equal || !self.is_fee_related_stake_key(write_set_1.0))
                    {
                        if !gas_equal && self.filter_stake_key_resource_group(write_set_1.0, write_set_1.1, write_set_2.1) {
                            continue;
                        }
                        write_set_error = true;
                        self.output_result_str(format!(
                            "write set value is different at version: {}, index: {} for key:{:?}, key eq:{}",
                            cur_version, idx, write_set_1.0, write_set_1.0 == write_set_2.0
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
                // compare events
                let mut event_error = false;
                if res_1.1.len() != res_2.1.len() {
                    event_error = true;
                }
                for idx in 0..cmp::min(res_1.1.len(), res_2.1.len()) {
                    let event_1 = &res_1.1[idx];
                    let event_2 = &res_2.1[idx];
                    if event_1 != event_2 && (gas_equal || !self.is_fee_statement_event_key(event_1)) {
                        if self.is_withdraw_event_key(event_1) && !write_set_error {
                            println!("Withdraw event v1:{:?}", bcs::from_bytes::<WithdrawFAEvent>(&event_1.event_data()));
                            println!("Withdraw event v2:{:?}", bcs::from_bytes::<WithdrawFAEvent>(&event_2.event_data()));
                            continue;
                        }
                        if self.is_deposit_event_key(event_1) && !write_set_error {
                            println!("Deposit event v1:{:?}", bcs::from_bytes::<DepositFAEvent>(&event_1.event_data()));
                            println!("Deposit event v2:{:?}", bcs::from_bytes::<DepositFAEvent>(&event_2.event_data()));
                            continue;
                        }
                        if self.is_fee_statement_event_key(event_1) && !write_set_error {
                            println!("FeeStatement v1:{:?}", bcs::from_bytes::<FeeStatement>(&event_1.event_data()));
                            println!("FeeStatement v2:{:?}", bcs::from_bytes::<FeeStatement>(&event_2.event_data()));
                            continue;
                        }
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
            },
        }
    }
}
