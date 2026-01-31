// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    check_aptos_packages_availability, compile_aptos_packages, compile_package,
    data_state_view::DataStateView, generate_compiled_blob, is_aptos_package, CompilationCache,
    DataManager, IndexReader, PackageInfo, TxnIndex, APTOS_COMMONS,
};
use anyhow::Result;
use aptos_framework::APTOS_PACKAGES;
use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_replay_benchmark::diff::{Diff, TransactionDiffBuilder};
use aptos_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
use aptos_types::{
    access_path::Path,
    on_chain_config::{FeatureFlag, Features, OnChainConfig},
    state_store::state_key::{inner::StateKeyInner, StateKey},
    transaction::{Transaction, TransactionOutput, Version},
    vm_status::VMStatus,
    write_set::{WriteOp, TOTAL_SUPPLY_STATE_KEY},
};
use aptos_validator_interface::AptosValidatorInterface;
use clap::ValueEnum;
use move_binary_format::file_format_common::VERSION_DEFAULT;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
};

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

pub(crate) fn add_aptos_packages_to_state_store(
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

#[derive(ValueEnum, Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd)]
pub enum ExecutionMode {
    #[default]
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

pub struct Execution {
    input_path: PathBuf,
    pub execution_mode: ExecutionMode,
    pub bytecode_version: u32,
    pub enable_features: Vec<FeatureFlag>,
    pub disable_features: Vec<FeatureFlag>,
}

impl Execution {
    pub fn output_result_str(&self, msg: String) {
        eprintln!("{}", msg);
    }

    pub fn new(
        input_path: PathBuf,
        execution_mode: ExecutionMode,
        enable_features: Vec<FeatureFlag>,
        disable_features: Vec<FeatureFlag>,
    ) -> Self {
        Self {
            input_path,
            execution_mode,
            bytecode_version: VERSION_DEFAULT,
            enable_features,
            disable_features,
        }
    }

    pub async fn execute_txns(
        &self,
        begin: Version,
        num_txns_to_execute: u64,
        base_experiments: Vec<String>,
        compared_experiments: Vec<String>,
    ) -> Result<()> {
        let aptos_commons_path = self.input_path.join(APTOS_COMMONS);
        if !check_aptos_packages_availability(aptos_commons_path.clone()) {
            return Err(anyhow::Error::msg("aptos packages are missing"));
        }
        let mut compiled_cache = CompilationCache::default();
        if self.execution_mode.is_v1_or_compare() {
            compile_aptos_packages(
                &aptos_commons_path,
                &mut compiled_cache.base_compiled_package_cache,
                &base_experiments,
                "base",
            )?;
        }
        if self.execution_mode.is_v2_or_compare() {
            compile_aptos_packages(
                &aptos_commons_path,
                &mut compiled_cache.compared_compiled_package_cache,
                &compared_experiments,
                "compared",
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
            let res = self.execute_one_txn(
                cur_version,
                &data_manager,
                &mut compiled_cache,
                &base_experiments,
                &compared_experiments,
            );
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

    fn compile_code(
        &self,
        txn_idx: &TxnIndex,
        compiled_cache: &mut CompilationCache,
        base_experiments: &[String],
        compared_experiments: &[String],
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
                .base_compiled_package_cache
                .contains_key(&package_info)
        {
            if compiled_cache.failed_packages_base.contains(&package_info) {
                v1_failed = true;
            } else {
                let compiled_res_v1 =
                    compile_package(package_dir.clone(), &package_info, base_experiments, "base");
                if let Ok(compiled_res) = compiled_res_v1 {
                    generate_compiled_blob(
                        &package_info,
                        &compiled_res,
                        &mut compiled_cache.base_compiled_package_cache,
                    );
                } else {
                    v1_failed = true;
                    compiled_cache
                        .failed_packages_base
                        .insert(package_info.clone());
                }
            }
        }
        if self.execution_mode.is_v2_or_compare()
            && !compiled_cache
                .compared_compiled_package_cache
                .contains_key(&package_info)
        {
            if compiled_cache
                .failed_packages_compared
                .contains(&package_info)
            {
                v2_failed = true;
            } else {
                let compiled_res_v2 =
                    compile_package(package_dir, &package_info, compared_experiments, "compared");
                if let Ok(compiled_res) = compiled_res_v2 {
                    generate_compiled_blob(
                        &package_info,
                        &compiled_res,
                        &mut compiled_cache.compared_compiled_package_cache,
                    );
                } else {
                    v2_failed = true;
                    compiled_cache
                        .failed_packages_compared
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
        base_experiments: &[String],
        compared_experiments: &[String],
    ) -> Result<()> {
        if let Some(txn_idx) = data_manager.get_txn_index(cur_version) {
            // compile the code if the source code is available
            if txn_idx.package_info.is_compilable()
                && !is_aptos_package(&txn_idx.package_info.package_name)
            {
                let compiled_result = self.compile_code(
                    &txn_idx,
                    compiled_cache,
                    base_experiments,
                    compared_experiments,
                );
                if let Err(err) = compiled_result {
                    self.output_result_str(format!("{} at version:{}", err, cur_version));
                    return Err(err);
                }
            }
            // read the state data
            let state = data_manager.get_state(cur_version);
            self.execute_and_compare(
                cur_version,
                state,
                &txn_idx,
                &compiled_cache.base_compiled_package_cache,
                &compiled_cache.compared_compiled_package_cache,
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
        debugger: Option<Arc<dyn AptosValidatorInterface + Send>>,
    ) {
        let mut package_cache_main = compiled_package_cache;
        let package_cache_other = compiled_package_cache_v2;
        if self.execution_mode.is_v2() {
            package_cache_main = compiled_package_cache_v2;
        }
        let res_main = self.execute_code(
            cur_version,
            state.clone(),
            &txn_idx.package_info,
            &txn_idx.txn,
            package_cache_main,
            debugger.clone(),
        );
        if self.execution_mode.is_compare() {
            let res_other = self.execute_code(
                cur_version,
                state,
                &txn_idx.package_info,
                &txn_idx.txn,
                package_cache_other,
                debugger.clone(),
            );
            self.print_mismatches(
                cur_version,
                &res_main,
                &res_other,
                Some(txn_idx.package_info.package_name.clone()),
            );
        } else {
            match res_main {
                Ok(res) => {
                    let write_set = res.write_set();
                    let events = res.events();
                    let txn_status = res.status();
                    let gas = res.gas_used();
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

    fn enable_features(&self, features: &mut Features, enable_features: &Vec<FeatureFlag>) {
        for feature in enable_features {
            features.enable(*feature);
        }
    }

    fn disable_features(&self, features: &mut Features, disable_features: &Vec<FeatureFlag>) {
        for feature in disable_features {
            features.disable(*feature);
        }
    }

    fn execute_code(
        &self,
        version: Version,
        state: InMemoryStateStore,
        package_info: &PackageInfo,
        txn: &Transaction,
        compiled_package_cache: &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
        debugger_opt: Option<Arc<dyn AptosValidatorInterface + Send>>,
    ) -> Result<TransactionOutput, VMStatus> {
        // Always add Aptos (0x1) packages.
        add_aptos_packages_to_state_store(&state, compiled_package_cache);

        // Add other modules.
        if package_info.is_compilable() {
            add_packages_to_state_store(&state, package_info, compiled_package_cache);
        }

        // Update features if needed to the correct binary format used by V2 compiler.
        let mut features = Features::fetch_config(&state).unwrap_or_default();
        self.enable_features(&mut features, &self.enable_features);
        self.disable_features(&mut features, &self.disable_features);

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
                .map(|mut res| res.pop().unwrap())
        } else {
            executor
                .execute_transaction_block_with_state_view(txns, &state, false)
                .map(|mut res| res.pop().unwrap())
        }
    }

    /// Filter out stake key related to fee so that related write set diff will be ignored
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

    /// Filter out stake key related to FA or supply so that related write set diff will be ignored
    fn filter_stake_key_resource_group(
        &self,
        key: &StateKey,
        value_1: &WriteOp,
        value_2: &WriteOp,
    ) -> bool {
        if let StateKeyInner::AccessPath(p) = key.inner() {
            let path = p.get_path();
            if let Path::ResourceGroup(_) = path {
                let state_value_1_opt = value_1.as_state_value_opt();
                let state_value_2_opt = value_2.as_state_value_opt();
                if let (Some(start_value_1), Some(start_value_2)) =
                    (state_value_1_opt, state_value_2_opt)
                {
                    let byte_map_1: BTreeMap<StructTag, Vec<u8>> =
                        bcs::from_bytes(start_value_1.bytes()).unwrap_or_default();
                    let byte_map_2: BTreeMap<StructTag, Vec<u8>> =
                        bcs::from_bytes(start_value_2.bytes()).unwrap_or_default();
                    if byte_map_1.len() != byte_map_2.len() {
                        return false;
                    }
                    for tag_1 in byte_map_1.keys() {
                        if !byte_map_2.contains_key(tag_1) {
                            return false;
                        }
                        if tag_1.name.as_str() != "ConcurrentSupply"
                            && tag_1.name.as_str() != "FungibleStore"
                        {
                            if byte_map_1.get(tag_1).unwrap() != byte_map_2.get(tag_1).unwrap() {
                                return false;
                            }
                        }
                    }
                    return true;
                } else {
                    return false;
                }
            }
        }
        false
    }

    fn print_mismatches(
        &self,
        cur_version: u64,
        res_1: &Result<TransactionOutput, VMStatus>,
        res_2: &Result<TransactionOutput, VMStatus>,
        package_name: Option<String>,
    ) {
        let gas_diff = |gas_1: u64, gas_2: u64| -> (f64, bool, bool) {
            assert!(gas_1 > 0);
            assert!(gas_2 > 0);
            let gas2_ge_gas1: bool = gas_2 > gas_1;
            let gas1_ge_gas2: bool = gas_1 > gas_2;
            let mut denominator = gas_1;
            let mut difference = gas_2 as i64 - gas_1 as i64;
            if !gas2_ge_gas1 {
                difference = gas_1 as i64 - gas_2 as i64;
                denominator = gas_2;
            }
            let percentage_difference = difference as f64 / denominator as f64 * 100.0;
            (percentage_difference, gas2_ge_gas1, gas1_ge_gas2)
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
            (Ok(res_1), Ok(res_2)) => {
                let transaction_diff_builder = TransactionDiffBuilder::new(true, true);
                let transaction_diff =
                    transaction_diff_builder.build_from_outputs(res_1.clone(), res_2.clone(), None);
                for diff in transaction_diff.diffs {
                    match diff {
                        Diff::GasUsed { left, right } => {
                            let (diff, gas2_gt_gas1, gas1_gt_gas_2) = gas_diff(left, right);
                            let greater_version = if gas1_gt_gas_2 { "v1" } else { "v2" };
                            let gas_equal = !(gas2_gt_gas1 || gas1_gt_gas_2);
                            if !gas_equal {
                                self.output_result_str(format!(
                                    "gas v1:{}, gas v2:{}, gas diff: {}'s gas usage is {} percent more than the other at version: {}, v1 status:{:?}, v2 status:{:?} for package:{}",
                                    left, right, greater_version, diff, cur_version, res_1.status(), res_2.status(), package_name.clone().unwrap_or("unknown package".to_string())
                                ));
                            }
                        },
                        Diff::ExecutionStatus { left, right } => {
                            self.output_result_str(format!("txn status is different at version: {}, execution status from V1: {:?}, execution status from V2: {:?}", cur_version, left, right));
                        },
                        Diff::Event { left, right } => {
                            self.output_result_str(format!("event is different at version: {}, event from V1: {:?}, event from V2: {:?}", cur_version, left, right));
                        },
                        Diff::WriteSet {
                            state_key,
                            left,
                            right,
                        } => {
                            let mut fa_related = false;
                            if let (Some(left), Some(right)) = (&left, &right) {
                                if self.is_fee_related_stake_key(&state_key)
                                    || self.filter_stake_key_resource_group(&state_key, left, right)
                                {
                                    fa_related = true;
                                }
                            }
                            if !fa_related {
                                self.output_result_str(format!("write set value is different at version: {}, for key:{:?}, write set from V1: {:?}, write set from V2: {:?}", cur_version, state_key, left, right));
                            }
                        },
                    }
                }
            },
        }
    }
}
