// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_transactional_test_harness::run_aptos_test_with_config;
use move_model::metadata::LanguageVersion;
use move_transactional_test_runner::vm_test_harness::TestRunConfig;
use std::path::Path;

datatest_stable::harness!(runner, "tests", r".*\.(move|masm)$");

fn runner(path: &Path) -> anyhow::Result<(), Box<dyn std::error::Error>> {
    run_aptos_test_with_config(
        path,
        TestRunConfig::new(LanguageVersion::latest_stable(), vec![(
            "attach-compiled-module".to_owned(),
            true,
        )]),
    )
}
