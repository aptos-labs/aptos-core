// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::term::termcolor::Buffer;
#[allow(unused_imports)]
use log::debug;
use move_abigen::{Abigen, AbigenOptions};
use move_prover_test_utils::baseline_test::verify_or_update_baseline;
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

const STDLIB_DEPENDENCY: &str = "../../move-stdlib/sources";
const NAMED_ADDRESSES: &str = "std=0x1";

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let options = AbigenOptions {
        compiled_script_directory: "tests/sources".to_string(),
        ..AbigenOptions::default()
    };
    test_abigen(path, options, "exp")?;
    Ok(())
}

fn get_abi_paths_under_dir(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut abi_paths = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                abi_paths.append(&mut get_abi_paths_under_dir(&path)?);
            } else if let Some("abi") = path.extension().and_then(OsStr::to_str) {
                abi_paths.push(path.to_str().unwrap().to_string());
            }
        }
    }
    Ok(abi_paths)
}

fn test_abigen(path: &Path, mut options: AbigenOptions, suffix: &str) -> anyhow::Result<()> {
    let temp_path = PathBuf::from(TempDir::new()?.path());
    options.output_directory = temp_path.to_string_lossy().to_string();
    let output_dir = path.with_extension("");
    let mut expected_apis = match get_abi_paths_under_dir(&output_dir) {
        Ok(found) => found.iter().map(PathBuf::from).collect::<BTreeSet<_>>(),
        _ => BTreeSet::new(),
    };

    let compiler_options = move_compiler_v2::Options {
        sources: vec![path.to_string_lossy().to_string()],
        dependencies: vec![STDLIB_DEPENDENCY.to_string()],
        named_address_mapping: vec![NAMED_ADDRESSES.to_string()],
        skip_attribute_checks: true,
        compile_verify_code: true,
        ..Default::default()
    };

    let mut error_writer = Buffer::no_color();
    match move_compiler_v2::run_move_compiler_for_analysis(&mut error_writer, compiler_options) {
        Ok(model) => {
            let mut generator = Abigen::new(&model, &options);
            generator.r#gen();
            for (file, content) in generator.into_result() {
                let abi_path = PathBuf::from(&file);
                fs::create_dir_all(abi_path.parent().unwrap())?;
                fs::write(abi_path.as_path(), &content)?;
            }
            for abi_path in get_abi_paths_under_dir(&temp_path)?.iter() {
                let mut contents = String::new();
                if let Ok(mut file) = File::open(abi_path) {
                    file.read_to_string(&mut contents).unwrap();
                }
                let buf = PathBuf::from(abi_path);
                let mut baseline_file_name = PathBuf::from(path);
                baseline_file_name.pop();
                baseline_file_name.push(buf.strip_prefix(&temp_path)?);
                verify_or_update_baseline(&baseline_file_name, &contents)?;
                expected_apis.remove(&baseline_file_name);
            }
            let empty_contents = "".to_string();
            for baseline_file_name in expected_apis.iter() {
                verify_or_update_baseline(Path::new(baseline_file_name), &empty_contents)?;
            }
            let buffer = error_writer.into_inner();
            let contents = String::from_utf8_lossy(&buffer);
            let baseline_path = path.with_extension(suffix);
            verify_or_update_baseline(&baseline_path, &contents)?;
        },
        Err(err) => {
            let mut contents = format!("Abigen returns: {}\n", err);
            contents += &String::from_utf8_lossy(&error_writer.into_inner());
            let baseline_path = path.with_extension(suffix);
            verify_or_update_baseline(&baseline_path, &contents)?;
        },
    };
    Ok(())
}

datatest_stable::harness!(test_runner, "tests/sources", r".*\.move$",);
