// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_framework::{extended_checks, prover::ProverOptions};
use move_binary_format::file_format_common::VERSION_DEFAULT;
use move_model::metadata::{CompilerVersion, LanguageVersion};
use std::{collections::BTreeMap, path::PathBuf};

const ENV_TEST_INCONSISTENCY: &str = "MVP_TEST_INCONSISTENCY";
const ENV_TEST_UNCONDITIONAL_ABORT_AS_INCONSISTENCY: &str =
    "MVP_TEST_UNCONDITIONAL_ABORT_AS_INCONSISTENCY";
const ENV_TEST_DISALLOW_TIMEOUT_OVERWRITE: &str = "MVP_TEST_DISALLOW_TIMEOUT_OVERWRITE";
const ENV_TEST_VC_TIMEOUT: &str = "MVP_TEST_VC_TIMEOUT";

// Note: to run these tests, use:
//
//   cargo test -- --include-ignored prover

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

pub fn read_env_var(v: &str) -> String {
    std::env::var(v).unwrap_or_else(|_| String::new())
}

pub fn run_prover_for_pkg(
    path_to_pkg: impl Into<String>,
    shards: usize,
    only_shard: Option<usize>,
) {
    let pkg_path = path_in_crate(path_to_pkg);
    let mut options = ProverOptions::default_for_test();
    let no_tools = read_env_var("BOOGIE_EXE").is_empty()
        || !options.cvc5 && read_env_var("Z3_EXE").is_empty()
        || options.cvc5 && read_env_var("CVC5_EXE").is_empty();
    if no_tools {
        panic!(
            "Prover tools are not configured, \
        See https://github.com/velor-chain/velor-core/blob/main/velor-move/framework/FRAMEWORK-PROVER-GUIDE.md \
        for instructions, or \
        use \"-- --skip prover\" to filter out the prover tests"
        );
    } else {
        let inconsistency_flag = read_env_var(ENV_TEST_INCONSISTENCY) == "1";
        let unconditional_abort_inconsistency_flag =
            read_env_var(ENV_TEST_UNCONDITIONAL_ABORT_AS_INCONSISTENCY) == "1";
        let disallow_timeout_overwrite = read_env_var(ENV_TEST_DISALLOW_TIMEOUT_OVERWRITE) == "1";
        options.shards = Some(shards);
        options.only_shard = only_shard;
        options.check_inconsistency = inconsistency_flag;
        options.unconditional_abort_as_inconsistency = unconditional_abort_inconsistency_flag;
        options.disallow_global_timeout_to_be_overwritten = disallow_timeout_overwrite;
        options.vc_timeout = read_env_var(ENV_TEST_VC_TIMEOUT)
            .parse::<usize>()
            .ok()
            .or(options.vc_timeout);
        let skip_attribute_checks = false;
        options
            .prove(
                false,
                pkg_path.as_path(),
                BTreeMap::default(),
                Some(VERSION_DEFAULT),
                Some(CompilerVersion::latest_stable()),
                Some(LanguageVersion::latest_stable()),
                skip_attribute_checks,
                extended_checks::get_all_attribute_names(),
                &[],
            )
            .unwrap()
    }
}

#[test]
fn move_framework_prover_tests() {
    run_prover_for_pkg("velor-framework", 1, None);
}

#[test]
fn move_token_prover_tests() {
    run_prover_for_pkg("velor-token", 1, None);
}

#[test]
fn move_velor_stdlib_prover_tests() {
    run_prover_for_pkg("velor-stdlib", 1, None);
}

#[test]
fn move_stdlib_prover_tests() {
    run_prover_for_pkg("move-stdlib", 1, None);
}
