// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

mod runner;

datatest_stable::harness!(
    resolver_tests_offline,
    "testsuite-offline",
    r"^testsuite-offline/[^/]+/Move\.toml$"
);

fn resolver_tests_offline(manifest_path: &Path) -> datatest_stable::Result<()> {
    runner::run_resolver_expected_output_tests(manifest_path)
}
