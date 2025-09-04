// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use shadow_rs::{is_release, shadow};
use std::collections::BTreeMap;

/// Build information keys
pub const BUILD_BRANCH: &str = "build_branch";
pub const BUILD_CARGO_VERSION: &str = "build_cargo_version";
pub const BUILD_CLEAN_CHECKOUT: &str = "build_clean_checkout";
pub const BUILD_COMMIT_HASH: &str = "build_commit_hash";
pub const BUILD_TAG: &str = "build_tag";
pub const BUILD_TIME: &str = "build_time";
pub const BUILD_OS: &str = "build_os";
pub const BUILD_PKG_VERSION: &str = "build_pkg_version";
pub const BUILD_RUST_CHANNEL: &str = "build_rust_channel";
pub const BUILD_RUST_VERSION: &str = "build_rust_version";
pub const BUILD_IS_RELEASE_BUILD: &str = "build_is_release_build";
pub const BUILD_PROFILE_NAME: &str = "build_profile_name";
pub const BUILD_USING_TOKIO_UNSTABLE: &str = "build_using_tokio_unstable";

/// This macro returns the build information as visible during build-time.
/// Use of this macro is recommended over the `get_build_information`
/// function because this macro includes the caller crate package version
/// in the returned build information map.
#[macro_export]
macro_rules! build_information {
    () => {{
        let mut build_information = velor_build_info::get_build_information();

        build_information.insert(
            velor_build_info::BUILD_PKG_VERSION.into(),
            env!("CARGO_PKG_VERSION").into(),
        );

        build_information
    }};
}

/// The only known way to get the build profile name is to look at the path
/// in the OUT_DIR env var: https://stackoverflow.com/a/73603419/3846032.
/// This env var is set during compilation, hence the use of `std::env!`.
///
/// WARNING: This does not return the expected value for the `dev`, `test`,
/// and `bench` profiles. See the SO link above for more details.
fn get_build_profile_name() -> String {
    // The profile name is always the 3rd last part of the path (with 1 based indexing).
    // e.g. /code/core/target/debug/build/velor-build-info-9f91ba6f99d7a061/out
    std::env!("OUT_DIR")
        .split(std::path::MAIN_SEPARATOR)
        .nth_back(3)
        .unwrap_or("unknown")
        .to_string()
}

/// This method returns the build information as visible during build-time.
/// Note that it is recommended to use the `build_information` macro since
/// this method does not return the build package version.
pub fn get_build_information() -> BTreeMap<String, String> {
    shadow!(build);

    let mut build_information = BTreeMap::new();

    // Get Git metadata from shadow_rs crate.
    // This is applicable for native builds where the cargo has
    // access to the .git directory.
    build_information.insert(BUILD_BRANCH.into(), build::BRANCH.into());
    build_information.insert(BUILD_CARGO_VERSION.into(), build::CARGO_VERSION.into());
    build_information.insert(BUILD_CLEAN_CHECKOUT.into(), build::GIT_CLEAN.to_string());
    build_information.insert(BUILD_COMMIT_HASH.into(), build::COMMIT_HASH.into());
    build_information.insert(BUILD_TAG.into(), build::TAG.into());
    build_information.insert(BUILD_TIME.into(), build::BUILD_TIME.into());
    build_information.insert(BUILD_OS.into(), build::BUILD_OS.into());
    build_information.insert(BUILD_RUST_CHANNEL.into(), build::RUST_CHANNEL.into());
    build_information.insert(BUILD_RUST_VERSION.into(), build::RUST_VERSION.into());

    // Compilation information
    build_information.insert(BUILD_IS_RELEASE_BUILD.into(), is_release().to_string());
    build_information.insert(BUILD_PROFILE_NAME.into(), get_build_profile_name());
    build_information.insert(
        BUILD_USING_TOKIO_UNSTABLE.into(),
        std::env!("USING_TOKIO_UNSTABLE").to_string(),
    );

    // Get Git metadata from environment variables set during build-time.
    // This is applicable for docker based builds  where the cargo cannot
    // access the .git directory, or to override shadow_rs provided info.
    if let Ok(git_sha) = std::env::var("GIT_SHA") {
        build_information.insert(BUILD_COMMIT_HASH.into(), git_sha);
    }

    if let Ok(git_branch) = std::env::var("GIT_BRANCH") {
        build_information.insert(BUILD_BRANCH.into(), git_branch);
    }

    if let Ok(git_tag) = std::env::var("GIT_TAG") {
        build_information.insert(BUILD_TAG.into(), git_tag);
    }

    if let Ok(build_date) = std::env::var("BUILD_DATE") {
        build_information.insert(BUILD_TIME.into(), build_date);
    }

    build_information
}

pub fn get_git_hash() -> String {
    // Docker builds don't have the git directory so it has to be provided by this variable
    // Otherwise, shadow will have the right commit hash
    if let Ok(git_sha) = std::env::var("GIT_SHA") {
        git_sha
    } else {
        shadow!(build);
        build::COMMIT_HASH.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_get_build_information_env_override() {
        let commit_hash = String::from("COMMIT-HASH-1");
        let build_branch = String::from("branch-1");
        let build_tag = String::from("release-1");

        std::env::set_var("GIT_SHA", commit_hash.clone());
        std::env::set_var("GIT_BRANCH", build_branch.clone());
        std::env::set_var("GIT_TAG", build_tag.clone());

        let info = get_build_information();

        assert!(info.contains_key(BUILD_COMMIT_HASH));
        assert_eq!(info.get(BUILD_COMMIT_HASH), Some(&commit_hash));

        assert!(info.contains_key(BUILD_BRANCH));
        assert_eq!(info.get(BUILD_BRANCH), Some(&build_branch));

        assert!(info.contains_key(BUILD_TAG));
        assert_eq!(info.get(BUILD_TAG), Some(&build_tag));
    }
}
