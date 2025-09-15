// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::{
    natives::code::PackageMetadata, unzip_metadata_str, BuiltPackage, APTOS_PACKAGES,
};
use aptos_transaction_simulation::InMemoryStateStore;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Transaction,
    write_set::WriteSet,
};
use rocksdb::{DBWithThreadMode, SingleThreaded, DB};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

mod data_collection;
mod data_state_view;
mod execution;
mod online_execution;

pub use data_collection::*;
pub use execution::*;
use legacy_move_compiler::compiled_unit::CompiledUnitEnum;
use move_core_types::language_storage::ModuleId;
use move_model::metadata::CompilerVersion;
use move_package::{
    compilation::compiled_package::CompiledPackage,
    source_package::{
        manifest_parser::{parse_move_manifest_string, parse_source_manifest},
        parsed_manifest::Dependency,
    },
};
pub use online_execution::*;

const APTOS_PACKAGES_DIR_NAMES: [&str; 5] = [
    "aptos-framework",
    "move-stdlib",
    "aptos-stdlib",
    "aptos-token",
    "aptos-token-objects",
];

const STATE_DATA: &str = "state_data";
const WRITE_SET_DATA: &str = "write_set_data";
const INDEX_FILE: &str = "version_index.txt";
const ERR_LOG: &str = "err_log.txt";
const ROCKS_INDEX_DB: &str = "rocks_txn_idx_db";
pub const APTOS_COMMONS: &str = "aptos-commons";
const MAX_TO_FLUSH: usize = 50000;

struct IndexWriter {
    index_writer: BufWriter<File>,
    err_logger: BufWriter<File>,
    version_vec: Vec<u64>,
    counter: usize,
}

impl IndexWriter {
    pub fn new(root: &Path) -> Self {
        let create_file = |file_name: &str| -> File {
            let path = root.to_path_buf().join(file_name);
            if !path.exists() {
                File::create(path).expect("Error encountered while creating file!")
            } else {
                OpenOptions::new().append(true).open(path).unwrap()
            }
        };
        let index_file = create_file(INDEX_FILE);
        let err_log = create_file(ERR_LOG);
        Self {
            index_writer: BufWriter::with_capacity(4096 * 1024 /* 4096KB */, index_file),
            err_logger: BufWriter::with_capacity(4096 * 1024 /* 4096KB */, err_log),
            version_vec: vec![],
            counter: 0,
        }
    }

    pub fn reset_vec(&mut self) {
        self.version_vec = vec![];
    }

    pub fn add_version(&mut self, version: u64) {
        self.version_vec.push(version);
    }

    pub fn dump_version(&mut self) {
        self.version_vec.sort();
        self.version_vec.iter().for_each(|&version| {
            self.index_writer
                .write_fmt(format_args!("{}\n", version))
                .unwrap()
        });
        self.counter += self.version_vec.len();
        self.reset_vec();
        if self.counter > MAX_TO_FLUSH {
            self.flush_writer();
        }
    }

    pub fn write_err(&mut self, err_msg: &str) {
        self.err_logger
            .write_fmt(format_args!("{}\n", err_msg))
            .unwrap();
        self.err_logger.flush().unwrap();
    }

    pub fn flush_writer(&mut self) {
        self.index_writer.flush().unwrap();
        self.counter = 0;
    }
}

struct IndexReader {
    index_reader: BufReader<File>,
    _version_cache: Vec<u64>,
}

impl IndexReader {
    pub fn check_availability(root: &Path) -> bool {
        root.to_path_buf().join(INDEX_FILE).exists()
    }

    pub fn new(root: &Path) -> Self {
        let index_path = root.to_path_buf().join(INDEX_FILE);
        let index_file = File::open(index_path).unwrap();
        let index_reader = BufReader::new(index_file);
        Self {
            index_reader,
            _version_cache: vec![],
        }
    }

    pub fn _load_all_versions(&mut self) {
        loop {
            let next_val = self.get_next_version();
            if next_val.is_err() {
                continue;
            }
            if let Some(val) = next_val.unwrap() {
                self._version_cache.push(val);
            } else {
                break;
            }
        }
    }

    pub fn get_next_version(&mut self) -> Result<Option<u64>, ()> {
        let mut cur_idx = String::new();
        let num_bytes = self.index_reader.read_line(&mut cur_idx).unwrap();
        if num_bytes == 0 {
            return Ok(None);
        }
        let indx = cur_idx.trim().parse();
        if indx.is_ok() {
            Ok(indx.ok())
        } else {
            Err(())
        }
    }

