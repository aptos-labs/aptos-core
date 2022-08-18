// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_telemetry_service::types::telemetry::TelemetryEvent;

use crate as aptos_telemetry;
use crate::utils;
use std::collections::BTreeMap;

/// Build information event name
const APTOS_NODE_BUILD_INFORMATION: &str = "APTOS_NODE_BUILD_INFORMATION";

/// Build information keys
pub const BUILD_BRANCH: &str = "build_branch";
pub const BUILD_CARGO_VERSION: &str = "build_cargo_version";
pub const BUILD_CHAIN_ID: &str = "build_chain_id";
pub const BUILD_CLAP_VERSION: &str = "build_clap_version";
pub const BUILD_COMMIT_HASH: &str = "build_commit_hash";
pub const BUILD_OS: &str = "build_os";
pub const BUILD_PKG_VERSION: &str = "build_pkg_version";
pub const BUILD_PROJECT_NAME: &str = "build_project_name";
pub const BUILD_RUST_CHANNEL: &str = "build_rust_channel";
pub const BUILD_RUST_VERSION: &str = "build_rust_version";
pub const BUILD_TAG: &str = "build_tag";
pub const BUILD_TARGET: &str = "build_target";
pub const BUILD_TARGET_ARCH: &str = "build_target_arch";
pub const BUILD_TIME: &str = "build_time";
pub const BUILD_VERSION: &str = "build_version";

// Used by external crates to collect crate specific build information
#[macro_export]
macro_rules! collect_build_information {
    () => {{
        // Collect and return the build information
        let mut build_information: std::collections::BTreeMap<String, String> = BTreeMap::new();

        // Get Git metadata from environment variables set during build-time.
        // This is applicable for docker based builds.
        if let Ok(git_sha) = std::env::var("GIT_SHA") {
            build_information.insert(
                aptos_telemetry::build_information::BUILD_COMMIT_HASH.into(),
                git_sha,
            );
        }

        if let Ok(git_branch) = std::env::var("GIT_BRANCH") {
            build_information.insert(
                aptos_telemetry::build_information::BUILD_BRANCH.into(),
                git_branch,
            );
        }

        if let Ok(git_tag) = std::env::var("GIT_TAG") {
            build_information.insert(
                aptos_telemetry::build_information::BUILD_TAG.into(),
                git_tag,
            );
        }

        if let Ok(build_date) = std::env::var("BUILD_DATE") {
            build_information.insert(
                aptos_telemetry::build_information::BUILD_TIME.into(),
                build_date,
            );
        }

        build_information
    }};
}

/// Collects and sends the build information via telemetry
pub(crate) async fn create_build_info_telemetry_event(chain_id: String) -> TelemetryEvent {
    // Collect the build information
    let build_information = get_build_information(Some(chain_id));

    // Create and return a new telemetry event
    TelemetryEvent {
        name: APTOS_NODE_BUILD_INFORMATION.into(),
        params: build_information,
    }
}

/// Used to collect build information
pub(crate) fn get_build_information(chain_id: Option<String>) -> BTreeMap<String, String> {
    let mut build_information = collect_build_information!();
    utils::insert_optional_value(&mut build_information, BUILD_CHAIN_ID, chain_id);
    build_information
}

#[test]
fn test_get_build_information() {
    let commit_hash = String::from("COMMIT-HASH-1");
    let build_branch = String::from("branch-1");
    let build_tag = String::from("release-1");

    std::env::set_var("GIT_SHA", commit_hash.clone());
    std::env::set_var("GIT_BRANCH", build_branch.clone());
    std::env::set_var("GIT_TAG", build_tag.clone());

    let info = get_build_information(None);

    assert!(info.contains_key(BUILD_COMMIT_HASH));
    assert_eq!(info.get(BUILD_COMMIT_HASH), Some(&commit_hash));

    assert!(info.contains_key(BUILD_BRANCH));
    assert_eq!(info.get(BUILD_BRANCH), Some(&build_branch));

    assert!(info.contains_key(BUILD_TAG));
    assert_eq!(info.get(BUILD_TAG), Some(&build_tag));
}
