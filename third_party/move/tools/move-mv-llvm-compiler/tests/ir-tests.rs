// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tests of compilation from MVIR to LLVM IR.
//!
//! # Usage
//!
//! These tests require `move-ir-compiler` to be pre-built:
//!
//! ```
//! cargo build -p move-ir-compiler
//! ```
//!
//! Running the tests:
//!
//! ```
//! cargo test -p move-mv-llvm-compiler --test ir-tests
//! ```
//!
//! Running a specific test:
//!
//! ```
//! cargo test -p move-mv-llvm-compiler --test ir-tests -- struct.mvir
//! ```
//!
//! Promoting all results to expected results:
//!
//! ```
//! PROMOTE_LLVM_IR=1 cargo test -p move-mv-llvm-compiler --test ir-tests
//! ```
//!
//! # Details
//!
//! They do the following:
//!
//! - Create a test for every .mvir file in ir-tests/
//! - Run `move-ir-compiler` to convert MVIR to MV bytecode.
//! - Run `move-mv-llvm-compiler` to convert MV bytecode to LLVM IR.
//! - Compare the actual IR to an existing expected IR.
//!
//! If the `PROMOTE_LLVM_IR` env var is set, the actual IR is promoted to the
//! expected IR.
//!
//! MVIR files may contain "test directives" instructing the harness
//! how to behave. These are specially-interpreted comments of the form
//!
//! - `// ignore` - don't run the test

use anyhow::Context;
use similar::{ChangeTag, TextDiff};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

mod test_common;
use test_common as tc;

pub const TEST_DIR: &str = "tests/ir-tests";

datatest_stable::harness!(run_test, TEST_DIR, r".*\.mvir$");

fn run_test(test_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    tc::setup_logging_for_test();
    Ok(run_test_inner(test_path)?)
}

fn run_test_inner(test_path: &Path) -> anyhow::Result<()> {
    let harness_paths = tc::get_harness_paths("move-ir-compiler")?;
    let test_plan = get_test_plan(test_path)?;

    if test_plan.should_ignore() {
        eprintln!("ignoring {}", test_plan.name);
        return Ok(());
    }

    compile_mvir_to_mvbc(&harness_paths, &test_plan)?;
    compile_mvbc_to_llvmir(&harness_paths, &test_plan)?;
    maybe_promote_actual_llvmir_to_expected(&test_plan)?;
    compare_actual_llvmir_to_expected(&test_plan)?;

    Ok(())
}

#[derive(Debug)]
struct TestPlan {
    name: String,
    /// The mvir file to be compiled to LLVM IR
    mvir_file: PathBuf,
    /// The move bytecode file, compiled from mvir, compiled to LLVM
    mvbc_file: PathBuf,
    /// The final LLVM IR file
    llir_file: PathBuf,
    /// The existing LLVM IR file to compare to
    llir_file_expected: PathBuf,
    /// Special commands embedded in the test file as comments
    directives: Vec<TestDirective>,
}

#[derive(Debug, Eq, PartialEq)]
enum TestDirective {
    Ignore,
    Xfail(String), // The test is expected to fail with the `String` message. It is an error if test passes.
}

impl TestPlan {
    fn should_ignore(&self) -> bool {
        self.directives.contains(&TestDirective::Ignore)
    }

    pub fn xfail_message(&self) -> Option<String> {
        self.directives.iter().find_map(|d| match d {
            TestDirective::Xfail(message) => Some(message.clone()),
            _ => None,
        })
    }

    pub fn test_msg(&self, msg: String) -> anyhow::Result<()> {
        let xfail = self.xfail_message();
        match xfail {
            Some(x) => {
                if x.contains(&msg) {
                    Ok(())
                } else {
                    anyhow::bail!(format!("XFAIL: expected: {}, got: {}", x, msg));
                }
            }
            _ => anyhow::bail!(msg),
        }
    }
}

