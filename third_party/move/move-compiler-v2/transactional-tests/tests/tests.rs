// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";

use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use std::path::Path;

datatest_stable::harness!(run, TEST_DIR, r".*\.move$");

/// Root of tests which require to skip v1-v2 comparison
const SKIP_V1_COMPARISON_PATH: &str = "/no-v1-comparison/";

fn run(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let test_config: TestRunConfig = if let Some(p) = path.to_str() {
        if p.contains(SKIP_V1_COMPARISON_PATH) {
            TestRunConfig::CompilerV2
        } else {
            TestRunConfig::ComparisonV1V2
        }
    } else {
        TestRunConfig::ComparisonV1V2
    };
    vm_test_harness::run_test_with_config(test_config, path)
}
