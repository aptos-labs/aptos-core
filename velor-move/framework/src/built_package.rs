// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    docgen::DocgenOptions,
    extended_checks,
    natives::code::{ModuleMetadata, MoveOption, PackageDep, PackageMetadata, UpgradePolicy},
    zip_metadata, zip_metadata_str,
};
use anyhow::bail;
use velor_types::{
    account_address::AccountAddress,
    transaction::EntryABI,
    vm::module_metadata::{
        RuntimeModuleMetadataV1, VELOR_METADATA_KEY, VELOR_METADATA_KEY_V1,
        METADATA_V1_MIN_FILE_FORMAT_VERSION,
    },
};
use clap::Parser;
use codespan_reporting::{
    diagnostic::Severity,
    term::termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor},
};
use itertools::Itertools;
use legacy_move_compiler::{
    compiled_unit::{CompiledUnit, NamedCompiledModule},
    shared::NumericalAddress,
};
use move_binary_format::{file_format_common, file_format_common::VERSION_DEFAULT, CompiledModule};
use move_command_line_common::files::MOVE_COMPILED_EXTENSION;
use move_compiler_v2::{external_checks::ExternalChecks, options::Options, Experiment};
use move_core_types::{language_storage::ModuleId, metadata::Metadata};
use move_model::{
    metadata::{CompilerVersion, LanguageVersion},
    model::GlobalEnv,
};
use move_package::{
    compilation::{compiled_package::CompiledPackage, package_layout::CompiledPackageLayout},
    resolution::resolution_graph::ResolvedGraph,
    source_package::{
        manifest_parser::{parse_move_manifest_string, parse_source_manifest},
        std_lib::StdVersion,
    },
    BuildConfig, CompilerConfig, ModelConfig,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{stderr, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

pub const METADATA_FILE_NAME: &str = "package-metadata.bcs";
pub const UPGRADE_POLICY_CUSTOM_FIELD: &str = "upgrade_policy";

pub const VELOR_PACKAGES: [&str; 6] = [
    "VelorFramework",
    "MoveStdlib",
    "VelorStdlib",
    "VelorToken",
    "VelorTokenObjects",
    "VelorExperimental",
];

/// Represents a set of options for building artifacts from Move.
#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct BuildOptions {
    /// Enables dev mode, which uses all dev-addresses and dev-dependencies
    ///
    /// Dev mode allows for changing dependencies and addresses to the preset [dev-addresses] and
    /// [dev-dependencies] fields.  This works both inside and out of tests for using preset values.
    ///
    /// Currently, it also additionally pulls in all test compilation artifacts
    #[clap(long)]
    pub dev: bool,
    #[clap(long)]
    pub with_srcs: bool,
    #[clap(long)]
    pub with_abis: bool,
    #[clap(long)]
    pub with_source_maps: bool,
    #[clap(long, default_value_t = true)]
    pub with_error_map: bool,
    #[clap(long)]
    pub with_docs: bool,
    /// Installation directory for compiled artifacts. Defaults to `<package>/build`.
    #[clap(long, value_parser)]
    pub install_dir: Option<PathBuf>,
    #[clap(skip)] // TODO: have a parser for this; there is one in the CLI buts its  downstream
    pub named_addresses: BTreeMap<String, AccountAddress>,
    /// Whether to override the standard library with the given version.
    #[clap(long, value_parser)]
    pub override_std: Option<StdVersion>,
    #[clap(skip)]
    pub docgen_options: Option<DocgenOptions>,
    #[clap(long)]
    pub skip_fetch_latest_git_deps: bool,
    #[clap(long)]
    pub bytecode_version: Option<u32>,
    #[clap(long, value_parser = clap::value_parser!(CompilerVersion))]
    pub compiler_version: Option<CompilerVersion>,
    #[clap(long, value_parser = clap::value_parser!(LanguageVersion))]
    pub language_version: Option<LanguageVersion>,
    #[clap(long)]
    pub skip_attribute_checks: bool,
    #[clap(long)]
    pub check_test_code: bool,
    #[clap(skip)]
    pub known_attributes: BTreeSet<String>,
    #[clap(skip)]
    pub experiments: Vec<String>,
}

// Because named_addresses has no parser, we can't use clap's default impl. This must be aligned
// with defaults above.
impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            dev: false,
            with_srcs: false,
            with_abis: false,
            with_source_maps: false,
            with_error_map: true,
            with_docs: false,
            install_dir: None,
            named_addresses: Default::default(),
            override_std: None,
            docgen_options: None,
            // This is false by default, because it could accidentally pull new dependencies
            // while in a test (and cause some havoc)
            skip_fetch_latest_git_deps: false,
            bytecode_version: None,
            compiler_version: None,
            language_version: None,
            skip_attribute_checks: false,
            check_test_code: true,
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            experiments: vec![],
        }
    }
}

