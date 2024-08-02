// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_transactional_test_harness::run_aptos_test_with_config;
use move_model::metadata::LanguageVersion;
use move_transactional_test_runner::vm_test_harness::TestRunConfig;
use std::path::Path;

datatest_stable::harness!(runner, "tests", r".*\.(mvir|move)$");

fn runner(path: &Path) -> anyhow::Result<(), Box<dyn std::error::Error>> {
    if path.to_str().unwrap().contains("v2-tests/") {
        // TODO: we may later want to change this to comparison testing. For now we are mostly
        //    interested in debugging v2 bytecode.
        run_aptos_test_with_config(path, TestRunConfig::CompilerV2 {
            language_version: LanguageVersion::default(),
            v2_experiments: vec![("attach-compiled-module".to_owned(), true)],
        })
    } else {
        run_aptos_test_with_config(path, TestRunConfig::CompilerV1)
    }
}