fn get_test_plan(test_path: &Path) -> anyhow::Result<TestPlan> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo_manifest_dir");
    let mvir_file = Path::new(&manifest_dir).join(test_path);

    let name = test_path.to_string_lossy().to_string();
    let mvbc_file = mvir_file.with_extension("mv");
    let llir_file = mvir_file.with_extension("actual.ll");
    let llir_file_expected = mvir_file.with_extension("expected.ll");
    let directives = load_directives(test_path)?;

    Ok(TestPlan {
        name,
        mvir_file,
        mvbc_file,
        llir_file,
        llir_file_expected,
        directives,
    })
}

fn load_directives(test_path: &Path) -> anyhow::Result<Vec<TestDirective>> {
    let mut directives = Vec::new();
    let source = std::fs::read_to_string(test_path)?;

    for line in source.lines() {
        let line = line.trim();
        let line_is_comment = line.starts_with("//");
        if !line_is_comment {
            continue;
        }
        let line = &line[2..].trim();
        if line.starts_with("ignore") {
            directives.push(TestDirective::Ignore);
        }
        if line.starts_with("xfail: ") {
            let msg = line["xfail:".len()..].trim();
            directives.push(TestDirective::Xfail(msg.to_string()));
        }
    }

    Ok(directives)
}

/// Run `move-ir-compiler` to produce Move bytecode, `mvbc_file`.
fn compile_mvir_to_mvbc(
    harness_paths: &tc::HarnessPaths,
    test_plan: &TestPlan,
) -> anyhow::Result<()> {
    let mut cmd = Command::new(harness_paths.dep.to_str().expect("PathBuf"));
    cmd.arg("-m");
    cmd.arg(test_plan.mvir_file.to_str().expect("PathBuf"));

    let output = cmd.output()?;
    if !output.status.success() {
        anyhow::bail!(
            "move-ir-compiler failed. stderr:\n\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Run `move-mv-llvm-compiler` to produce LLVM IR, `llir_file`.
fn compile_mvbc_to_llvmir(
    harness_paths: &tc::HarnessPaths,
    test_plan: &TestPlan,
) -> anyhow::Result<()> {
    let mut cmd = Command::new(
        harness_paths
            .move_mv_llvm_compiler
            .to_str()
            .expect("PathBuf"),
    );
    cmd.arg("-b");
    cmd.arg(test_plan.mvbc_file.to_str().expect("PathBuf"));
    cmd.arg("-o");
    cmd.arg(test_plan.llir_file.to_str().expect("PathBuf"));
    cmd.arg("-S");

    let output = cmd.output().context("run move-mv-llvm-compiler failed")?;
    if !output.status.success() {
        anyhow::bail!(
            "move-mv-llvm-compiler failed. stderr:\n\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Copy actual LLVM IR, `llir_file`, to expected IR, `llir_file_expected`, if `PROMOTE_LLVM_IR` env var is set.
fn maybe_promote_actual_llvmir_to_expected(test_plan: &TestPlan) -> anyhow::Result<()> {
    if std::env::var("PROMOTE_LLVM_IR").is_ok() {
        std::fs::copy(
            test_plan.llir_file.as_path(),
            test_plan.llir_file_expected.as_path(),
        )?;
    }

    Ok(())
}

/// Compare `llir_file` to `llir_file_expected`.
///
/// If different, print a diff.
fn compare_actual_llvmir_to_expected(test_plan: &TestPlan) -> anyhow::Result<()> {
    if !test_plan.llir_file_expected.exists() {
        return test_plan.test_msg("no expected.ll file".to_string());
    }

    let mut diff_msg = String::new();
    let file_actual = std::fs::read_to_string(test_plan.llir_file.as_path())?;
    let file_expected = std::fs::read_to_string(test_plan.llir_file_expected.as_path())?;

    let diff = TextDiff::from_lines(&file_expected, &file_actual);
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => Some("-"),
            ChangeTag::Insert => Some("+"),
            ChangeTag::Equal => None,
        };

        if let Some(sign) = sign {
            diff_msg.push_str(&format!("{}{}", sign, change));
        }
    }

    if !diff_msg.is_empty() {
        return test_plan.test_msg(format!(
            "llvm IR actual does not equal expected: \n\n{}",
            diff_msg
        ));
    }
    Ok(())
}
