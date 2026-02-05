// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Baseline tests for move-query.
//!
//! Each test package directory under `test_packages/` is a test case.
//! The test queries all items from the model and compares output against
//! the `{package_name}.exp` baseline file.

use move_command_line_common::testing::{
    add_update_baseline_fix, format_diff, read_env_update_baseline,
};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_query::{BuildConfig, ModelConfig, QueryEngine};
use serde::Serialize;
use std::{fs, path::Path};

fn run_test(path: &Path) -> datatest_stable::Result<()> {
    let package_dir = path.parent().unwrap();
    let model_config = ModelConfig {
        all_files_as_targets: false,
        target_filter: None,
        compiler_version: CompilerVersion::default(),
        language_version: LanguageVersion::default(),
    };
    let build_config = BuildConfig {
        test_mode: true, // Include test functions to cover attribute formatting
        compiler_config: move_package::CompilerConfig {
            skip_attribute_checks: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut engine = QueryEngine::new(package_dir.to_path_buf(), build_config, model_config)?;

    // Verify rebuild works without error (model should remain valid after rebuild)
    engine.rebuild()?;

    let mut output = String::new();

    // Query and print package info
    let package = engine.get_package()?;
    write_section("Package", &mut output);
    write_json(&package, &mut output);

    // Query and print full info for each target module
    for module_name in &package.modules {
        output_module_info(&engine, module_name, &mut output, false)?;
    }

    // Query and print full info for each dependency module
    for dep in &package.dependencies {
        for module_name in &dep.modules {
            output_module_info(&engine, module_name, &mut output, true)?;
        }
    }

    // Compare against baseline
    let package_name = package_dir.file_name().unwrap().to_str().unwrap();
    let exp_path = package_dir.join(format!("{}.exp", package_name));
    check_or_update(&exp_path, output, read_env_update_baseline())
}

fn output_module_info(
    engine: &QueryEngine,
    module_name: &str,
    output: &mut String,
    is_dependency: bool,
) -> datatest_stable::Result<()> {
    let module = engine.get_module(module_name)?;
    let module_label = format!(
        "{}{}",
        module_name,
        if is_dependency { " (dependency)" } else { "" }
    );
    write_section(&format!("Module: {}", module_label), output);
    write_json(&module, output);

    // Query and print full info for each struct
    for struct_name in &module.structs {
        let full_name = format!("{}::{}", module_name, struct_name);
        let struct_info = engine.get_struct(&full_name)?;
        write_section(
            &format!("Struct: {}::{}", module_label, struct_name),
            output,
        );
        write_json(&struct_info, output);
    }

    // Query and print full info for each constant
    for const_name in &module.constants {
        let full_name = format!("{}::{}", module_name, const_name);
        let const_info = engine.get_constant(&full_name)?;
        write_section(
            &format!("Constant: {}::{}", module_label, const_name),
            output,
        );
        write_json(&const_info, output);
    }

    // Query and print full info for each function, keeping first for get_source demo
    let mut first_func: Option<move_query::Function> = None;
    for func_name in &module.functions {
        let full_name = format!("{}::{}", module_name, func_name);
        let func_info = engine.get_function(&full_name)?;
        write_section(
            &format!("Function: {}::{}", module_label, func_name),
            output,
        );
        write_json(&func_info, output);
        if first_func.is_none() {
            first_func = Some(func_info);
        }
    }

    // Demo get_source: show source for the first function (if any)
    if !is_dependency {
        if let Some(func_info) = first_func {
            let source = engine.get_source(&func_info.location)?;
            write_section(
                &format!("Source: {}::{}", module_label, func_info.name),
                output,
            );
            output.push_str(&source);
            output.push_str("\n\n");
        }
    }

    Ok(())
}

fn write_section(label: &str, output: &mut String) {
    output.push_str(&format!("// === {} ===\n", label));
}

fn write_json<T: Serialize>(value: &T, output: &mut String) {
    output.push_str(&serde_json::to_string_pretty(value).unwrap());
    output.push_str("\n\n");
}

fn check_or_update(
    exp_path: &Path,
    output: String,
    update_baseline: bool,
) -> datatest_stable::Result<()> {
    if update_baseline {
        fs::write(exp_path, &output)?;
        return Ok(());
    }

    if exp_path.is_file() {
        let expected = fs::read_to_string(exp_path)?;
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
            "No expected output found for {:?}.\n\
             Run with `env UPDATE_BASELINE=1` to create it.",
            exp_path
        )
        .into());
    }
    Ok(())
}

datatest_stable::harness!(run_test, "tests/test_packages", r"Move\.toml$");