impl BuildOptions {
    pub fn move_2() -> Self {
        BuildOptions {
            bytecode_version: Some(VERSION_DEFAULT),
            language_version: Some(LanguageVersion::latest_stable()),
            compiler_version: Some(CompilerVersion::latest_stable()),
            ..Self::default()
        }
    }

    pub fn inferred_bytecode_version(&self) -> u32 {
        self.language_version
            .unwrap_or_default()
            .infer_bytecode_version(self.bytecode_version)
    }

    pub fn with_experiment(mut self, exp: &str) -> Self {
        self.experiments.push(exp.to_string());
        self
    }

    pub fn set_latest_language(self) -> Self {
        BuildOptions {
            language_version: Some(LanguageVersion::latest()),
            bytecode_version: Some(file_format_common::VERSION_MAX),
            ..self
        }
    }
}

/// Represents a built package.  It allows to extract `PackageMetadata`. Can also be used to
/// just build Move code and related artifacts.
pub struct BuiltPackage {
    options: BuildOptions,
    package_path: PathBuf,
    pub package: CompiledPackage,
}

pub fn build_model(
    dev_mode: bool,
    package_path: &Path,
    additional_named_addresses: BTreeMap<String, AccountAddress>,
    target_filter: Option<String>,
    bytecode_version: Option<u32>,
    compiler_version: Option<CompilerVersion>,
    language_version: Option<LanguageVersion>,
    skip_attribute_checks: bool,
    known_attributes: BTreeSet<String>,
    experiments: Vec<String>,
) -> anyhow::Result<GlobalEnv> {
    let bytecode_version = Some(
        language_version
            .unwrap_or_default()
            .infer_bytecode_version(bytecode_version),
    );
    let build_config = BuildConfig {
        dev_mode,
        additional_named_addresses,
        generate_abis: false,
        generate_docs: false,
        generate_move_model: false,
        full_model_generation: false,
        install_dir: None,
        test_mode: false,
        override_std: None,
        force_recompilation: false,
        fetch_deps_only: false,
        skip_fetch_latest_git_deps: true,
        compiler_config: CompilerConfig {
            bytecode_version,
            compiler_version,
            language_version,
            skip_attribute_checks,
            known_attributes,
            experiments,
        },
    };
    let compiler_version = compiler_version.unwrap_or_default();
    let language_version = language_version.unwrap_or_default();
    compiler_version.check_language_support(language_version)?;
    build_config.move_model_for_package(package_path, ModelConfig {
        target_filter,
        all_files_as_targets: false,
        compiler_version,
        language_version,
    })
}

impl BuiltPackage {
    /// Builds the package and on success delivers a `BuiltPackage`.
    ///
    /// This function currently reports all Move compilation errors and warnings to stdout,
    /// and is not `Ok` if there was an error among those.
    pub fn build(package_path: PathBuf, options: BuildOptions) -> anyhow::Result<Self> {
        let build_config = Self::create_build_config(&options)?;
        let resolved_graph = Self::prepare_resolution_graph(package_path, build_config.clone())?;
        BuiltPackage::build_with_external_checks(resolved_graph, options, build_config, vec![])
    }

