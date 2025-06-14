// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_framework::{
    natives::code::PackageMetadata, unzip_metadata_str, BuiltPackage, APTOS_PACKAGES,
};
use aptos_types::{
    block_executor::transaction_slice_metadata::TransactionSliceMetadata,
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        Transaction, Version,
    },
};
use move_core_types::account_address::AccountAddress;
use move_package::{
    compilation::compiled_package::CompiledPackage,
    source_package::{
        manifest_parser::{parse_move_manifest_string, parse_source_manifest},
        parsed_manifest::Dependency,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::File,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

/// A workload to benchmark. Contains signature verified transactions, and metadata specifying the
/// start and end versions of these transactions.
pub(crate) struct Workload {
    /// Stores a non-empty block of  signature verified transactions ready for execution.
    pub(crate) txn_provider: DefaultTxnProvider<SignatureVerifiedTransaction>,
    /// Stores metadata for the version range of a block, corresponding to [begin, end) versions.
    /// It is always set to [TransactionSliceMetadata::Chunk].
    pub(crate) transaction_slice_metadata: TransactionSliceMetadata,
}

/// On-disk representation of a workload, saved to the local filesystem.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct TransactionBlock {
    /// The version of the first transaction in the block.
    pub(crate) begin_version: Version,
    /// Non-empty list of transactions in a block.
    pub(crate) transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub(crate) struct PackageInfo {
    pub(crate) address: AccountAddress,
    pub(crate) package_name: String,
    pub(crate) upgrade_number: Option<u64>,
}

impl fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut name = format!("{}.{}", self.package_name, self.address);
        if self.upgrade_number.is_some() {
            name = format!("{}.{}", name, self.upgrade_number.unwrap());
        }
        write!(f, "{}", name)?;
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct CompilationCache {
    pub(crate) compiled_package_map: HashMap<PackageInfo, CompiledPackage>,
    pub(crate) failed_packages: HashSet<PackageInfo>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct BlockIndex {
    pub(crate) transaction_block: TransactionBlock,
    pub(crate) package_info: HashMap<Version, PackageInfo>,
    pub(crate) _parallel_execution: bool, // TODO: use this field to determine whether to run in parallel
}

pub const APTOS_PACKAGES_DIR_NAMES: [&str; 6] = [
    "aptos-framework",
    "move-stdlib",
    "aptos-stdlib",
    "aptos-token",
    "aptos-token-objects",
    "aptos-experimental",
];

pub const APTOS_COMMONS: &str = "aptos-commons";

pub(crate) fn is_aptos_package(package_name: &str) -> bool {
    APTOS_PACKAGES.contains(&package_name)
}

fn get_aptos_dir(package_name: &str) -> Option<&str> {
    if is_aptos_package(package_name) {
        for i in 0..APTOS_PACKAGES.len() {
            if APTOS_PACKAGES[i] == package_name {
                return Some(APTOS_PACKAGES_DIR_NAMES[i]);
            }
        }
    }
    None
}

async fn download_aptos_packages(path: &Path) -> anyhow::Result<()> {
    let git_url = "https://github.com/aptos-labs/aptos-core";
    let tmp_dir = TempDir::new()?;
    Command::new("git")
        .args(["clone", git_url, tmp_dir.path().to_str().unwrap()])
        .output()
        .map_err(|_| anyhow::anyhow!("Failed to clone Git repository"))?;
    let source_framework_path = PathBuf::from(tmp_dir.path()).join("aptos-move/framework");
    for package_name in APTOS_PACKAGES {
        let source_framework_path =
            source_framework_path.join(get_aptos_dir(package_name).unwrap());
        let target_framework_path = PathBuf::from(path).join(get_aptos_dir(package_name).unwrap());
        Command::new("cp")
            .arg("-r")
            .arg(source_framework_path)
            .arg(target_framework_path)
            .output()
            .map_err(|_| anyhow::anyhow!("Failed to copy"))?;
    }

    Ok(())
}

pub async fn prepare_aptos_packages(path: PathBuf) {
    let mut success = true;
    if path.exists() {
        success = std::fs::remove_dir_all(path.clone()).is_ok();
    }
    if success {
        std::fs::create_dir_all(path.clone()).unwrap();
        download_aptos_packages(&path).await.unwrap();
    }
}

pub(crate) fn dump_and_check_src(
    version: Version,
    address: AccountAddress,
    package_name: String,
    map: HashMap<(AccountAddress, String), PackageMetadata>,
    compilation_cache: &mut CompilationCache,
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
    if compilation_cache.failed_packages.contains(&package_info) {
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
        );
        if res.is_err() {
            eprintln!("{} at: {}", res.unwrap_err(), version);
            return None;
        }
    }
    Some(package_info)
}

fn dump_and_compile_from_package_metadata(
    package_info: PackageInfo,
    root_dir: PathBuf,
    dep_map: &HashMap<(AccountAddress, String), PackageMetadata>,
    compilation_cache: &mut CompilationCache,
) -> anyhow::Result<()> {
    let root_package_dir = root_dir.join(format!("{}", package_info,));
    if compilation_cache.failed_packages.contains(&package_info) {
        return Err(anyhow::Error::msg("compilation failed"));
    }
    if !root_package_dir.exists() {
        std::fs::create_dir_all(root_package_dir.as_path())?;
    }
    let root_package_metadata = dep_map
        .get(&(package_info.address, package_info.package_name.clone()))
        .unwrap();
    // step 1: unzip and save the source code into src into corresponding folder
    let sources_dir = root_package_dir.join("sources");
    std::fs::create_dir_all(sources_dir.as_path())?;
    let modules = root_package_metadata.modules.clone();
    for module in modules {
        let module_path = sources_dir.join(format!("{}.move", module.name));
        if !module_path.exists() {
            File::create(module_path.clone()).expect("Error encountered while creating file!");
        };
        let source_str = unzip_metadata_str(&module.source).unwrap();
        std::fs::write(&module_path.clone(), source_str).unwrap();
    }

    // step 2: unzip, parse the manifest file
    let manifest_u8 = root_package_metadata.manifest.clone();
    let manifest_str = unzip_metadata_str(&manifest_u8).unwrap();
    let mut manifest =
        parse_source_manifest(parse_move_manifest_string(manifest_str.clone()).unwrap()).unwrap();

    let fix_manifest_dep = |dep: &mut Dependency, local_str: &str| {
        dep.git_info = None;
        dep.subst = None;
        dep.version = None;
        dep.digest = None;
        dep.node_info = None;
        dep.local = PathBuf::from("..").join(local_str); // PathBuf::from(local_str);
    };

    // step 3: fix the manifest file and recursively dump the code it depends
    let manifest_deps = &mut manifest.dependencies;
    for manifest_dep in manifest_deps {
        let manifest_dep_name = manifest_dep.0.as_str();
        let dep = manifest_dep.1;
        for pack_dep in &root_package_metadata.deps {
            let pack_dep_address = pack_dep.account;
            let pack_dep_name = pack_dep.clone().package_name;
            if pack_dep_name == manifest_dep_name {
                if is_aptos_package(&pack_dep_name) {
                    fix_manifest_dep(
                        dep,
                        &format!(
                            "{}/{}",
                            APTOS_COMMONS,
                            get_aptos_dir(&pack_dep_name).unwrap()
                        ),
                    );
                    break;
                }
                let dep_metadata_opt = dep_map.get(&(pack_dep_address, pack_dep_name.clone()));
                if let Some(dep_metadata) = dep_metadata_opt {
                    let package_info = PackageInfo {
                        address: pack_dep_address,
                        package_name: pack_dep_name.clone(),
                        upgrade_number: Some(dep_metadata.clone().upgrade_number),
                    };
                    let path_str = format!("{}", package_info);
                    fix_manifest_dep(dep, &path_str);
                    dump_and_compile_from_package_metadata(
                        package_info,
                        root_dir.clone(),
                        dep_map,
                        compilation_cache,
                    )?;
                }
                break;
            }
        }
    }

    // step 4: dump the fixed manifest file
    let toml_path = root_package_dir.join("Move.toml");
    std::fs::write(toml_path, manifest.to_string()).unwrap();

    // step 5: test whether the code can be compiled
    if !compilation_cache
        .compiled_package_map
        .contains_key(&package_info)
    {
        let package_v1 = compile_package(root_package_dir.clone(), &package_info, "baseline");
        if let Ok(built_package) = package_v1 {
            compilation_cache
                .compiled_package_map
                .insert(package_info.clone(), built_package);
        } else {
            if !compilation_cache.failed_packages.contains(&package_info) {
                compilation_cache.failed_packages.insert(package_info);
            }
            return Err(anyhow::Error::msg("compilation failed at v1"));
        }
    }
    Ok(())
}

fn compile_package(
    root_dir: PathBuf,
    package_info: &PackageInfo,
    version: &str,
) -> anyhow::Result<CompiledPackage> {
    let mut build_options = aptos_framework::BuildOptions {
        ..Default::default()
    };
    build_options
        .named_addresses
        .insert(package_info.package_name.clone(), package_info.address);
    let compiled_package = BuiltPackage::build(root_dir, build_options);
    if let Ok(built_package) = compiled_package {
        Ok(built_package.package)
    } else {
        Err(anyhow::Error::msg(format!(
            "compilation failed for compiler: {}",
            version
        )))
    }
}

impl From<TransactionBlock> for Workload {
    fn from(txn_block: TransactionBlock) -> Self {
        assert!(!txn_block.transactions.is_empty());

        let end = txn_block.begin_version + txn_block.transactions.len() as Version;
        let transaction_slice_metadata =
            TransactionSliceMetadata::chunk(txn_block.begin_version, end);

        let signature_verified_txns = into_signature_verified_block(txn_block.transactions);
        let txn_provider = DefaultTxnProvider::new(signature_verified_txns);

        Self {
            txn_provider,
            transaction_slice_metadata,
        }
    }
}
