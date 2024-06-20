// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Utility functions for MoveSmith.
// TODO: consider move compiler/vm glue code to a separate file

use crate::{ast::CompileUnit, config::Config, move_smith::MoveSmith};
use arbitrary::{Result, Unstructured};
use log::{error, info};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::BuildConfig;
use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    error::Error,
    fs,
    fs::File,
    io::{stderr, Write},
    path::{Path, PathBuf},
};
use tempfile::{tempdir, TempDir};

const MOVE_TOML_TEMPLATE: &str = r#"[package]
name = "test"
version = "0.0.0"
"#;

/// Choose a random index based on the given probabilities.
/// e.g. if `weights` has [10, 20, 20], there are 3 options,
/// so this function will return 0, 1, or 2.
/// The probability for returning each element is based on the given weights.
// TODO: consider using `rand::distributions::WeightedIndex` for this.
// The current `int_in_range` doesn't seems to be evenly distributed.
// Concern is that the fuzzer will not be able to directly control the choice
pub fn choose_idx_weighted(u: &mut Unstructured, weights: &Vec<u32>) -> Result<usize> {
    assert!(!weights.is_empty());
    let sum = weights.iter().sum::<u32>();
    let thresholds = weights
        .iter()
        .scan(0.0f32, |acc, x| {
            *acc += *x as f32 / sum as f32;
            Some(*acc)
        })
        .collect::<Vec<f32>>();

    let choice = u.int_in_range(0..=100)? as f32 / 100.0;
    for (i, threshold) in thresholds.iter().enumerate() {
        if choice <= *threshold {
            return Ok(i);
        }
    }
    Ok(0)
}

/// Get random bytes
pub fn get_random_bytes(seed: u64, length: usize) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut buffer = vec![0u8; length];
    rng.fill(&mut buffer[..]);
    buffer
}

/// Turn raw bytes into a Move module.
/// This is useful to check the libfuzzer's corpus.
pub fn raw_to_compile_unit(data: &[u8]) -> Result<CompileUnit> {
    let mut u = Unstructured::new(data);
    let mut smith = MoveSmith::default();
    smith.generate(&mut u)?;
    Ok(smith.get_compile_unit())
}

/// Create a temporary Move file with the given code.
// TODO: if on Linux, we can create in-memory file to reduce I/O
fn create_tmp_move_file(code: String, name_hint: Option<&str>) -> (PathBuf, TempDir) {
    let dir: TempDir = tempdir().unwrap();
    let name = name_hint.unwrap_or("temp.move");
    let file_path = dir.path().join(name);
    {
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", code.as_str()).unwrap();
    }
    (file_path, dir)
}

/// Create a Move package with the given code and minimal Move.toml.
pub fn create_move_package(code: String, output_dir: &Path) {
    let source_dir = output_dir.join("sources");
    fs::create_dir_all(&source_dir).expect("Failed to create package directory");

    let move_toml_path = output_dir.join("Move.toml");
    fs::write(move_toml_path, MOVE_TOML_TEMPLATE).expect("Failed to write Move.toml");

    let move_path = source_dir.join("MoveSmith.move");
    fs::write(move_path, code).expect("Failed to write the Move file");
}

fn create_compiler_config_v1() -> BuildConfig {
    let mut config = BuildConfig::default();
    // config.force_recompilation = true;
    config.compiler_config.compiler_version = Some(CompilerVersion::V1);
    config
}

fn create_compiler_config_v2() -> BuildConfig {
    let mut config = BuildConfig::default();
    // config.force_recompilation = true;
    config.compiler_config.compiler_version = Some(CompilerVersion::V2_0);
    config.compiler_config.language_version = Some(LanguageVersion::V2_0);
    config
}

fn compile_with_config(package_path: &Path, config: BuildConfig, name: &str) -> bool {
    match config.compile_package_no_exit(package_path, &mut stderr()) {
        Ok(_) => {
            info!("Successfully compiled the package with compiler {}", name);
            true
        },
        Err(err) => {
            error!(
                "Failed to compile the package with compiler {}: {:?}",
                name, err
            );
            false
        },
    }
}

/// Create a temporary Move package with the given code.
pub fn create_tmp_move_package(code: String) -> (PathBuf, TempDir) {
    let dir: TempDir = tempdir().unwrap();
    let output_dir = dir.path().to_path_buf();
    create_move_package(code, &output_dir);
    (output_dir, dir)
}

/// Create a temporary package and compiler the given Move code.
/// V1 and V2 can be enabled/disabled separately.
pub fn compile_move_code(code: String, v1: bool, v2: bool) -> bool {
    let (package_path, dir) = create_tmp_move_package(code.clone());
    info!("created temp move package at {:?}", package_path);

    let v1_result = if v1 {
        let config = create_compiler_config_v1();
        compile_with_config(&package_path, config, "v1")
    } else {
        true
    };

    let v2_result = if v2 {
        let config = create_compiler_config_v2();
        compile_with_config(&package_path, config, "v2")
    } else {
        true
    };

    dir.close().unwrap();

    v1_result == v2_result
}

