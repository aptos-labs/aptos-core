// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::path::Path;

mod runner;

datatest_stable::harness! {
    {
        test = resolver_tests_offline,
        root = "testsuite-offline",
        pattern = r"^[^/]+/Move\.toml$",
    },
}

fn resolver_tests_offline(manifest_path: &Path) -> datatest_stable::Result<()> {
    runner::run_resolver_expected_output_tests(manifest_path)
}
