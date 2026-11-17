// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub use aptos_comparison_testing_dump_format::APTOS_COMMONS;
use aptos_comparison_testing_dump_format::{
    DataManager, IndexReader, IndexWriter, PackageInfo, TxnIndex,
};
use aptos_framework::{
    natives::code::PackageMetadata, unzip_metadata_str, BuiltPackage, APTOS_PACKAGES,
};
use aptos_types::account_address::AccountAddress;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
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
use move_model::metadata::LanguageVersion;
use move_package::{
    compilation::compiled_package::CompiledPackage,
    source_package::{
        manifest_parser::{parse_move_manifest_string, parse_source_manifest},
        parsed_manifest::Dependency,
    },
};
pub use online_execution::*;

const APTOS_PACKAGES_DIR_NAMES: [&str; 7] = [
    "aptos-framework",
    "move-stdlib",
    "aptos-stdlib",
    "aptos-token",
    "aptos-token-objects",
    "aptos-trading",
    "aptos-experimental",
];

pub const DISABLE_SPEC_CHECK: &str = "spec-check=off";
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

async fn download_aptos_packages(path: &Path, branch_opt: Option<String>) -> anyhow::Result<()> {
    let git_url = "https://github.com/aptos-labs/aptos-core";
    let tmp_dir = TempDir::new()?;
    let branch = branch_opt.unwrap_or("main".to_string());
    Command::new("git")
        .args([
            "clone",
            "--branch",
            &branch,
            git_url,
            tmp_dir.path().to_str().unwrap(),
            "--depth",
            "1",
        ])
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

pub async fn prepare_aptos_packages(
    path: PathBuf,
    branch_opt: Option<String>,
    force_override_framework: bool,
) {
    let mut download_flag = true;
    if path.exists() {
        if force_override_framework {
            std::fs::remove_dir_all(path.clone()).unwrap();
        } else {
            let mut need_download = false;
            for package_name in APTOS_PACKAGES {
                let target_framework_path = path.clone().join(get_aptos_dir(package_name).unwrap());
                if !target_framework_path.exists() {
                    need_download = true;
                    break;
                }
            }
            if need_download {
                std::fs::remove_dir_all(path.clone()).unwrap();
            } else {
                download_flag = false;
            }
        }
    }
    if download_flag {
        println!("Downloading aptos packages...");
        std::fs::create_dir_all(path.clone()).unwrap();
        download_aptos_packages(&path, branch_opt).await.unwrap();
    }
}

#[derive(Default)]
struct CompilationCache {
    compiled_package_map: HashMap<PackageInfo, CompiledPackage>,
    failed_packages_base: HashSet<PackageInfo>,
    failed_packages_compared: HashSet<PackageInfo>,
    base_compiled_package_cache: HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
    compared_compiled_package_cache: HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
    /// Packages already dumped to disk; consulted when compilation is skipped.
    dumped_packages: HashSet<PackageInfo>,
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
    experiments: &[String],
    version: &str,
) -> anyhow::Result<()> {
    for package in APTOS_PACKAGES {
        let root_package_dir = aptos_commons_path.join(get_aptos_dir(package).unwrap());
        if !root_package_dir.exists() {
            continue;
        }
        // For simplicity, all packages including aptos token are stored under 0x1 in the map
        let package_info = PackageInfo {
            address: AccountAddress::ONE,
            package_name: package.to_string(),
            upgrade_number: None,
        };
        let compiled_package =
            compile_package(root_package_dir, &package_info, experiments, version);
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
    experiments: &[String],
    version: &str,
) -> anyhow::Result<CompiledPackage> {
    let mut build_options = aptos_framework::BuildOptions {
        language_version: Some(LanguageVersion::latest()),
        experiments: experiments.to_vec(),
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
            "compilation failed for the package:{} when using compiler: {}",
            package_info.package_name.clone(),
            version
        )))
    }
}

