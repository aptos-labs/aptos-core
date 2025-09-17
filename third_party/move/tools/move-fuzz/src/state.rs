// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::sequence::{ResourceTag, SeedInput},
    prep::{canvas::ScriptSignature, ident::DatatypeIdent},
};
use anyhow::{anyhow, Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier,
    language_storage::TypeTag as VmTypeTag, value::MoveValue,
};
use move_coverage::coverage_map::{ExecCoverageMap, ModuleCoverageMap};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub const AUTO_STATE_VERSION: u32 = 6;
pub const AUTO_STATE_FILENAME: &str = "auto_state.json";
pub const ENTRYPOINT_CACHE_VERSION: u32 = 2;
pub const ENTRYPOINT_CACHE_FILENAME: &str = "entrypoints_cache.json";
pub const PACKAGE_BUILD_CACHE_DIR: &str = "package-cache";
pub const PACKAGE_BUILD_CACHE_INFO_VERSION: u32 = 2;
pub const PACKAGE_BUILD_CACHE_INFO_FILENAME: &str = "build_cache_info.json";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PersistedDatatypeIdent {
    pub address: AccountAddress,
    pub module: String,
    pub name: String,
}

impl PersistedDatatypeIdent {
    pub fn from_ident(ident: &DatatypeIdent) -> Self {
        Self {
            address: ident.address(),
            module: ident.module_name().to_string(),
            name: ident.datatype_name().to_string(),
        }
    }