/// Runs the given Move code as a transactional test.
pub fn run_transactional_test(code: String, config: &Config) -> Result<(), Box<dyn Error>> {
    let (file_path, dir) = create_tmp_move_file(code, None);

    let ignores = config.known_error.clone();

    for (name, experiments) in config.experiment_combos.iter() {
        let result = run_transactional_test_with_experiments(&file_path, experiments);

        let processed_result = process_transactional_test_result(name, &ignores, result);
        match &processed_result {
            Ok(_) => {},
            Err(_) => return processed_result,
        };
    }

    dir.close().unwrap();
    Ok(())
}

fn run_transactional_test_with_experiments(
    file_path: &Path,
    experiments: &[(String, bool)],
) -> Result<(), Box<dyn Error>> {
    let vm_test_config = TestRunConfig::ComparisonV1V2 {
        language_version: LanguageVersion::V2_0,
        v2_experiments: experiments.to_owned(),
    };

    vm_test_harness::run_test_with_config_and_exp_suffix(vm_test_config, file_path, &None)
}

/// Filtering the error messages from the transactional test.
/// Currently only treat `error[Exxxx]` as a real error to ignore warnings.
fn process_transactional_test_result(
    name: &String,
    ignores: &[String],
    result: Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    if result.is_ok() {
        return Ok(());
    }
    let err = result.unwrap_err();
    let msg = format!("{:}", err);
    for ignore in ignores.iter() {
        if msg.contains(ignore) {
            return Ok(());
        }
    }
    if msg.contains("error[E") || msg.contains("error:") || msg.contains("bug") {
        // Raise an error with the name of the experiment
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("error with experiment: {:?}, {:?}", name, err),
        )))
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    const MOVE_CODE: &str = r#" //# publish
module 0xCAFE::Module1 {
    struct Struct3 has drop, copy {
        var32: u16,
        var33: u32,
        var34: u8,
        var35: u32,
        var36: u32,
    }

    public fun function6(): Struct3 {
        let var44: u16 =  21859u16;
        let var45: u32 =  1399722001u32;
        Struct3 {
            var32: var44,
            var33: var45,
            var34: 154u8,
            var35: var45,
            var36: var45,
        }
    }
}"#;

    const MOVE_CODE_V1_ERR: &str = r#" //# publish
module 0xCAFE::Module0 {
    public fun function0<T0: drop, T1: drop + store, T2: copy + drop + store> (var0: T2): T2 {
        if ((var0 == if (true)  { var0 } else { var0 }))  {
        } else {
            var0 = var0;
        };
        var0
    }
}"#;

    fn check_frequency(weights: &[u32], counts: &[u32], tolerance: f64) {
        let sum = weights.iter().sum::<u32>() as f64;
        let total_times = counts.iter().sum::<u32>() as f64;
        for idx in 0..weights.len() {
            let actual = counts[idx];
            let exp = (weights[idx] as f64 / sum) * total_times;
            let lower = (exp * (1.0 - tolerance)) as u32;
            let upper = (exp * (1.0 + tolerance)) as u32;
            let err_msg = format!(
                "Expecting the count for index {:?} to be in range [{:?}, {:?}], got {:?}",
                idx, lower, upper, actual
            );
            assert!(actual >= lower, "{}", err_msg);
            assert!(actual <= upper, "{}", err_msg);
        }
    }

    #[test]
    fn test_choose_idx_weighted() {
        let buffer = get_random_bytes(12345, 4096);
        let mut u = Unstructured::new(&buffer);

        let weights = vec![10, 20, 20];
        let mut counts = vec![0u32; weights.len()];

        let total_times = 1000;
        for _ in 0..total_times {
            let idx = choose_idx_weighted(&mut u, &weights).unwrap();
            counts[idx] += 1;
        }
        assert!(counts[0] < counts[1]);
        assert!(counts[0] < counts[2]);
        check_frequency(&weights, &counts, 0.25);
    }

    #[test]
    fn test_choose_idx_zero_weighted() {
        let buffer = get_random_bytes(12345, 4096);
        let mut u = Unstructured::new(&buffer);

        let weights = vec![30, 0, 20];
        let mut counts = vec![0; weights.len()];

        let total_times = 1000;
        for _ in 0..total_times {
            let idx = choose_idx_weighted(&mut u, &weights).unwrap();
            counts[idx] += 1;
        }
        assert_eq!(counts[1], 0);
        check_frequency(&weights, &counts, 0.25);
    }

    #[test]
    fn test_compile() {
        let code = MOVE_CODE.to_string();
        let result = compile_move_code(code, true, true);
        assert!(result);
    }

    #[test]
    fn test_compile_err() {
        let code = MOVE_CODE_V1_ERR.to_string();

        // Should not compile with V1
        let result = panic::catch_unwind(|| compile_move_code(code.clone(), true, true));
        assert!(result.is_err());
    }
}