    pub fn get_next_version_ge(&mut self, version: u64) -> Option<u64> {
        loop {
            let next_val = self.get_next_version();
            if next_val.is_err() {
                continue;
            }
            if let Some(val) = next_val.unwrap() {
                if val >= version {
                    return Some(val);
                }
            } else {
                break;
            }
        }
        None
    }
}

struct DataManager {
    state_data_dir_path: PathBuf,
    write_set_dir_path: PathBuf,
    db: DBWithThreadMode<SingleThreaded>,
}

impl DataManager {
    pub fn new_with_dir_creation(root: &Path) -> Self {
        let dm = Self::new(root);
        if !dm.state_data_dir_path.exists() {
            std::fs::create_dir_all(dm.state_data_dir_path.as_path()).unwrap();
        }
        if !dm.write_set_dir_path.exists() {
            std::fs::create_dir_all(dm.write_set_dir_path.as_path()).unwrap();
        }
        dm
    }

    pub fn new(root: &Path) -> Self {
        let db = DB::open_default(root.to_path_buf().join(ROCKS_INDEX_DB)).unwrap();
        let state_data_dir_path = root.join(STATE_DATA);
        let write_set_dir_path = root.join(WRITE_SET_DATA);
        Self {
            state_data_dir_path,
            write_set_dir_path,
            db,
        }
    }

    pub fn check_dir_availability(&self) -> bool {
        if !(self.state_data_dir_path.exists() && self.write_set_dir_path.exists()) {
            return false;
        }
        true
    }

    pub fn dump_state_data(&self, version: u64, state: &HashMap<StateKey, StateValue>) {
        let state_path = self.state_data_dir_path.join(format!("{}_state", version));
        if !state_path.exists() {
            let mut data_state_file = File::create(state_path).unwrap();
            let state_store = InMemoryStateStore::new_with_state_values(state.to_owned());
            data_state_file
                .write_all(&bcs::to_bytes(&state_store.to_btree_map()).unwrap())
                .unwrap();
        }
    }

    pub fn dump_write_set(&self, version: u64, write_set: &WriteSet) {
        let write_set_path = self
            .write_set_dir_path
            .join(format!("{}_write_set", version));
        if !write_set_path.exists() {
            let mut write_set_file = File::create(write_set_path).unwrap();
            write_set_file
                .write_all(&bcs::to_bytes(&write_set).unwrap())
                .unwrap();
        }
    }

    pub fn dump_txn_index(&self, version: u64, version_idx: &TxnIndex) {
        self.db
            .put(
                bcs::to_bytes(&version).unwrap(),
                bcs::to_bytes(&version_idx).unwrap(),
            )
            .unwrap();
    }

    pub fn get_txn_index(&self, version: u64) -> Option<TxnIndex> {
        let db_val = self.db.get(bcs::to_bytes(&version).unwrap());
        if let Ok(Some(val)) = db_val {
            let txn_idx = bcs::from_bytes::<TxnIndex>(&val).unwrap();
            Some(txn_idx)
        } else {
            None
        }
    }

    pub fn get_state(&self, version: u64) -> InMemoryStateStore {
        let state_path = self.state_data_dir_path.join(format!("{}_state", version));
        let mut data_state_file = File::open(state_path).unwrap();
        let mut buffer = Vec::<u8>::new();
        data_state_file.read_to_end(&mut buffer).unwrap();
        InMemoryStateStore::new_with_state_values(
            bcs::from_bytes::<BTreeMap<StateKey, StateValue>>(&buffer).unwrap(),
        )
    }
}

