// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use move_command_line_common::testing::{
    add_update_baseline_fix, format_diff, read_env_update_baseline, EXP_EXT, EXP_EXT_V2,
};
use move_compiler::shared::known_attributes::KnownAttribute;
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::{
    compilation::{build_plan::BuildPlan, model_builder::ModelBuilder},
    package_hooks::{self, PackageHooks},
    resolution::resolution_graph as RG,
    source_package::{
        manifest_parser as MP,
        parsed_manifest::{CustomDepInfo, PackageDigest},
        std_lib::StdVersion,
    },
    BuildConfig, CompilerConfig, ModelConfig,
};
use move_symbol_pool::Symbol;
use std::{
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
};
use tempfile::tempdir;

const COMPILE_EXT: &str = "compile";
const MODEL_EXT: &str = "model";
const OVERRIDE_EXT: &str = "override";

fn run_test_impl(
    path: &Path,
    compiler_version: CompilerVersion,
) -> datatest_stable::Result<String> {
    let mut compiler_config = CompilerConfig {
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        ..Default::default()
    };
    compiler_config.compiler_version = Some(compiler_version);
    let override_path = path.with_extension(OVERRIDE_EXT);
    let override_std = if override_path.is_file() {
        Some(
            StdVersion::from_rev(&fs::read_to_string(override_path)?)
                .expect("one of mainnet/testnet/devnet"),
        )
    } else {
        None
    };
    let should_compile = path.with_extension(COMPILE_EXT).is_file();
    let should_model = path.with_extension(MODEL_EXT).is_file();
    let contents = fs::read_to_string(path)?;
    let output = match MP::parse_move_manifest_string(contents)
        .and_then(MP::parse_source_manifest)
        .and_then(|parsed_manifest| {
            RG::ResolutionGraph::new(
                parsed_manifest,
                path.parent().unwrap().to_path_buf(),
                BuildConfig {
                    dev_mode: true,
                    test_mode: false,
                    override_std,
                    generate_docs: false,
                    generate_abis: false,
                    install_dir: Some(tempdir().unwrap().path().to_path_buf()),
                    force_recompilation: false,
                    compiler_config: compiler_config.clone(),
                    ..Default::default()
                },
                &mut Vec::new(), /* empty writer as no diags needed */
            )
        })
        .and_then(|rg| rg.resolve())
    {
        Ok(mut resolved_package) => match (should_compile, should_model) {
            (true, true) => {
                return Err(anyhow::format_err!(
                    "Cannot have compile and model flags set for same package"
                )
                .into())
            },
            (true, _) => match BuildPlan::create(resolved_package)
                .and_then(|bp| bp.compile_no_exit(&compiler_config.clone(), &mut Vec::new()))
            {
                Ok((mut pkg, _)) => {
                    pkg.compiled_package_info.source_digest =
                        Some(PackageDigest::from("ELIDED_FOR_TEST"));
                    pkg.compiled_package_info.build_flags.install_dir =
                        Some(PathBuf::from("ELIDED_FOR_TEST"));
                    format!("{:#?}\n", pkg.compiled_package_info)
                },
                Err(error) => format!("{:#}\n", error),
            },
            (_, true) => match ModelBuilder::create(resolved_package, ModelConfig {
                all_files_as_targets: false,
                target_filter: None,
                compiler_version,
                language_version: LanguageVersion::default(),
            })
            .build_model()
            {
                Ok(_) => "Built model".to_string(),
                Err(error) => format!("{:#}\n", error),
            },
            (_, _) => {
                for (_, package) in resolved_package.package_table.iter_mut() {
                    package.package_path = PathBuf::from("ELIDED_FOR_TEST");
                    package.source_digest = PackageDigest::from("ELIDED_FOR_TEST");
                }
                resolved_package.build_options.install_dir = Some(PathBuf::from("ELIDED_FOR_TEST"));
                format!("{:#?}\n", resolved_package)
            },
        },
        Err(error) => format!("{:#}\n", error),
    };
    Ok(output)
}

fn check_or_update(
    path: &Path,
    output: String,
    update_baseline: bool,
    compiler_version: CompilerVersion,
) -> datatest_stable::Result<()> {
    let exp_ext = if compiler_version == CompilerVersion::V2_0 {
        EXP_EXT_V2
    } else {
        EXP_EXT
    };
    let exp_path = path.with_extension(exp_ext);
    let exp_exists = exp_path.is_file();
    if update_baseline {
        fs::write(&exp_path, &output)?;
        return Ok(());
    }

    if exp_exists {
        let expected = fs::read_to_string(&exp_path)?;
        if expected != output {
            let msg = format!(
                "Expected outputs differ for {:?}:\n{}",
                exp_path,
                format_diff(expected, output)
            );
            return Err(anyhow::format_err!(add_update_baseline_fix(msg)).into());
        }
    } else {
        return Err(anyhow::format_err!(
            "No expected output found for {:?}.\
                    You probably want to rerun with `env UPDATE_BASELINE=1`",
            path
        )
        .into());
    }
    Ok(())
}

pub fn run_test(path: &Path) -> datatest_stable::Result<()> {
    package_hooks::register_package_hooks(Box::new(TestHooks()));
    if path
        .components()
        .any(|component| component == Component::Normal(OsStr::new("deps_only")))
    {
        return Ok(());
    }

    let output_v1 = run_test_impl(path, CompilerVersion::default())?;
    let update_baseline = read_env_update_baseline();
    check_or_update(
        path,
        output_v1.clone(),
        update_baseline,
        CompilerVersion::default(),
    )
}

/// Some dummy hooks for testing the hook mechanism
struct TestHooks();

impl PackageHooks for TestHooks {
    fn custom_package_info_fields(&self) -> Vec<String> {
        vec!["test_hooks_field".to_owned()]
    }

    fn custom_dependency_key(&self) -> Option<String> {
        Some("custom".to_owned())
    }

    fn resolve_custom_dependency(
        &self,
        dep_name: Symbol,
        info: &CustomDepInfo,
    ) -> anyhow::Result<()> {
        bail!(
            "TestHooks resolve dep {} = {} {} {}",
            dep_name,
            info.node_url,
            info.package_name,
            info.package_address
        )
    }
}

datatest_stable::harness!(run_test, "tests/test_sources", r".*\.toml$");