    pub fn create_build_config(options: &BuildOptions) -> anyhow::Result<BuildConfig> {
        let bytecode_version = Some(options.inferred_bytecode_version());
        let compiler_version = options.compiler_version;
        let language_version = options.language_version;
        Self::check_versions(&compiler_version, &language_version)?;
        let skip_attribute_checks = options.skip_attribute_checks;
        Ok(BuildConfig {
            dev_mode: options.dev,
            additional_named_addresses: options.named_addresses.clone(),
            generate_abis: options.with_abis,
            generate_docs: false,
            generate_move_model: true,
            full_model_generation: options.check_test_code,
            install_dir: options.install_dir.clone(),
            test_mode: false,
            override_std: options.override_std.clone(),
            force_recompilation: false,
            fetch_deps_only: false,
            skip_fetch_latest_git_deps: options.skip_fetch_latest_git_deps,
            compiler_config: CompilerConfig {
                bytecode_version,
                compiler_version,
                language_version,
                skip_attribute_checks,
                known_attributes: options.known_attributes.clone(),
                experiments: options.experiments.clone(),
            },
        })
    }

    pub fn prepare_resolution_graph(
        package_path: PathBuf,
        build_config: BuildConfig,
    ) -> anyhow::Result<ResolvedGraph> {
        eprintln!("Compiling, may take a little while to download git dependencies...");
        build_config.resolution_graph_for_package(&package_path, &mut stderr())
    }

    /// Same as `build` but allows to provide external checks to be made on Move code.
    /// The `external_checks` are only run when compiler v2 is used.
    pub fn build_with_external_checks(
        resolved_graph: ResolvedGraph,
        options: BuildOptions,
        build_config: BuildConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
    ) -> anyhow::Result<Self> {
        {
            let package_path = resolved_graph.root_package_path.clone();
            let bytecode_version = build_config.compiler_config.bytecode_version;

            let (mut package, model_opt) = build_config.compile_package_no_exit(
                resolved_graph,
                external_checks,
                &mut stderr(),
            )?;

            // Run extended checks as well derive runtime metadata
            let model = &model_opt.expect("move model");

            if let Some(model_options) = model.get_extension::<Options>() {
                if model_options.experiment_on(Experiment::STOP_BEFORE_EXTENDED_CHECKS) {
                    std::process::exit(if model.has_warnings() { 1 } else { 0 })
                }
            }

            let runtime_metadata = extended_checks::run_extended_checks(model);
            if model.diag_count(Severity::Warning) > 0
                && !model
                    .get_extension::<Options>()
                    .is_some_and(|model_options| {
                        model_options.experiment_on(Experiment::SKIP_BAILOUT_ON_EXTENDED_CHECKS)
                    })
            {
                let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
                model.report_diag(&mut error_writer, Severity::Warning);
                if model.has_errors() {
                    bail!("extended checks failed")
                }
            }

            if let Some(model_options) = model.get_extension::<Options>() {
                if model_options.experiment_on(Experiment::FAIL_ON_WARNING) && model.has_warnings()
                {
                    bail!("found warning(s), and `--fail-on-warning` is set")
                } else if model_options.experiment_on(Experiment::STOP_AFTER_EXTENDED_CHECKS) {
                    std::process::exit(if model.has_warnings() { 1 } else { 0 })
                }
            }

            let compiled_pkg_path = package
                .compiled_package_info
                .build_flags
                .install_dir
                .as_ref()
                .unwrap_or(&package_path)
                .join(CompiledPackageLayout::Root.path())
                .join(package.compiled_package_info.package_name.as_str());
            inject_runtime_metadata(
                compiled_pkg_path,
                &mut package,
                runtime_metadata,
                bytecode_version,
            )?;

            // If enabled generate docs.
            if options.with_docs {
                let docgen = options.docgen_options.clone().unwrap_or_default();
                let dep_paths = package
                    .deps_compiled_units
                    .iter()
                    .map(|(_, u)| {
                        u.source_path
                            .parent()
                            .unwrap()
                            .parent()
                            .unwrap()
                            .join("doc")
                            .display()
                            .to_string()
                    })
                    .unique()
                    .collect::<Vec<_>>();
                docgen.run(package_path.display().to_string(), dep_paths, model)?
            }

            Ok(Self {
                options,
                package_path,
                package,
            })
        }
    }