    pub fn into_ident(self) -> Result<DatatypeIdent> {
        Ok(DatatypeIdent::from_struct_tuple(
            self.address,
            move_core_types::identifier::Identifier::new(self.module)
                .context("invalid persisted datatype module name")?,
            move_core_types::identifier::Identifier::new(self.name)
                .context("invalid persisted datatype name")?,
        ))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PersistedObjectState {
    pub object_addresses: BTreeSet<AccountAddress>,
    pub dict_object: Vec<PersistedObjectBucket>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PersistedObjectBucket {
    pub ident: PersistedDatatypeIdent,
    pub addresses: BTreeSet<AccountAddress>,
}

impl PersistedObjectState {
    pub fn merge(states: impl IntoIterator<Item = Self>) -> Self {
        let mut merged = Self::default();
        let mut dict_object: BTreeMap<PersistedDatatypeIdent, BTreeSet<AccountAddress>> =
            BTreeMap::new();
        for state in states {
            merged.object_addresses.extend(state.object_addresses);
            for bucket in state.dict_object {
                dict_object
                    .entry(bucket.ident)
                    .or_default()
                    .extend(bucket.addresses);
            }
        }
        merged.dict_object = dict_object
            .into_iter()
            .map(|(ident, addresses)| PersistedObjectBucket { ident, addresses })
            .collect();
        merged
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedFunctionCoverage {
    pub function: String,
    pub counts: BTreeMap<u64, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedModuleCoverage {
    pub address: AccountAddress,
    pub module: String,
    pub functions: Vec<PersistedFunctionCoverage>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedExecCoverageMap {
    pub exec_id: String,
    pub modules: Vec<PersistedModuleCoverage>,
}

impl PersistedExecCoverageMap {
    pub fn from_exec_coverage_map(coverage: &ExecCoverageMap) -> Self {
        Self {
            exec_id: coverage.exec_id.clone(),
            modules: coverage
                .module_maps
                .iter()
                .map(
                    |((address, module_ident), module_map)| PersistedModuleCoverage {
                        address: *address,
                        module: module_ident.to_string(),
                        functions: module_map
                            .function_maps
                            .iter()
                            .map(|(function_ident, counts)| PersistedFunctionCoverage {
                                function: function_ident.to_string(),
                                counts: counts.clone(),
                            })
                            .collect(),
                    },
                )
                .collect(),
        }
    }

    pub fn into_exec_coverage_map(self) -> Result<ExecCoverageMap> {
        let mut module_maps = BTreeMap::new();
        for module in self.modules {
            let module_name =
                Identifier::new(module.module).context("invalid persisted coverage module name")?;
            let mut function_maps = BTreeMap::new();
            for function in module.functions {
                let function_name = Identifier::new(function.function)
                    .context("invalid persisted coverage function name")?;
                function_maps.insert(function_name, function.counts);
            }
            module_maps.insert((module.address, module_name.clone()), ModuleCoverageMap {
                module_addr: module.address,
                module_name,
                function_maps,
            });
        }
        Ok(ExecCoverageMap {
            exec_id: self.exec_id,
            module_maps,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistedMoveValue {
    Bool(bool),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    U128(u128),
    I128(i128),
    U256(move_core_types::int256::U256),
    I256(move_core_types::int256::I256),
    Address(AccountAddress),
    Signer(AccountAddress),
    Vector(Vec<PersistedMoveValue>),
}

impl PersistedMoveValue {
    pub fn try_from_move_value(value: &MoveValue) -> Result<Self> {
        Ok(match value {
            MoveValue::Bool(v) => Self::Bool(*v),
            MoveValue::U8(v) => Self::U8(*v),
            MoveValue::I8(v) => Self::I8(*v),
            MoveValue::U16(v) => Self::U16(*v),
            MoveValue::I16(v) => Self::I16(*v),
            MoveValue::U32(v) => Self::U32(*v),
            MoveValue::I32(v) => Self::I32(*v),
            MoveValue::U64(v) => Self::U64(*v),
            MoveValue::I64(v) => Self::I64(*v),
            MoveValue::U128(v) => Self::U128(*v),
            MoveValue::I128(v) => Self::I128(*v),
            MoveValue::U256(v) => Self::U256(*v),
            MoveValue::I256(v) => Self::I256(*v),
            MoveValue::Address(v) => Self::Address(*v),
            MoveValue::Signer(v) => Self::Signer(*v),
            MoveValue::Vector(values) => Self::Vector(
                values
                    .iter()
                    .map(Self::try_from_move_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
            other => {
                return Err(anyhow!(
                    "unsupported MoveValue in persisted fuzz state: {other:?}"
                ));
            },
        })
    }

    pub fn into_move_value(self) -> Result<MoveValue> {
        Ok(match self {
            Self::Bool(v) => MoveValue::Bool(v),
            Self::U8(v) => MoveValue::U8(v),
            Self::I8(v) => MoveValue::I8(v),
            Self::U16(v) => MoveValue::U16(v),
            Self::I16(v) => MoveValue::I16(v),
            Self::U32(v) => MoveValue::U32(v),
            Self::I32(v) => MoveValue::I32(v),
            Self::U64(v) => MoveValue::U64(v),
            Self::I64(v) => MoveValue::I64(v),
            Self::U128(v) => MoveValue::U128(v),
            Self::I128(v) => MoveValue::I128(v),
            Self::U256(v) => MoveValue::U256(v),
            Self::I256(v) => MoveValue::I256(v),
            Self::Address(v) => MoveValue::Address(v),
            Self::Signer(v) => MoveValue::Signer(v),
            Self::Vector(values) => MoveValue::Vector(
                values
                    .into_iter()
                    .map(Self::into_move_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedSeedInput {
    pub sender: AccountAddress,
    pub ty_args: Vec<VmTypeTag>,
    pub args: Vec<PersistedMoveValue>,
}

impl PersistedSeedInput {
    pub fn try_from_seed(seed: &SeedInput) -> Result<Self> {
        Ok(Self {
            sender: seed.sender,
            ty_args: seed.ty_args.clone(),
            args: seed
                .args
                .iter()
                .map(PersistedMoveValue::try_from_move_value)
                .collect::<Result<Vec<_>>>()?,
        })
    }

    pub fn into_seed(self) -> Result<SeedInput> {
        Ok(SeedInput {
            sender: self.sender,
            ty_args: self.ty_args,
            args: self
                .args
                .into_iter()
                .map(PersistedMoveValue::into_move_value)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedOneshotSeedRecord {
    pub input: PersistedSeedInput,
    pub score: u32,
    pub last_used_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedChainSeedRecord {
    pub input: Vec<PersistedSeedInput>,
    pub score: u32,
    pub last_used_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedSeedNode {
    pub id: u64,
    pub script_index: usize,
    pub seed: PersistedSeedInput,
    pub succeeded: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedDefUseGraph {
    pub num_scripts: usize,
    pub type_nodes: Vec<ResourceTag>,
    pub defs: Vec<BTreeSet<usize>>,
    pub uses: Vec<BTreeSet<usize>>,
    pub initial_types: BTreeSet<usize>,
    pub producers: BTreeMap<usize, BTreeSet<usize>>,
    pub ever_succeeded: BTreeSet<usize>,
    pub modification_count: usize,
    pub seed_modification_count: usize,
    pub seed_nodes: Vec<PersistedSeedNode>,
    pub seed_defs: Vec<BTreeSet<usize>>,
    pub seed_uses: Vec<BTreeSet<usize>>,
    pub seed_producers: BTreeMap<usize, BTreeSet<usize>>,
    pub next_seed_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedSequenceEntry {
    pub id: u64,
    pub steps: Vec<usize>,
    pub seed: Vec<PersistedSeedInput>,
    pub produced_types: BTreeSet<ResourceTag>,
    pub consumed_types: BTreeSet<ResourceTag>,
    pub step_produced_types: Vec<BTreeSet<ResourceTag>>,
    pub step_consumed_types: Vec<BTreeSet<ResourceTag>>,
    pub all_succeeded: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedSequenceDb {
    pub entries: Vec<PersistedSequenceEntry>,
    pub next_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedMissingDataSignal {
    pub script_index: usize,
    pub hits: u64,
    pub last_seen_iter: u64,
    pub unresolved_tags: BTreeSet<ResourceTag>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedOneshotFuzzer {
    pub script_index: usize,
    pub script_identity: String,
    pub replay_log: Vec<PersistedSeedInput>,
    pub seedpool: Vec<PersistedOneshotSeedRecord>,
    pub coverage: PersistedExecCoverageMap,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedChainFuzzer {
    pub steps: Vec<usize>,
    pub step_identities: Vec<String>,
    #[serde(default)]
    pub identity_seed: Vec<PersistedSeedInput>,
    pub replay_log: Vec<Vec<PersistedSeedInput>>,
    pub seedpool: Vec<PersistedChainSeedRecord>,
    pub coverage: PersistedExecCoverageMap,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedAutoState {
    pub version: u32,
    pub campaign_fingerprint: String,
    pub entrypoint_identities: Vec<String>,
    pub phase2_entered: bool,
    pub bootstrap_profile_count: usize,
    pub dug: PersistedDefUseGraph,
    pub oneshot_fuzzers: Vec<PersistedOneshotFuzzer>,
    pub sequence_db: PersistedSequenceDb,
    pub chain_fuzzers: Vec<PersistedChainFuzzer>,
    pub chain_seed_nonce: u64,
    pub object_state: PersistedObjectState,
    pub missing_data_signals: Vec<PersistedMissingDataSignal>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedEntrypoint {
    pub signature: ScriptSignature,
    #[serde(with = "hex::serde")]
    pub code: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedEntrypointCache {
    pub version: u32,
    pub fingerprint: String,
    pub entrypoints: Vec<PersistedEntrypoint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedPackageBuildCacheInfo {
    pub version: u32,
    pub package_name: String,
    pub manifest_identity: String,
    pub fingerprint: String,
    pub root_module_names: Vec<String>,
    pub root_script_names: Vec<String>,
}

impl PersistedEntrypointCache {
    pub fn new(fingerprint: String, entrypoints: Vec<PersistedEntrypoint>) -> Self {
        Self {
            version: ENTRYPOINT_CACHE_VERSION,
            fingerprint,
            entrypoints,
        }
    }
}

impl PersistedPackageBuildCacheInfo {
    pub fn new(
        package_name: String,
        manifest_identity: String,
        fingerprint: String,
        root_module_names: Vec<String>,
        root_script_names: Vec<String>,
    ) -> Self {
        Self {
            version: PACKAGE_BUILD_CACHE_INFO_VERSION,
            package_name,
            manifest_identity,
            fingerprint,
            root_module_names,
            root_script_names,
        }
    }
}

impl PersistedAutoState {
    pub fn new(
        campaign_fingerprint: String,
        entrypoint_identities: Vec<String>,
        phase2_entered: bool,
        bootstrap_profile_count: usize,
        dug: PersistedDefUseGraph,
        oneshot_fuzzers: Vec<PersistedOneshotFuzzer>,
        sequence_db: PersistedSequenceDb,
        chain_fuzzers: Vec<PersistedChainFuzzer>,
        chain_seed_nonce: u64,
        object_state: PersistedObjectState,
        missing_data_signals: Vec<PersistedMissingDataSignal>,
    ) -> Self {
        Self {
            version: AUTO_STATE_VERSION,
            campaign_fingerprint,
            entrypoint_identities,
            phase2_entered,
            bootstrap_profile_count,
            dug,
            oneshot_fuzzers,
            sequence_db,
            chain_fuzzers,
            chain_seed_nonce,
            object_state,
            missing_data_signals,
        }
    }
}

impl PersistedDefUseGraph {
    pub fn remap_script_indices(&mut self, old_to_new: &[usize]) -> Result<()> {
        if self.num_scripts != old_to_new.len()
            || self.defs.len() != self.num_scripts
            || self.uses.len() != self.num_scripts
        {
            return Err(anyhow!(
                "persisted DUG cannot be remapped because script dimensions do not match"
            ));
        }

        let mut new_defs = vec![BTreeSet::new(); self.num_scripts];
        let mut new_uses = vec![BTreeSet::new(); self.num_scripts];
        for (old_idx, refs) in self.defs.iter().cloned().enumerate() {
            new_defs[old_to_new[old_idx]] = refs;
        }
        for (old_idx, refs) in self.uses.iter().cloned().enumerate() {
            new_uses[old_to_new[old_idx]] = refs;
        }
        self.defs = new_defs;
        self.uses = new_uses;

        self.ever_succeeded = self
            .ever_succeeded
            .iter()
            .map(|old_idx| old_to_new[*old_idx])
            .collect();

        let mut new_producers = BTreeMap::new();
        for (type_idx, scripts) in &self.producers {
            new_producers.insert(
                *type_idx,
                scripts.iter().map(|old_idx| old_to_new[*old_idx]).collect(),
            );
        }
        self.producers = new_producers;

        for seed_node in &mut self.seed_nodes {
            seed_node.script_index = old_to_new[seed_node.script_index];
        }

        Ok(())
    }
}

impl PersistedSequenceDb {
    pub fn remap_script_indices(&mut self, old_to_new: &[usize]) -> Result<()> {
        for entry in &mut self.entries {
            if entry
                .steps
                .iter()
                .any(|old_idx| *old_idx >= old_to_new.len())
            {
                return Err(anyhow!(
                    "persisted sequence entry references out-of-range script index"
                ));
            }
            for step in &mut entry.steps {
                *step = old_to_new[*step];
            }
        }
        Ok(())
    }
}

pub fn load_auto_state(path: &Path) -> Result<Option<PersistedAutoState>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes =
        fs::read(path).with_context(|| format!("failed to read auto state {}", path.display()))?;
    let state = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse auto state {}", path.display()))?;
    Ok(Some(state))
}

pub fn load_entrypoint_cache(path: &Path) -> Result<Option<PersistedEntrypointCache>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read entrypoint cache {}", path.display()))?;
    let cache = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse entrypoint cache {}", path.display()))?;
    Ok(Some(cache))
}

pub fn load_package_build_cache_info(
    path: &Path,
) -> Result<Option<PersistedPackageBuildCacheInfo>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read package build cache info {}", path.display()))?;
    let info = serde_json::from_slice(&bytes).with_context(|| {
        format!(
            "failed to parse package build cache info {}",
            path.display()
        )
    })?;
    Ok(Some(info))
}

pub fn save_auto_state(path: &Path, state: &PersistedAutoState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create state directory {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(state).context("failed to serialize auto state")?;
    let tmp_path = temp_path_for(path);
    fs::write(&tmp_path, bytes)
        .with_context(|| format!("failed to write auto state {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to atomically install auto state {}", path.display()))?;
    Ok(())
}

pub fn save_entrypoint_cache(path: &Path, cache: &PersistedEntrypointCache) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create entrypoint cache directory {}",
                parent.display()
            )
        })?;
    }
    let bytes = serde_json::to_vec_pretty(cache).context("failed to serialize entrypoint cache")?;
    let tmp_path = temp_path_for(path);
    fs::write(&tmp_path, bytes).with_context(|| {
        format!(
            "failed to write entrypoint cache temporary file {}",
            tmp_path.display()
        )
    })?;
    fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "failed to atomically install entrypoint cache {}",
            path.display()
        )
    })?;
    Ok(())
}

pub fn save_package_build_cache_info(
    path: &Path,
    info: &PersistedPackageBuildCacheInfo,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create package build cache directory {}",
                parent.display()
            )
        })?;
    }
    let bytes =
        serde_json::to_vec_pretty(info).context("failed to serialize package build cache info")?;
    let tmp_path = temp_path_for(path);
    fs::write(&tmp_path, bytes).with_context(|| {
        format!(
            "failed to write package build cache temporary file {}",
            tmp_path.display()
        )
    })?;
    fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "failed to atomically install package build cache info {}",
            path.display()
        )
    })?;
    Ok(())
}

fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(AUTO_STATE_FILENAME);
    path.with_file_name(format!("{file_name}.tmp"))
}

#[cfg(test)]
mod tests {
    use super::{
        load_auto_state, load_entrypoint_cache, load_package_build_cache_info, save_auto_state,
        save_entrypoint_cache, save_package_build_cache_info, PersistedAutoState,
        PersistedChainFuzzer, PersistedChainSeedRecord, PersistedDefUseGraph, PersistedEntrypoint,
        PersistedEntrypointCache, PersistedExecCoverageMap, PersistedMissingDataSignal,
        PersistedMoveValue, PersistedObjectBucket, PersistedObjectState, PersistedOneshotFuzzer,
        PersistedOneshotSeedRecord, PersistedPackageBuildCacheInfo, PersistedSeedInput,
        PersistedSequenceDb, ENTRYPOINT_CACHE_VERSION, PACKAGE_BUILD_CACHE_INFO_VERSION,
    };
    use crate::{
        executor::sequence::{ResourceTag, SeedInput},
        prep::{
            canvas::{BasicInput, ScriptSignature},
            ident::{DatatypeIdent, FunctionIdent},
        },
    };
    use anyhow::Result;
    use move_core_types::{
        ability::AbilitySet, account_address::AccountAddress, identifier::Identifier,
        language_storage::StructTag, value::MoveValue,
    };
    use move_coverage::coverage_map::ExecCoverageMap;
    use std::collections::{BTreeMap, BTreeSet};
    use tempfile::TempDir;

    fn sample_coverage(exec_id: &str) -> ExecCoverageMap {
        let mut coverage = ExecCoverageMap::new(exec_id.to_string());
        coverage.insert(
            AccountAddress::from_hex_literal("0x1").unwrap(),
            Identifier::new("vault").unwrap(),
            Identifier::new("deposit").unwrap(),
            7,
        );
        coverage
    }

    #[test]
    fn test_persisted_move_value_roundtrip_nested_vector() -> Result<()> {
        let value = MoveValue::Vector(vec![
            MoveValue::U8(7),
            MoveValue::Vector(vec![
                MoveValue::Bool(true),
                MoveValue::Address(AccountAddress::ONE),
            ]),
        ]);
        let persisted = PersistedMoveValue::try_from_move_value(&value)?;
        assert_eq!(persisted.clone().into_move_value()?, value);
        Ok(())
    }

    #[test]
    fn test_persisted_seed_input_roundtrip() -> Result<()> {
        let seed = SeedInput {
            sender: AccountAddress::from_hex_literal("0x44")?,
            ty_args: vec![move_core_types::language_storage::TypeTag::Bool],
            args: vec![MoveValue::U64(99), MoveValue::Signer(AccountAddress::ONE)],
        };
        let persisted = PersistedSeedInput::try_from_seed(&seed)?;
        assert_eq!(persisted.into_seed()?, seed);
        Ok(())
    }

    #[test]
    fn test_auto_state_file_roundtrip() -> Result<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("auto_state.json");
        let object_ident = DatatypeIdent::from_struct_tuple(
            AccountAddress::from_hex_literal("0xcafe")?,
            Identifier::new("vault")?,
            Identifier::new("Position")?,
        );
        let state = PersistedAutoState::new(
            "fingerprint".to_string(),
            vec!["script-a".to_string()],
            false,
            3,
            PersistedDefUseGraph {
                num_scripts: 1,
                type_nodes: vec![],
                defs: vec![Default::default()],
                uses: vec![Default::default()],
                initial_types: Default::default(),
                producers: Default::default(),
                ever_succeeded: Default::default(),
                modification_count: 0,
                seed_modification_count: 0,
                seed_nodes: vec![],
                seed_defs: vec![],
                seed_uses: vec![],
                seed_producers: Default::default(),
                next_seed_id: 0,
            },
            vec![PersistedOneshotFuzzer {
                script_index: 0,
                script_identity: "script-a".to_string(),
                replay_log: vec![PersistedSeedInput::try_from_seed(&SeedInput {
                    sender: AccountAddress::from_hex_literal("0x44")?,
                    ty_args: vec![],
                    args: vec![MoveValue::U64(3)],
                })?],
                seedpool: vec![PersistedOneshotSeedRecord {
                    input: PersistedSeedInput::try_from_seed(&SeedInput {
                        sender: AccountAddress::from_hex_literal("0x44")?,
                        ty_args: vec![],
                        args: vec![MoveValue::U64(7)],
                    })?,
                    score: 9,
                    last_used_at: 11,
                }],
                coverage: PersistedExecCoverageMap::from_exec_coverage_map(&sample_coverage(
                    "oneshot",
                )),
            }],
            PersistedSequenceDb {
                entries: vec![],
                next_id: 0,
            },
            vec![PersistedChainFuzzer {
                steps: vec![0],
                step_identities: vec!["script-a".to_string()],
                identity_seed: vec![PersistedSeedInput::try_from_seed(&SeedInput {
                    sender: AccountAddress::from_hex_literal("0x44")?,
                    ty_args: vec![],
                    args: vec![MoveValue::Bool(false)],
                })?],
                replay_log: vec![vec![PersistedSeedInput::try_from_seed(&SeedInput {
                    sender: AccountAddress::from_hex_literal("0x44")?,
                    ty_args: vec![],
                    args: vec![MoveValue::Bool(false)],
                })?]],
                seedpool: vec![PersistedChainSeedRecord {
                    input: vec![PersistedSeedInput::try_from_seed(&SeedInput {
                        sender: AccountAddress::from_hex_literal("0x44")?,
                        ty_args: vec![],
                        args: vec![MoveValue::Bool(true)],
                    })?],
                    score: 5,
                    last_used_at: 17,
                }],
                coverage: PersistedExecCoverageMap::from_exec_coverage_map(&sample_coverage(
                    "chain",
                )),
            }],
            7,
            PersistedObjectState {
                object_addresses: BTreeSet::from([AccountAddress::from_hex_literal("0x44")?]),
                dict_object: vec![PersistedObjectBucket {
                    ident: super::PersistedDatatypeIdent::from_ident(&object_ident),
                    addresses: BTreeSet::from([AccountAddress::from_hex_literal("0x55")?]),
                }],
            },
            vec![PersistedMissingDataSignal {
                script_index: 0,
                hits: 4,
                last_seen_iter: 12,
                unresolved_tags: BTreeSet::from([ResourceTag {
                    account: AccountAddress::from_hex_literal("0x44")?,
                    struct_tag: StructTag {
                        address: AccountAddress::ONE,
                        module: Identifier::new("m")?,
                        name: Identifier::new("State")?,
                        type_args: vec![],
                    },
                }]),
            }],
        );

        save_auto_state(&path, &state)?;
        let loaded = load_auto_state(&path)?.expect("state exists");
        assert_eq!(loaded.version, state.version);
        assert_eq!(loaded.campaign_fingerprint, "fingerprint");
        assert_eq!(loaded.entrypoint_identities, vec!["script-a".to_string()]);
        assert_eq!(loaded.bootstrap_profile_count, 3);
        assert_eq!(loaded.chain_seed_nonce, 7);
        assert_eq!(loaded.oneshot_fuzzers.len(), 1);
        assert_eq!(loaded.chain_fuzzers.len(), 1);
        assert_eq!(loaded.oneshot_fuzzers[0].script_identity, "script-a");
        assert_eq!(loaded.oneshot_fuzzers[0].replay_log.len(), 1);
        assert_eq!(loaded.oneshot_fuzzers[0].seedpool[0].score, 9);
        assert_eq!(loaded.chain_fuzzers[0].step_identities, vec![
            "script-a".to_string()
        ]);
        assert_eq!(loaded.chain_fuzzers[0].replay_log.len(), 1);
        assert_eq!(loaded.chain_fuzzers[0].seedpool[0].score, 5);
        assert_eq!(
            loaded.oneshot_fuzzers[0]
                .coverage
                .clone()
                .into_exec_coverage_map()?
                .module_maps
                .len(),
            1
        );
        assert_eq!(loaded.object_state.object_addresses.len(), 1);
        assert_eq!(loaded.object_state.dict_object.len(), 1);
        assert_eq!(
            loaded.object_state.dict_object[0].ident,
            super::PersistedDatatypeIdent::from_ident(&object_ident)
        );
        assert_eq!(loaded.missing_data_signals.len(), 1);
        assert_eq!(loaded.missing_data_signals[0].hits, 4);
        Ok(())
    }

    #[test]
    fn test_persisted_object_state_merge_unions_buckets() -> Result<()> {
        let ident_a = super::PersistedDatatypeIdent {
            address: AccountAddress::ONE,
            module: "m".to_string(),
            name: "A".to_string(),
        };
        let ident_b = super::PersistedDatatypeIdent {
            address: AccountAddress::ONE,
            module: "m".to_string(),
            name: "B".to_string(),
        };
        let merged = PersistedObjectState::merge([
            PersistedObjectState {
                object_addresses: BTreeSet::from([AccountAddress::from_hex_literal("0x44")?]),
                dict_object: vec![PersistedObjectBucket {
                    ident: ident_a.clone(),
                    addresses: BTreeSet::from([AccountAddress::from_hex_literal("0x44")?]),
                }],
            },
            PersistedObjectState {
                object_addresses: BTreeSet::from([AccountAddress::from_hex_literal("0x55")?]),
                dict_object: vec![
                    PersistedObjectBucket {
                        ident: ident_a.clone(),
                        addresses: BTreeSet::from([AccountAddress::from_hex_literal("0x66")?]),
                    },
                    PersistedObjectBucket {
                        ident: ident_b.clone(),
                        addresses: BTreeSet::from([AccountAddress::from_hex_literal("0x55")?]),
                    },
                ],
            },
        ]);
        assert_eq!(
            merged.object_addresses,
            BTreeSet::from([
                AccountAddress::from_hex_literal("0x44")?,
                AccountAddress::from_hex_literal("0x55")?,
            ])
        );
        assert_eq!(merged.dict_object.len(), 2);
        assert_eq!(merged.dict_object[0].ident, ident_a);
        assert_eq!(
            merged.dict_object[0].addresses,
            BTreeSet::from([
                AccountAddress::from_hex_literal("0x44")?,
                AccountAddress::from_hex_literal("0x66")?,
            ])
        );
        assert_eq!(merged.dict_object[1].ident, ident_b);
        assert_eq!(
            merged.dict_object[1].addresses,
            BTreeSet::from([AccountAddress::from_hex_literal("0x55")?])
        );
        Ok(())
    }

    #[test]
    fn test_entrypoint_cache_roundtrip() -> Result<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("entrypoints_cache.json");
        let cache = PersistedEntrypointCache::new("cache-fingerprint".to_string(), vec![
            PersistedEntrypoint {
                signature: ScriptSignature {
                    name: "script_0".to_string(),
                    ident: FunctionIdent::from_function_tuple(
                        AccountAddress::ONE,
                        Identifier::new("m")?,
                        Identifier::new("f")?,
                    ),
                    generics: vec![AbilitySet::PRIMITIVES],
                    parameters: vec![
                        BasicInput::Address,
                        BasicInput::Vector(Box::new(BasicInput::ObjectKnown {
                            ident: DatatypeIdent::from_struct_tuple(
                                AccountAddress::ONE,
                                Identifier::new("m")?,
                                Identifier::new("Obj")?,
                            ),
                            type_args: vec![],
                        })),
                    ],
                },
                code: vec![1, 2, 3, 4],
            },
        ]);

        save_entrypoint_cache(&path, &cache)?;
        let loaded = load_entrypoint_cache(&path)?.expect("cache exists");
        assert_eq!(loaded.version, ENTRYPOINT_CACHE_VERSION);
        assert_eq!(loaded.fingerprint, "cache-fingerprint");
        assert_eq!(loaded.entrypoints, cache.entrypoints);
        Ok(())
    }

    #[test]
    fn test_package_build_cache_info_roundtrip() -> Result<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("build_cache_info.json");
        let info = PersistedPackageBuildCacheInfo::new(
            "Example".to_string(),
            "/tmp/example/Move.toml".to_string(),
            "fingerprint-123".to_string(),
            vec!["Vault".to_string()],
            vec!["setup".to_string()],
        );

        save_package_build_cache_info(&path, &info)?;
        let loaded = load_package_build_cache_info(&path)?.expect("cache info exists");
        assert_eq!(loaded.version, PACKAGE_BUILD_CACHE_INFO_VERSION);
        assert_eq!(loaded.package_name, "Example");
        assert_eq!(loaded.manifest_identity, "/tmp/example/Move.toml");
        assert_eq!(loaded.fingerprint, "fingerprint-123");
        assert_eq!(loaded.root_module_names, vec!["Vault".to_string()]);
        assert_eq!(loaded.root_script_names, vec!["setup".to_string()]);
        Ok(())
    }

    #[test]
    fn test_persisted_state_remaps_script_indices() -> Result<()> {
        let mut dug = PersistedDefUseGraph {
            num_scripts: 2,
            type_nodes: vec![],
            defs: vec![BTreeSet::from([0]), BTreeSet::from([1])],
            uses: vec![BTreeSet::new(), BTreeSet::new()],
            initial_types: BTreeSet::new(),
            producers: BTreeMap::from([
                (0usize, BTreeSet::from([0usize])),
                (1, BTreeSet::from([1])),
            ]),
            ever_succeeded: BTreeSet::from([1usize]),
            modification_count: 0,
            seed_modification_count: 0,
            seed_nodes: vec![
                super::PersistedSeedNode {
                    id: 0,
                    script_index: 0,
                    seed: PersistedSeedInput::try_from_seed(&SeedInput::new(
                        AccountAddress::ONE,
                        vec![],
                        vec![],
                    ))?,
                    succeeded: true,
                },
                super::PersistedSeedNode {
                    id: 1,
                    script_index: 1,
                    seed: PersistedSeedInput::try_from_seed(&SeedInput::new(
                        AccountAddress::ONE,
                        vec![],
                        vec![],
                    ))?,
                    succeeded: true,
                },
            ],
            seed_defs: vec![BTreeSet::new(), BTreeSet::new()],
            seed_uses: vec![BTreeSet::new(), BTreeSet::new()],
            seed_producers: BTreeMap::new(),
            next_seed_id: 2,
        };
        dug.remap_script_indices(&[1, 0])?;
        assert_eq!(dug.defs[1], BTreeSet::from([0]));
        assert_eq!(dug.defs[0], BTreeSet::from([1]));
        assert!(dug.ever_succeeded.contains(&0));
        assert_eq!(dug.seed_nodes[0].script_index, 1);
        assert_eq!(dug.seed_nodes[1].script_index, 0);

        let mut seq_db = PersistedSequenceDb {
            entries: vec![super::PersistedSequenceEntry {
                id: 0,
                steps: vec![0, 1, 0],
                seed: vec![],
                produced_types: BTreeSet::new(),
                consumed_types: BTreeSet::new(),
                step_produced_types: vec![],
                step_consumed_types: vec![],
                all_succeeded: true,
            }],
            next_id: 1,
        };
        seq_db.remap_script_indices(&[1, 0])?;
        assert_eq!(seq_db.entries[0].steps, vec![1, 0, 1]);
        Ok(())
    }
}
