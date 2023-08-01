// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";

use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use std::path::Path;

datatest_stable::harness!(run, TEST_DIR, r".*\.move$");

fn run(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    vm_test_harness::run_test_with_config(TestRunConfig::ComparisonV1V2, path)
}