    // Check versions and warn user if using unstable ones.
    fn check_versions(
        compiler_version: &Option<CompilerVersion>,
        language_version: &Option<LanguageVersion>,
    ) -> anyhow::Result<()> {
        let effective_compiler_version = compiler_version.unwrap_or_default();
        let effective_language_version = language_version.unwrap_or_default();
        let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
        if effective_compiler_version.unstable() {
            error_writer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            writeln!(
                &mut error_writer,
                "Warning: compiler version `{}` is experimental \
                and should not be used in production",
                effective_compiler_version
            )?;
            error_writer.reset()?;
        }
        if effective_language_version.unstable() {
            error_writer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            writeln!(
                &mut error_writer,
                "Warning: language version `{}` is experimental \
                and should not be used in production",
                effective_language_version
            )?;
            error_writer.reset()?;
        }
        effective_compiler_version.check_language_support(effective_language_version)?;
        Ok(())
    }

    /// Returns the name of this package.
    pub fn name(&self) -> &str {
        self.package.compiled_package_info.package_name.as_str()
    }

    pub fn package_path(&self) -> &Path {
        self.package_path.as_path()
    }

    pub fn package_artifacts_path(&self) -> PathBuf {
        self.package_path
            .join(CompiledPackageLayout::Root.path())
            .join(self.name())
    }

    /// Extracts the bytecode for the modules of the built package.
    pub fn extract_code(&self) -> Vec<Vec<u8>> {
        self.package
            .root_modules()
            .map(|unit_with_source| {
                let bytecode_version = self.options.inferred_bytecode_version();
                unit_with_source.unit.serialize(Some(bytecode_version))
            })
            .collect()
    }

    /// Returns an iterator over the bytecode for the modules of the built package, along with the
    /// module names.
    pub fn module_code_iter<'a>(&'a self) -> impl Iterator<Item = (String, Vec<u8>)> + 'a {
        self.package.root_modules().map(|unit_with_source| {
            let bytecode_version = self.options.inferred_bytecode_version();
            let code = unit_with_source.unit.serialize(Some(bytecode_version));
            (unit_with_source.unit.name().as_str().to_string(), code)
        })
    }

    /// Returns the abis for this package, if available.
    pub fn extract_abis(&self) -> Option<Vec<EntryABI>> {
        self.package.compiled_abis.as_ref().map(|abis| {
            abis.iter()
                .map(|(_, bytes)| bcs::from_bytes::<EntryABI>(bytes.as_slice()).unwrap())
                .collect()
        })
    }

    /// Returns an iterator for all compiled proper (non-script) modules.
    pub fn modules(&self) -> impl Iterator<Item = &CompiledModule> {
        self.package
            .root_modules()
            .filter_map(|unit| match &unit.unit {
                CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module),
                CompiledUnit::Script(_) => None,
            })
    }

    /// Returns an iterator for all compiled proper (non-script) modules, including
    /// modules that are dependencies of the root modules.
    pub fn all_modules(&self) -> impl Iterator<Item = &CompiledModule> {
        self.package
            .all_modules()
            .filter_map(|unit| match &unit.unit {
                CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module),
                CompiledUnit::Script(_) => None,
            })
    }

    /// Replaces a module by name with a new CompiledModule instance
    #[cfg(feature = "testing")]
    pub fn replace_module(
        &mut self,
        module_name: &str,
        new_module: CompiledModule,
    ) -> anyhow::Result<()> {
        for unit_with_source in &mut self.package.root_compiled_units {
            if let CompiledUnit::Module(named_module) = &mut unit_with_source.unit {
                if named_module.name.as_str() == module_name {
                    named_module.module = new_module;
                    return Ok(());
                }
            }
        }

        Err(anyhow::anyhow!(
            "Module '{}' not found in package",
            module_name
        ))
    }

    /// Returns the number of scripts in the package.
    pub fn script_count(&self) -> usize {
        self.package.scripts().count()
    }

    /// Returns the serialized bytecode of the scripts in the package.
    pub fn extract_script_code(&self) -> Vec<Vec<u8>> {
        self.package
            .scripts()
            .map(|unit_with_source| {
                unit_with_source
                    .unit
                    .serialize(Some(self.options.inferred_bytecode_version()))
            })
            .collect()
    }

    /// Extracts metadata, as needed for releasing a package, from the built package.
    pub fn extract_metadata(&self) -> anyhow::Result<PackageMetadata> {
        let source_digest = self
            .package
            .compiled_package_info
            .source_digest
            .map(|s| s.to_string())
            .unwrap_or_default();
        let manifest_file = self.package_path.join("Move.toml");
        let manifest = std::fs::read_to_string(manifest_file)?;
        let custom_props = extract_custom_fields(&manifest)?;
        let manifest = zip_metadata_str(&manifest)?;
        let upgrade_policy = if let Some(val) = custom_props.get(UPGRADE_POLICY_CUSTOM_FIELD) {
            str::parse::<UpgradePolicy>(val.as_ref())?
        } else {
            UpgradePolicy::compat()
        };
        let mut modules = vec![];
        for u in self.package.root_modules() {
            let name = u.unit.name().to_string();
            let source = if self.options.with_srcs {
                zip_metadata_str(&std::fs::read_to_string(&u.source_path)?)?
            } else {
                vec![]
            };
            let source_map = if self.options.with_source_maps {
                zip_metadata(&u.unit.serialize_source_map())?
            } else {
                vec![]
            };
            modules.push(ModuleMetadata {
                name,
                source,
                source_map,
                extension: MoveOption::default(),
            })
        }
        let deps = self
            .package
            .deps_compiled_units
            .iter()
            .flat_map(|(name, unit)| match &unit.unit {
                CompiledUnit::Module(m) => {
                    let package_name = name.as_str().to_string();
                    let account = AccountAddress::new(m.address.into_bytes());

                    Some(PackageDep {
                        account,
                        package_name,
                    })
                },
                CompiledUnit::Script(_) => None,
            })
            .chain(
                self.package
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
            name: self.name().to_string(),
            upgrade_policy,
            upgrade_number: 0,
            source_digest,
            manifest,
            modules,
            deps,
            extension: MoveOption::none(),
        })
    }

    pub fn extract_metadata_and_save(&self) -> anyhow::Result<()> {
        let data = self.extract_metadata()?;
        let path = self.package_artifacts_path();
        std::fs::create_dir_all(&path)?;
        std::fs::write(path.join(METADATA_FILE_NAME), bcs::to_bytes(&data)?)?;
        Ok(())
    }
}

