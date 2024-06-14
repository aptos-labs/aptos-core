// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Utility functions for MoveSmith.
// TODO: consider move compiler/vm glue code to a separate file

use crate::{ast::CompileUnit, config::Config, move_smith::MoveSmith};
use arbitrary::{Result, Unstructured};
use move_compiler::{
    shared::{known_attributes::KnownAttribute, Flags},
    Compiler as MoveCompiler,
};
use move_compiler_v2::Experiment;
use move_model::metadata::LanguageVersion;
use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{error::Error, fs::File, io::Write, path::PathBuf};
use tempfile::{tempdir, TempDir};

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
    let dir = tempdir().unwrap();
    let name = name_hint.unwrap_or("temp.move");
    let file_path = dir.path().join(name);
    {
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", code.as_str()).unwrap();
    }
    (file_path, dir)
}

/// Compiles the given Move code using compiler v1.
pub fn compile_modules(code: String) {
    let (file_path, dir) = create_tmp_move_file(code, None);
    let (_, _units) = MoveCompiler::from_files(
        vec![file_path.to_str().unwrap().to_string()],
        vec![],
        move_stdlib::move_stdlib_named_addresses(),
        Flags::empty().set_skip_attribute_checks(false),
        KnownAttribute::get_all_attribute_names(),
    )
    .build_and_report()
    .unwrap();
    dir.close().unwrap();
}

/// Runs the given Move code as a transactional test.
pub fn run_transactional_test(code: String, config: Option<&Config>) -> Result<(), Box<dyn Error>> {
    let (file_path, dir) = create_tmp_move_file(code, None);
    let vm_test_config = TestRunConfig::ComparisonV1V2 {
        language_version: LanguageVersion::V2_0,
        v2_experiments: vec![
            (Experiment::OPTIMIZE.to_string(), true),
            (Experiment::AST_SIMPLIFY.to_string(), false),
            (Experiment::ACQUIRES_CHECK.to_string(), false),
        ],
    };
    let result = vm_test_harness::run_test_with_config_and_exp_suffix(
        vm_test_config,
        file_path.as_path(),
        &None,
    );
    dir.close().unwrap();

    let ignores = match config {
        Some(c) => c.known_error.clone(),
        None => Vec::new(),
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => process_transactional_test_err(&ignores, e),
    }
}

/// Filtering the error messages from the transactional test.
/// Currently only treat `error[Exxxx]` as a real error to ignore warnings.
fn process_transactional_test_err(
    ignores: &[String],
    err: Box<dyn Error>,
) -> Result<(), Box<dyn Error>> {
    let msg = format!("{:}", err);
    for ignore in ignores.iter() {
        if msg.contains(ignore) {
            return Ok(());
        }
    }
    if msg.contains("error[E") || msg.contains("error:") || msg.contains("bug") {
        Err(err)
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

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
}
