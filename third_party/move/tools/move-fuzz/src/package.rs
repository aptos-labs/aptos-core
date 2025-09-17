// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::Account,
    deps::{PkgManifest, PkgNamedAddr},
    language::LanguageSetting,
    state::{
        load_package_build_cache_info, save_package_build_cache_info,
        PersistedPackageBuildCacheInfo, PACKAGE_BUILD_CACHE_INFO_FILENAME,
    },
    utils,
};
use anyhow::{bail, Context, Result};
use aptos_framework::{
    natives::code::{ModuleMetadata, PackageDep, PackageMetadata, UpgradePolicy},
    zip_metadata_str, BuildOptions, BuiltPackage, UPGRADE_POLICY_CUSTOM_FIELD,
};
use aptos_gas_schedule::{
    InitialGasSchedule, MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION,
};
use aptos_types::on_chain_config::{
    aptos_test_feature_flags_genesis, Features, TimedFeaturesBuilder,
};
use aptos_vm::natives;
use fs_extra::dir;
use legacy_move_compiler::{compiled_unit::CompiledUnit, shared::NumericalAddress};
use move_core_types::account_address::AccountAddress;
use move_package::{
    compilation::compiled_package::{
        CompiledPackage, CompiledPackageInfo, CompiledUnitWithSource, OnDiskCompiledPackage,
    },
    source_package::{
        layout::SourcePackageLayout,
        manifest_parser::{parse_move_manifest_string, parse_source_manifest},
    },
    BuildConfig, CompilerConfig,
};
use move_unit_test::{
    package_test::{run_move_unit_tests, UnitTestResult},
    UnitTestingConfig,
};
use move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE;
use sha3::{Digest as Sha3Digest, Sha3_256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub enum FuzzPackage {
    Fresh(BuiltPackage),
    Cached(CompiledPackage),
}

impl FuzzPackage {
    pub fn from_built(built: BuiltPackage) -> Self {
        Self::Fresh(built)
    }

    pub fn compiled_package(&self) -> &CompiledPackage {
        match self {
            Self::Fresh(package) => &package.package,
            Self::Cached(package) => package,
        }
    }

    pub fn compiled_package_info(&self) -> &CompiledPackageInfo {
        &self.compiled_package().compiled_package_info
    }

    pub fn root_compiled_units(&self) -> &[CompiledUnitWithSource] {
        &self.compiled_package().root_compiled_units
    }

    pub fn name(&self) -> &str {
        self.compiled_package_info().package_name.as_str()
    }

    pub fn inferred_bytecode_version(&self) -> u32 {
        let config = &self.compiled_package_info().build_flags.compiler_config;
        config
            .language_version
            .unwrap_or_default()
            .infer_bytecode_version(config.bytecode_version)
    }

    pub fn extract_code(&self) -> Vec<Vec<u8>> {
        match self {
            Self::Fresh(package) => package.extract_code(),
            Self::Cached(package) => {
                let bytecode_version = Some(self.inferred_bytecode_version());
                package
                    .root_modules()
                    .map(|unit_with_source| unit_with_source.unit.serialize(bytecode_version))
                    .collect()
            },
        }
    }

    pub fn extract_metadata(&self, manifest_path: &Path) -> Result<PackageMetadata> {
        match self {
            Self::Fresh(package) => Ok(package.extract_metadata()?),
            Self::Cached(package) => extract_cached_metadata(package, manifest_path),
        }
    }
}

fn root_unit_names(package: &CompiledPackage) -> (Vec<String>, Vec<String>) {
    let mut modules = BTreeSet::new();
    let mut scripts = BTreeSet::new();
    for unit in &package.root_compiled_units {
        match &unit.unit {
            CompiledUnit::Module(module) => {
                modules.insert(module.name.as_str().to_string());
            },
            CompiledUnit::Script(script) => {
                scripts.insert(script.name.as_str().to_string());
            },
        }
    }
    (modules.into_iter().collect(), scripts.into_iter().collect())
}

fn dependency_name_from_source_path(source_path: &Path) -> Option<String> {
    let mut components = source_path
        .components()
        .filter_map(|component| component.as_os_str().to_str());
    while let Some(component) = components.next() {
        if component == "dependencies" {
            return components.next().map(str::to_string);
        }
    }
    None
}

fn collect_named_addresses(
    pkg: &PkgManifest,
    named_accounts: &BTreeMap<String, Account>,
    mapping: &mut BTreeMap<String, AccountAddress>,
) -> Result<()> {
    // collect the package itself
    for (name, addr) in &pkg.named_addresses {
        match addr {
            PkgNamedAddr::Fixed(_) => continue,
            PkgNamedAddr::Unset | PkgNamedAddr::Devel(_) => (),
        }
        match named_accounts.get(name) {
            None => bail!("named address not assigned: {}", name),
            Some(account) => {
                let address = account.address();
                if let Some(existing) = mapping.get(name) {
                    if *existing != address {
                        unreachable!("conflicting named address assignment: {}", name);
                    }
                } else {
                    mapping.insert(name.clone(), address);
                }
            },
        }
    }

    // collect the dependencies
    for dep in pkg.deps.values() {
        collect_named_addresses(dep, named_accounts, mapping)?;
    }

    // done
    Ok(())
}

fn assigned_named_addresses(
    pkg: &PkgManifest,
    named_accounts: &BTreeMap<String, Account>,
) -> Result<BTreeMap<String, AccountAddress>> {
    let mut named_addresses = BTreeMap::new();
    collect_named_addresses(pkg, named_accounts, &mut named_addresses)?;
    Ok(named_addresses)
}

fn source_paths_for_build(pkg: &PkgManifest, dev_mode: bool) -> Vec<PathBuf> {
    let mut paths = vec![
        pkg.path.join(SourcePackageLayout::Sources.path()),
        pkg.path.join(SourcePackageLayout::Scripts.path()),
    ];
    if dev_mode {
        paths.push(pkg.path.join(SourcePackageLayout::Examples.path()));
        paths.push(pkg.path.join(SourcePackageLayout::Tests.path()));
    }
    paths.push(pkg.path.join(SourcePackageLayout::Manifest.path()));
    paths
        .into_iter()
        .filter(|path| path.exists())
        .collect::<Vec<_>>()
}

fn source_digest_for_build(pkg: &PkgManifest, dev_mode: bool) -> Result<String> {
    let mut hashed_files = Vec::new();
    for path in source_paths_for_build(pkg, dev_mode) {
        if path.is_file() {
            hash_digest_input(&mut hashed_files, &path)?;
        } else {
            for entry in WalkDir::new(path).follow_links(true).into_iter() {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                if entry.file_type().is_file() {
                    hash_digest_input(&mut hashed_files, entry.path())?;
                }
            }
        }
    }
    hashed_files.sort();

    let mut hasher = Sha3_256::new();
    for file_hash in hashed_files {
        hasher.update(file_hash.as_bytes());
    }
    Ok(format!("{:X}", hasher.finalize()))
}

fn hash_digest_input(hashed_files: &mut Vec<String>, path: &Path) -> Result<()> {
    let should_hash = match path.extension().and_then(|ext| ext.to_str()) {
        Some("move") => true,
        _ => path.ends_with(SourcePackageLayout::Manifest.path()),
    };
    if !should_hash {
        return Ok(());
    }
    let contents = fs::read(path)?;
    hashed_files.push(format!("{:X}", Sha3_256::digest(&contents)));
    Ok(())
}

pub fn build_cache_fingerprint(
    pkg: &PkgManifest,
    named_accounts: &BTreeMap<String, Account>,
    language: LanguageSetting,
    dev_mode: bool,
    dependency_fingerprints: &BTreeMap<String, String>,
) -> Result<String> {
    let named_addresses = assigned_named_addresses(pkg, named_accounts)?;
    let source_digest = source_digest_for_build(pkg, dev_mode)?;
    let compiler_config = language.derive_compilation_config();
    let mut hasher = Sha3_256::new();
    hasher.update(b"move-fuzz-package-build-cache-v1");
    hasher.update(pkg.name.as_bytes());
    hasher.update(source_digest.as_bytes());
    hasher.update([dev_mode as u8]);
    for (name, addr) in named_addresses {
        hasher.update(name.as_bytes());
        hasher.update(addr.to_string().as_bytes());
    }
    hasher.update(format!("{:?}", compiler_config.bytecode_version).as_bytes());
    hasher.update(format!("{:?}", compiler_config.compiler_version).as_bytes());
    hasher.update(format!("{:?}", compiler_config.language_version).as_bytes());
    hasher.update([compiler_config.skip_attribute_checks as u8]);
    for attr in &compiler_config.known_attributes {
        hasher.update(attr.as_bytes());
    }
    for exp in &compiler_config.experiments {
        hasher.update(exp.as_bytes());
    }
    for (dep_name, dep_fingerprint) in dependency_fingerprints {
        hasher.update(dep_name.as_bytes());
        hasher.update(dep_fingerprint.as_bytes());
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn package_cache_slot_dir(
    package_cache_root: &Path,
    package_name: &str,
    manifest_identity: &str,
) -> PathBuf {
    let mut hasher = Sha3_256::new();
    hasher.update(package_name.as_bytes());
    hasher.update(b"\0");
    hasher.update(manifest_identity.as_bytes());
    let slot = hex::encode(hasher.finalize());
    package_cache_root.join(slot)
}

pub fn load_build_cache(
    cache_slot_dir: &Path,
    manifest_path: &Path,
    manifest_identity: &str,
    expected_package_name: &str,
    expected_fingerprint: &str,
) -> Result<Option<FuzzPackage>> {
    let cache_info_path = cache_slot_dir.join(PACKAGE_BUILD_CACHE_INFO_FILENAME);
    let Some(cache_info) = load_package_build_cache_info(&cache_info_path)? else {
        return Ok(None);
    };
    if cache_info.version != crate::state::PACKAGE_BUILD_CACHE_INFO_VERSION
        || cache_info.package_name != expected_package_name
        || cache_info.manifest_identity != manifest_identity
        || cache_info.fingerprint != expected_fingerprint
    {
        return Ok(None);
    }

    let build_info_path = cache_slot_dir
        .join(SourcePackageLayout::Build.path())
        .join(expected_package_name)
        .join("BuildInfo.yaml");
    if !build_info_path.exists() {
        return Ok(None);
    }

    let mut package = OnDiskCompiledPackage::from_path(&build_info_path)
        .with_context(|| format!("failed to load cached build {}", build_info_path.display()))?
        .into_compiled_package()
        .with_context(|| {
            format!(
                "failed to decode cached build {}",
                build_info_path.display()
            )
        })?;
    if package.compiled_package_info.package_name.as_str() != expected_package_name {
        bail!(
            "cached build package name mismatch for {}: found {}",
            manifest_path.display(),
            package.compiled_package_info.package_name
        );
    }
    let loaded_units = std::mem::take(&mut package.root_compiled_units);
    let mut root_units = Vec::new();
    let mut dep_units = Vec::new();
    for unit in loaded_units {
        if let Some(dep_name) = dependency_name_from_source_path(&unit.source_path) {
            dep_units.push((dep_name.into(), unit));
        } else {
            root_units.push(unit);
        }
    }
    package.root_compiled_units = root_units;
    package.deps_compiled_units = dep_units;

    let allowed_modules: BTreeSet<_> = cache_info.root_module_names.into_iter().collect();
    let allowed_scripts: BTreeSet<_> = cache_info.root_script_names.into_iter().collect();
    package.root_compiled_units.retain(|unit| match &unit.unit {
        CompiledUnit::Module(module) => allowed_modules.contains(module.name.as_str()),
        CompiledUnit::Script(script) => allowed_scripts.contains(script.name.as_str()),
    });
    Ok(Some(FuzzPackage::Cached(package)))
}

pub fn save_build_cache(
    cache_slot_dir: &Path,
    pkg: &PkgManifest,
    built_package: &BuiltPackage,
    manifest_identity: &str,
    fingerprint: &str,
) -> Result<()> {
    let build_dir = pkg.path.join(SourcePackageLayout::Build.path());
    if !build_dir.is_dir() {
        bail!(
            "built package directory not found for cache population: {}",
            build_dir.display()
        );
    }

    if cache_slot_dir.exists() {
        fs::remove_dir_all(cache_slot_dir).with_context(|| {
            format!(
                "failed to clear existing package build cache {}",
                cache_slot_dir.display()
            )
        })?;
    }
    fs::create_dir_all(cache_slot_dir).with_context(|| {
        format!(
            "failed to create package build cache directory {}",
            cache_slot_dir.display()
        )
    })?;

    dir::copy(&build_dir, cache_slot_dir, &dir::CopyOptions::new())
        .with_context(|| format!("failed to copy build cache from {}", build_dir.display()))?;

    let (root_module_names, root_script_names) = root_unit_names(&built_package.package);
    let cache_info = PersistedPackageBuildCacheInfo::new(
        pkg.name.clone(),
        manifest_identity.to_string(),
        fingerprint.to_string(),
        root_module_names,
        root_script_names,
    );
    save_package_build_cache_info(
        &cache_slot_dir.join(PACKAGE_BUILD_CACHE_INFO_FILENAME),
        &cache_info,
    )?;
    Ok(())
}

fn extract_cached_metadata(
    package: &CompiledPackage,
    manifest_path: &Path,
) -> Result<PackageMetadata> {
    let source_digest = package
        .compiled_package_info
        .source_digest
        .map(|digest| digest.to_string())
        .unwrap_or_default();
    let manifest_file = manifest_path.join(SourcePackageLayout::Manifest.path());
    let manifest = fs::read_to_string(&manifest_file).with_context(|| {
        format!(
            "failed to read package manifest {}",
            manifest_file.display()
        )
    })?;
    let parsed = parse_source_manifest(parse_move_manifest_string(manifest.clone())?)?;
    let custom_props = parsed
        .package
        .custom_properties
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<BTreeMap<_, _>>();
    let manifest = zip_metadata_str(&manifest)?;
    let upgrade_policy = if let Some(value) = custom_props.get(UPGRADE_POLICY_CUSTOM_FIELD) {
        value.parse::<UpgradePolicy>()?
    } else {
        UpgradePolicy::compat()
    };
    let modules = package
        .root_modules()
        .map(|unit| ModuleMetadata {
            name: unit.unit.name().to_string(),
            source: vec![],
            source_map: vec![],
            extension: None,
        })
        .collect();
    let deps = package
        .deps_compiled_units
        .iter()
        .flat_map(|(name, unit)| match &unit.unit {
            CompiledUnit::Module(module) => Some(PackageDep {
                account: AccountAddress::new(module.address.into_bytes()),
                package_name: name.as_str().to_string(),
            }),
            CompiledUnit::Script(_) => None,
        })
        .chain(
            package
                .bytecode_deps
                .iter()
                .map(|(name, module)| PackageDep {
                    account: NumericalAddress::from_account_address(*module.self_addr())
                        .into_inner(),
                    package_name: name.as_str().to_string(),
                }),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(PackageMetadata {
        name: package
            .compiled_package_info
            .package_name
            .as_str()
            .to_string(),
        upgrade_policy,
        upgrade_number: 0,
        source_digest,
        manifest,
        modules,
        deps,
        extension: None,
    })
}

/// Build a package with the given language settings and named accounts
pub fn build(
    pkg: &PkgManifest,
    named_accounts: &BTreeMap<String, Account>,
    language: LanguageSetting,
    dev_mode: bool,
) -> Result<BuiltPackage> {
    // collect assigned addresses
    let named_addresses = assigned_named_addresses(pkg, named_accounts)?;

    // fill the build options
    let CompilerConfig {
        bytecode_version,
        language_version,
        compiler_version,
        known_attributes,
        skip_attribute_checks,
        print_errors: _,
        experiments,
    } = language.derive_compilation_config();

    let options = BuildOptions {
        dev: dev_mode,
        check_test_code: dev_mode,
        with_test_mode: dev_mode,
        named_addresses,
        forced_named_addresses: BTreeMap::new(),
        skip_fetch_latest_git_deps: true,
        bytecode_version,
        compiler_version,
        language_version,
        known_attributes,
        skip_attribute_checks,
        experiments,
        // following a minimal config for the rest of the options
        with_abis: false,
        with_docs: false,
        with_srcs: false,
        with_source_maps: false,
        with_error_map: false,
        install_dir: None,
        override_std: None,
        docgen_options: None,
    };

    // build the package
    // HACK: silence logging in compilation
    let package_built =
        utils::with_logging_disabled(|| BuiltPackage::build(pkg.path.clone(), options))?;
    Ok(package_built)
}

/// Run Move unit tests in the given package
pub fn unit_test(
    pkg: &PkgManifest,
    named_accounts: &BTreeMap<String, Account>,
    language: LanguageSetting,
    test_filter: Option<&str>,
    gas: bool,
    single_thread: bool,
) -> Result<()> {
    // collect assigned addresses
    let named_addresses = assigned_named_addresses(pkg, named_accounts)?;

    // fill the build options
    let build_config = BuildConfig {
        dev_mode: true,
        test_mode: true,
        skip_fetch_latest_git_deps: true,
        additional_named_addresses: named_addresses,
        compiler_config: language.derive_compilation_config(),
        // enable model generation explicitly
        generate_move_model: true,
        full_model_generation: true,
        // following a minimal config for the rest of the options
        override_std: None,
        generate_docs: false,
        generate_abis: false,
        install_dir: None,
        force_recompilation: false,
        fetch_deps_only: true,
        forced_named_addresses: BTreeMap::new(),
        verify_mode: false,
    };
    let test_config = UnitTestingConfig {
        filter: test_filter.map(|s| s.to_string()),
        num_threads: if single_thread { 1 } else { num_cpus::get() },
        // values not used at all
        named_address_values: vec![],
        // minimal config for the rest of the options
        list: false,
        dep_files: vec![],
        source_files: vec![],
        ignore_compile_warnings: true,
        report_statistics: false,
        report_storage_on_error: false,
        report_stacktrace_on_abort: true,
        verbose: false,
        fail_fast: false,
    };

    // setup gas and natives
    let (cost_table, native_gas, misc_gas) = if gas {
        (
            Some(INITIAL_COST_SCHEDULE.clone()),
            NativeGasParameters::initial(),
            MiscGasParameters::initial(),
        )
    } else {
        (
            None,
            NativeGasParameters::zeros(),
            MiscGasParameters::zeros(),
        )
    };
    let natives = natives::aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        native_gas,
        misc_gas,
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    );

    // run the tests
    let result = run_move_unit_tests(
        &pkg.path,
        build_config,
        test_config,
        natives,
        aptos_test_feature_flags_genesis(),
        None, // unlimited gas consumption
        cost_table,
        false, // TODO: enable coverage calculation
        &mut std::io::stdout(),
        true,
    )?;

    match result {
        UnitTestResult::Success => Ok(()),
        UnitTestResult::Failure => bail!("Move unit test failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_cache_fingerprint, collect_named_addresses, dependency_name_from_source_path,
        package_cache_slot_dir, source_digest_for_build,
    };
    use crate::{
        common::Account,
        deps::{PkgManifest, PkgNamedAddr},
        language::LanguageSetting,
    };
    use anyhow::Result;
    use move_core_types::account_address::AccountAddress;
    use std::{collections::BTreeMap, fs, path::PathBuf};
    use tempfile::TempDir;

    fn manifest(
        name: &str,
        named_addresses: BTreeMap<String, PkgNamedAddr>,
        deps: BTreeMap<String, PkgManifest>,
    ) -> PkgManifest {
        PkgManifest {
            name: name.to_string(),
            path: PathBuf::from(format!("/tmp/{name}")),
            version: (1, 0, 0).into(),
            deps,
            named_addresses,
        }
    }

    #[test]
    fn test_collect_named_addresses_recurses_and_skips_fixed() -> Result<()> {
        let dep = manifest(
            "dep",
            BTreeMap::from([
                ("dep".to_string(), PkgNamedAddr::Unset),
                (
                    "fixed".to_string(),
                    PkgNamedAddr::Fixed(AccountAddress::from_hex_literal("0x7")?),
                ),
            ]),
            BTreeMap::new(),
        );
        let root = manifest(
            "root",
            BTreeMap::from([("root".to_string(), PkgNamedAddr::Devel(AccountAddress::ONE))]),
            BTreeMap::from([("dep".to_string(), dep)]),
        );

        let named_accounts = BTreeMap::from([
            (
                "root".to_string(),
                Account::Ref(AccountAddress::from_hex_literal("0xa")?),
            ),
            (
                "dep".to_string(),
                Account::Ref(AccountAddress::from_hex_literal("0xb")?),
            ),
        ]);

        let mut mapping = BTreeMap::new();
        collect_named_addresses(&root, &named_accounts, &mut mapping)?;

        assert_eq!(mapping.len(), 2);
        assert_eq!(mapping["root"], AccountAddress::from_hex_literal("0xa")?);
        assert_eq!(mapping["dep"], AccountAddress::from_hex_literal("0xb")?);
        assert!(!mapping.contains_key("fixed"));
        Ok(())
    }

    #[test]
    fn test_collect_named_addresses_requires_assignments_for_unset_names() {
        let pkg = manifest(
            "root",
            BTreeMap::from([("missing".to_string(), PkgNamedAddr::Unset)]),
            BTreeMap::new(),
        );
        let mut mapping = BTreeMap::new();
        let err = collect_named_addresses(&pkg, &BTreeMap::new(), &mut mapping).unwrap_err();
        assert!(err.to_string().contains("named address not assigned"));
    }

    #[test]
    fn test_package_cache_slot_dir_changes_with_manifest_identity() {
        let cache_root = PathBuf::from("/tmp/package-cache");
        let first =
            package_cache_slot_dir(&cache_root, "Example", "/project/move/example/Move.toml");
        let second = package_cache_slot_dir(
            &cache_root,
            "Example",
            "/project-copy/move/example/Move.toml",
        );
        assert_ne!(first, second);
    }

    #[test]
    fn test_build_cache_fingerprint_tracks_dependency_fingerprints() -> Result<()> {
        let tmp = TempDir::new()?;
        let pkg_dir = tmp.path().join("example");
        fs::create_dir_all(pkg_dir.join("sources"))?;
        fs::write(
            pkg_dir.join("Move.toml"),
            "[package]\nname = \"Example\"\nversion = \"1.0.0\"\n",
        )?;
        fs::write(
            pkg_dir.join("sources").join("main.move"),
            "module 0x1::m {}",
        )?;

        let pkg = PkgManifest {
            name: "Example".to_string(),
            path: pkg_dir,
            version: (1, 0, 0).into(),
            deps: BTreeMap::new(),
            named_addresses: BTreeMap::new(),
        };
        let named_accounts = BTreeMap::new();
        let first = build_cache_fingerprint(
            &pkg,
            &named_accounts,
            "2.5".parse::<LanguageSetting>()?,
            false,
            &BTreeMap::from([("dep".to_string(), "a".to_string())]),
        )?;
        let second = build_cache_fingerprint(
            &pkg,
            &named_accounts,
            "2.5".parse::<LanguageSetting>()?,
            false,
            &BTreeMap::from([("dep".to_string(), "b".to_string())]),
        )?;
        assert_ne!(first, second);
        Ok(())
    }

    #[test]
    fn test_dependency_name_from_source_path_extracts_dependency_segment() {
        let dep_path =
            PathBuf::from("/tmp/build/example/sources/dependencies/AptosFramework/object.move");
        assert_eq!(
            dependency_name_from_source_path(&dep_path),
            Some("AptosFramework".to_string())
        );

        let root_path = PathBuf::from("/tmp/build/example/sources/root.move");
        assert_eq!(dependency_name_from_source_path(&root_path), None);
    }

    #[test]
    fn test_source_digest_for_build_tracks_source_changes() -> Result<()> {
        let tmp = TempDir::new()?;
        let pkg_dir = tmp.path().join("example");
        fs::create_dir_all(pkg_dir.join("sources"))?;
        fs::write(
            pkg_dir.join("Move.toml"),
            "[package]\nname = \"Example\"\nversion = \"1.0.0\"\n",
        )?;
        let source_path = pkg_dir.join("sources").join("main.move");
        fs::write(&source_path, "module 0x1::m { public fun a() {} }")?;

        let pkg = PkgManifest {
            name: "Example".to_string(),
            path: pkg_dir,
            version: (1, 0, 0).into(),
            deps: BTreeMap::new(),
            named_addresses: BTreeMap::new(),
        };
        let before = source_digest_for_build(&pkg, false)?;
        fs::write(&source_path, "module 0x1::m { public fun b() {} }")?;
        let after = source_digest_for_build(&pkg, false)?;
        assert_ne!(before, after);
        Ok(())
    }
}