fn extract_custom_fields(toml: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let manifest = parse_source_manifest(parse_move_manifest_string(toml.to_owned())?)?;
    Ok(manifest
        .package
        .custom_properties
        .iter()
        .map(|(s, v)| (s.to_string(), v.to_string()))
        .collect())
}

fn inject_runtime_metadata(
    package_path: PathBuf,
    pack: &mut CompiledPackage,
    metadata: BTreeMap<ModuleId, RuntimeModuleMetadataV1>,
    bytecode_version: Option<u32>,
) -> anyhow::Result<()> {
    for unit_with_source in pack.root_compiled_units.iter_mut() {
        match &mut unit_with_source.unit {
            CompiledUnit::Module(named_module) => {
                if let Some(module_metadata) = metadata.get(&named_module.module.self_id()) {
                    if !module_metadata.is_empty() {
                        if bytecode_version.unwrap_or(METADATA_V1_MIN_FILE_FORMAT_VERSION)
                            >= METADATA_V1_MIN_FILE_FORMAT_VERSION
                        {
                            let serialized_metadata = bcs::to_bytes(&module_metadata)
                                .expect("BCS for RuntimeModuleMetadata");
                            named_module.module.metadata.push(Metadata {
                                key: VELOR_METADATA_KEY_V1.to_vec(),
                                value: serialized_metadata,
                            });
                        } else {
                            let serialized_metadata =
                                bcs::to_bytes(&module_metadata.clone().downgrade())
                                    .expect("BCS for RuntimeModuleMetadata");
                            named_module.module.metadata.push(Metadata {
                                key: VELOR_METADATA_KEY.to_vec(),
                                value: serialized_metadata,
                            });
                        }

                        // Also need to update the .mv file on disk.
                        let path = package_path
                            .join(CompiledPackageLayout::CompiledModules.path())
                            .join(named_module.name.as_str())
                            .with_extension(MOVE_COMPILED_EXTENSION);
                        if path.is_file() {
                            let bytes = unit_with_source.unit.serialize(bytecode_version);
                            std::fs::write(path, bytes)?;
                        }
                    }
                }
            },
            CompiledUnit::Script(_) => {},
        }
    }
    Ok(())
}
