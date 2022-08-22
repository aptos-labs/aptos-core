// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use shadow_rs::shadow;

/// Build information keys
pub const BUILD_BRANCH: &str = "build_branch";
pub const BUILD_CARGO_VERSION: &str = "build_cargo_version";
pub const BUILD_COMMIT_HASH: &str = "build_commit_hash";
pub const BUILD_TAG: &str = "build_tag";
pub const BUILD_TIME: &str = "build_time";
pub const BUILD_OS: &str = "build_os";
pub const BUILD_PKG_VERSION: &str = "build_pkg_version";
pub const BUILD_RUST_CHANNEL: &str = "build_rust_channel";
pub const BUILD_RUST_VERSION: &str = "build_rust_version";

pub type BuildInfomation = BTreeMap<String, String>;

pub fn get_build_information() -> BuildInfomation {
    shadow!(build);

    let mut build_information = BTreeMap::new();

    // Get Git metadata from shadow_rs crate.
    // This is applicable for native builds where the cargo has
    // access to the .git directory.
    build_information.insert(BUILD_BRANCH.into(), build::BRANCH.into());
    build_information.insert(BUILD_CARGO_VERSION.into(), build::CARGO_VERSION.into());
    build_information.insert(BUILD_COMMIT_HASH.into(), build::COMMIT_HASH.into());
    build_information.insert(BUILD_TAG.into(), build::TAG.into());
    build_information.insert(BUILD_TIME.into(), build::BUILD_TIME.into());
    build_information.insert(BUILD_OS.into(), build::BUILD_OS.into());
    build_information.insert(BUILD_PKG_VERSION.into(), build::PKG_VERSION.into());
    build_information.insert(BUILD_RUST_CHANNEL.into(), build::RUST_CHANNEL.into());
    build_information.insert(BUILD_RUST_VERSION.into(), build::RUST_VERSION.into());

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
