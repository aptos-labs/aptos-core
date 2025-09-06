// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";

use move_transactional_test_runner::{
    tasks::SyntaxChoice,
    vm_test_harness::{run_test_with_config, TestRunConfig},
};
use std::{error::Error, path::Path};

fn run(path: &Path) -> anyhow::Result<(), Box<dyn Error>> {
    run_test_with_config(
        TestRunConfig::default()
            .cross_compile_into(SyntaxChoice::ASM, true, None)
            .cross_compile_into(SyntaxChoice::Source, true, None),
        path,
    )
}

datatest_stable::harness!(run, TEST_DIR, r".*\.(mvir|move)$");