fn is_aptos_package(package_name: &str) -> bool {
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

fn check_aptos_packages_availability(path: PathBuf) -> bool {
    if !path.exists() {
        return false;
    }
    for package in APTOS_PACKAGES {
        if !path.join(get_aptos_dir(package).unwrap()).exists() {
            return false;
        }
    }
    true
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

#[derive(Default)]
struct CompilationCache {
    compiled_package_map: HashMap<PackageInfo, CompiledPackage>,
    failed_packages_v1: HashSet<PackageInfo>,
    failed_packages_v2: HashSet<PackageInfo>,
    compiled_package_cache_v1: HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
    compiled_package_cache_v2: HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub(crate) struct PackageInfo {
    address: AccountAddress,
    package_name: String,
    upgrade_number: Option<u64>,
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

impl PackageInfo {
    pub fn is_compilable(&self) -> bool {
        self.address != AccountAddress::ZERO
    }

    pub fn non_compilable_info() -> Self {
        Self {
            address: AccountAddress::ZERO,
            package_name: "".to_string(),
            upgrade_number: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TxnIndex {
    version: u64,
    package_info: PackageInfo,
    txn: Transaction,
}

fn generate_compiled_blob(
    package_info: &PackageInfo,
    compiled_package: &CompiledPackage,
    compiled_blobs: &mut HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
) {
    if compiled_blobs.contains_key(package_info) {
        return;
    }
    let root_modules = &compiled_package.root_compiled_units;
    let mut blob_map = HashMap::new();
    for compiled_module in root_modules {
        if let CompiledUnitEnum::Module(module) = &compiled_module.unit {
            let module_blob = compiled_module.unit.serialize(None);
            blob_map.insert(module.module.self_id(), module_blob);
        }
    }
    compiled_blobs.insert(package_info.clone(), blob_map);
}

fn compile_aptos_packages(
    aptos_commons_path: &Path,
    compiled_package_map: &mut HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
    v2_flag: bool,
) -> anyhow::Result<()> {
    for package in APTOS_PACKAGES {
        let root_package_dir = aptos_commons_path.join(get_aptos_dir(package).unwrap());
        let compiler_version = if v2_flag {
            Some(CompilerVersion::latest_stable())
        } else {
            Some(CompilerVersion::V1)
        };
        // For simplicity, all packages including aptos token are stored under 0x1 in the map
        let package_info = PackageInfo {
            address: AccountAddress::ONE,
            package_name: package.to_string(),
            upgrade_number: None,
        };
        let compiled_package = compile_package(root_package_dir, &package_info, compiler_version);
        if let Ok(built_package) = compiled_package {
            generate_compiled_blob(&package_info, &built_package, compiled_package_map);
        } else {
            return Err(anyhow::Error::msg(format!(
                "package {} cannot be compiled",
                package
            )));
        }
    }
    Ok(())
}

fn compile_package(
    root_dir: PathBuf,
    package_info: &PackageInfo,
    compiler_verion: Option<CompilerVersion>,
) -> anyhow::Result<CompiledPackage> {
    let mut build_options = aptos_framework::BuildOptions {
        compiler_version: compiler_verion,
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
            "compilation failed for compiler: {:?}",
            compiler_verion
        )))
    }
}

fn dump_and_compile_from_package_metadata(
    package_info: PackageInfo,
    root_dir: PathBuf,
    dep_map: &HashMap<(AccountAddress, String), PackageMetadata>,
    compilation_cache: &mut CompilationCache,
    execution_mode: Option<ExecutionMode>,
) -> anyhow::Result<()> {
    let root_package_dir = root_dir.join(format!("{}", package_info,));
    if compilation_cache.failed_packages_v1.contains(&package_info) {
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
                        execution_mode,
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
        let package_v1 = compile_package(
            root_package_dir.clone(),
            &package_info,
            Some(CompilerVersion::V1),
        );
        if let Ok(built_package) = package_v1 {
            if execution_mode.is_some_and(|mode| mode.is_v1_or_compare()) {
                generate_compiled_blob(
                    &package_info,
                    &built_package,
                    &mut compilation_cache.compiled_package_cache_v1,
                );
            }
            compilation_cache
                .compiled_package_map
                .insert(package_info.clone(), built_package);
        } else {
            if !compilation_cache.failed_packages_v1.contains(&package_info) {
                compilation_cache.failed_packages_v1.insert(package_info);
            }
            return Err(anyhow::Error::msg("compilation failed at v1"));
        }
        if execution_mode.is_some_and(|mode| mode.is_v2_or_compare()) {
            let package_v2 = compile_package(
                root_package_dir,
                &package_info,
                Some(CompilerVersion::latest_stable()),
            );
            if let Ok(built_package) = package_v2 {
                generate_compiled_blob(
                    &package_info,
                    &built_package,
                    &mut compilation_cache.compiled_package_cache_v2,
                );
            } else {
                if !compilation_cache.failed_packages_v1.contains(&package_info) {
                    compilation_cache.failed_packages_v1.insert(package_info);
                }
                return Err(anyhow::Error::msg("compilation failed at v2"));
            }
        }
    }
    Ok(())
}
