// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

mod runner;

datatest_stable::harness!(
    resolver_tests_online,
    "testsuite-online",
    r"^testsuite-online/[^/]+/Move\.toml$"
);

fn resolver_tests_online(manifest_path: &Path) -> datatest_stable::Result<()> {
    runner::run_resolver_expected_output_tests(manifest_path)
}