fn dump_and_compile_from_package_metadata(
    package_info: PackageInfo,
    root_dir: PathBuf,
    dep_map: &HashMap<(AccountAddress, String), PackageMetadata>,
    compilation_cache: &mut CompilationCache,
    execution_mode: Option<ExecutionMode>,
    base_experiments: &[String],
    compared_experiments: &[String],
    skip_source_compilation: bool,
) -> anyhow::Result<()> {
    if skip_source_compilation && compilation_cache.dumped_packages.contains(&package_info) {
        return Ok(());
    }
    let root_package_dir = root_dir.join(format!("{}", package_info,));
    if compilation_cache
        .failed_packages_base
        .contains(&package_info)
    {
        return Err(anyhow::Error::msg(format!(
            "compilation failed for the package:{} when using compiler v1",
            package_info.package_name
        )));
    }
    if compilation_cache
        .failed_packages_compared
        .contains(&package_info)
    {
        return Err(anyhow::Error::msg(format!(
            "compilation failed for the package:{} when using compiler v2",
            package_info.package_name
        )));
    }
    let root_package_metadata = dep_map
        .get(&(package_info.address, package_info.package_name.clone()))
        .unwrap();
    if !root_package_dir.exists() {
        std::fs::create_dir_all(root_package_dir.as_path())?;
    }
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
        std::fs::write(module_path.clone(), source_str).unwrap();
    }

    // step 2: unzip, parse the manifest file
    let manifest_u8 = root_package_metadata.manifest.clone();
    let manifest_str = unzip_metadata_str(&manifest_u8).unwrap();
    let mut manifest =
        parse_source_manifest(parse_move_manifest_string(manifest_str.clone()).unwrap()).unwrap();
    let mut updated_addresses_map = BTreeMap::new();
    if manifest.addresses.is_some() {
        for x in manifest.addresses.clone().unwrap() {
            if x.1.is_some() || x.0 == package_info.package_name.clone().into() {
                updated_addresses_map.insert(x.0, x.1);
            } else {
                updated_addresses_map.insert(x.0, Some(package_info.address));
            }
        }
        manifest.addresses = Some(updated_addresses_map);
    }

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
                        base_experiments,
                        compared_experiments,
                        skip_source_compilation,
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
    if skip_source_compilation {
        compilation_cache
            .dumped_packages
            .insert(package_info.clone());
        return Ok(());
    }
    if !compilation_cache
        .compiled_package_map
        .contains_key(&package_info)
    {
        let package_v1 = compile_package(
            root_package_dir.clone(),
            &package_info,
            base_experiments,
            "base",
        );
        if let Ok(built_package) = package_v1 {
            if execution_mode.is_some_and(|mode| mode.is_v1_or_compare()) {
                generate_compiled_blob(
                    &package_info,
                    &built_package,
                    &mut compilation_cache.base_compiled_package_cache,
                );
            }
            compilation_cache
                .compiled_package_map
                .insert(package_info.clone(), built_package);
        } else {
            if !compilation_cache
                .failed_packages_base
                .contains(&package_info)
            {
                compilation_cache
                    .failed_packages_base
                    .insert(package_info.clone());
            }
            return Err(anyhow::Error::msg(format!(
                "compilation failed for the package:{} when using compiler v1",
                package_info.package_name
            )));
        }
        if execution_mode.is_some_and(|mode| mode.is_v2_or_compare()) {
            let package_v2 = compile_package(
                root_package_dir,
                &package_info,
                compared_experiments,
                "compared",
            );
            if let Ok(built_package) = package_v2 {
                generate_compiled_blob(
                    &package_info,
                    &built_package,
                    &mut compilation_cache.compared_compiled_package_cache,
                );
            } else {
                if !compilation_cache
                    .failed_packages_compared
                    .contains(&package_info)
                {
                    compilation_cache
                        .failed_packages_compared
                        .insert(package_info.clone());
                }
                return Err(anyhow::Error::msg(format!(
                    "compilation failed for the package:{} when using compiler v2",
                    package_info.package_name
                )));
            }
        }
    }
    Ok(())
}
