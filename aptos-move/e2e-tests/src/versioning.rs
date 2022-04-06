// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
use crate::{account::Account, executor::FakeExecutor, utils};
use aptos_types::on_chain_config::APTOS_MAX_KNOWN_VERSION;

/// The current version numbers that e2e tests should be run against.
pub const CURRENT_RELEASE_VERSIONS: std::ops::RangeInclusive<u64> =
    APTOS_MAX_KNOWN_VERSION.major..=APTOS_MAX_KNOWN_VERSION.major;

#[derive(Debug)]
pub struct VersionedTestEnv {
    pub executor: FakeExecutor,
    pub dr_account: Account,
    pub dr_sequence_number: u64,
    pub version_number: u64,
}

impl VersionedTestEnv {
    // At each release, this function will need to be updated to handle the release logic
    pub fn new(version_number: u64) -> Option<Self> {
        let (executor, dr_account) = utils::start_with_released_df();
        let dr_sequence_number = 0;

        Some(Self {
            executor,
            dr_account,
            dr_sequence_number,
            version_number,
        })
    }
}

/// This is takes a test body parametrized by a `VersionedTestEnv`, and the `versions` to test
/// against The starting state of the `VersionedTestEnv` for each version number is determined by
/// the `starting_state` function.
pub fn run_with_versions<ParamExec, F>(
    test_golden_prefix: &str,
    versions: impl Iterator<Item = u64>,
    starting_state: ParamExec,
    test_func: F,
) where
    F: Fn(VersionedTestEnv),
    ParamExec: Fn(u64) -> Option<VersionedTestEnv>,
{
    for version in versions {
        let mut testing_env = match starting_state(version) {
            None => {
                eprintln!("Unsupported version number {}", version);
                continue;
            }
            Some(env) => env,
        };
        // Tag each golden file with the version that it's being run with, and should be compared against
        testing_env
            .executor
            .set_golden_file(&format!("{}_version_{}", test_golden_prefix, version));
        test_func(testing_env)
    }
}

// This needs to be a macro so that `current_function_name` behaves correctly.
#[macro_export]
macro_rules! test_with_different_versions {
    ($versions:expr, $expr:expr) => {
        language_e2e_tests::versioning::run_with_versions(
            language_e2e_tests::current_function_name!(),
            $versions,
            language_e2e_tests::versioning::VersionedTestEnv::new,
            $expr,
        )
    };
}
