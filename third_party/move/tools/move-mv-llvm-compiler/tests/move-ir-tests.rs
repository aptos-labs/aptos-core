// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tests of compilation from .move to LLVM IR.
//!
//! # Usage
//!
//! These tests require `move-compiler` to be pre-built:
//!
//! ```
//! cargo build -p move-compiler
//! ```
//!
//! Running the tests:
//!
//! ```
//! cargo test -p move-mv-llvm-compiler --test move-ir-tests
//! ```
//!
//! Running a specific test:
//!
//! ```
//! cargo test -p move-mv-llvm-compiler --test move-ir-tests -- empty-module.move
//! ```
//!
//! Promoting all results to expected results:
//!
//! ```
//! PROMOTE_LLVM_IR=1 cargo test -p move-mv-llvm-compiler --test move-ir-tests
//! ```
//!
//! # Details
//!
//! They do the following:
//!
//! - Create a test for every .move file in mover-ir-tests/
//! - Run `move-build` to convert Move source to multiple Move bytecode
//!   files in a dedicated `-build` directory
//! - Run `move-mv-llvm-compiler` to convert Move bytecode to LLVM IR.
//! - Compare the actual IR to an existing expected IR.
//!
//! If the `PROMOTE_LLVM_IR` env var is set, the actual IR is promoted to the
//! expected IR.
//!
//! MVIR files may contain "test directives" instructing the harness
//! how to behave. These are specially-interpreted comments of the form
//!
//! - `// ignore` - don't run the test

use extension_trait::extension_trait;
use similar::{ChangeTag, TextDiff};
use std::{
    fs,
    path::{Path, PathBuf},
};

mod test_common;
use tc::TestPlan;
use test_common as tc;

pub const TEST_DIR: &str = "tests/move-ir-tests";

datatest_stable::harness!(run_test, TEST_DIR, r".*\.move$");

fn run_test(test_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    tc::setup_logging_for_test();
    Ok(run_test_inner(test_path)?)
}

fn run_test_inner(test_path: &Path) -> anyhow::Result<()> {
    let harness_paths = tc::get_harness_paths("move-compiler")?;
    let test_plan = tc::get_test_plan(test_path)?;

    if test_plan.should_ignore() {
        eprintln!("ignoring {}", test_plan.name);
        return Ok(());
    }

    tc::run_move_build(&harness_paths, &test_plan)?;

    let compilation_units = tc::find_compilation_units(&test_plan)?;

    compile_all_bytecode_to_llvm_ir(&harness_paths, &compilation_units)?;
    maybe_promote_actual_llvm_ir_to_expected(&compilation_units)?;
    compare_all_actual_llvm_ir_to_expected(&compilation_units, &test_plan)?;

    Ok(())
}

#[extension_trait]
impl CompilationUnitExt for tc::CompilationUnit {
    fn llvm_ir_actual(&self) -> PathBuf {
        self.bytecode.with_extension("actual.ll")
    }

    fn llvm_ir_expected(&self) -> PathBuf {
        self.bytecode.with_extension("expected.ll")
    }
}

fn compile_all_bytecode_to_llvm_ir(
    harness_paths: &tc::HarnessPaths,
    compilation_units: &[tc::CompilationUnit],
) -> anyhow::Result<()> {
    tc::compile_all_bytecode(harness_paths, compilation_units, None, "-S", &|cu| {
        cu.llvm_ir_actual()
    })
}

fn maybe_promote_actual_llvm_ir_to_expected(
    compilation_units: &[tc::CompilationUnit],
) -> anyhow::Result<()> {
    if std::env::var("PROMOTE_LLVM_IR").is_err() {
        return Ok(());
    }

    for cu in compilation_units {
        fs::copy(cu.llvm_ir_actual(), cu.llvm_ir_expected())?;
    }

    Ok(())
}

fn compare_all_actual_llvm_ir_to_expected(
    compilation_units: &[tc::CompilationUnit],
    test_plan: &TestPlan,
) -> anyhow::Result<()> {
    for cu in compilation_units {
        compare_actual_llvm_ir_to_expected(cu, test_plan)?;
    }

    Ok(())
}

fn compare_actual_llvm_ir_to_expected(
    compilation_unit: &tc::CompilationUnit,
    test_plan: &TestPlan,
) -> anyhow::Result<()> {
    if !compilation_unit.llvm_ir_expected().exists() {
        return test_plan.test_msg(format!(
            "no expected.ll file: {:?}",
            compilation_unit
                .llvm_ir_expected()
                .strip_prefix(test_plan.test_root())?
        ));
    }

    let mut diff_msg = String::new();
    let file_actual = fs::read_to_string(compilation_unit.llvm_ir_actual())?;
    let file_expected = fs::read_to_string(compilation_unit.llvm_ir_expected())?;

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
            "LLVM IR actual ({:?}) does not equal expected: \n\n{}",
            compilation_unit.llvm_ir_actual(),
            diff_msg
        ));
    } else {
        // If the test was expected to fail but it passed, then issue an error.
        let xfail = test_plan.xfail_message();
        if let Some(x) = xfail {
            anyhow::bail!(format!("Test expected to fail with: {}", x));
        }
    }

    Ok(())
}
