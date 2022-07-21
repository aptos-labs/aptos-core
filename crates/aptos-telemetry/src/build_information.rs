// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate as aptos_telemetry;
use crate::{service::TelemetryEvent, utils};
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
        // Get access to shadow BUILD information
        shadow_rs::shadow!(build);

        // Collect and return the build information
        let mut build_information: std::collections::BTreeMap<String, String> = BTreeMap::new();
        build_information.insert(
            aptos_telemetry::build_information::BUILD_BRANCH.into(),
            build::BRANCH.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_CARGO_VERSION.into(),
            build::CARGO_VERSION.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_CLAP_VERSION.into(),
            build::CLAP_LONG_VERSION.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_COMMIT_HASH.into(),
            build::COMMIT_HASH.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_OS.into(),
            build::BUILD_OS.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_PKG_VERSION.into(),
            build::PKG_VERSION.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_PROJECT_NAME.into(),
            build::PROJECT_NAME.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_RUST_CHANNEL.into(),
            build::RUST_CHANNEL.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_RUST_VERSION.into(),
            build::RUST_VERSION.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_TAG.into(),
            build::TAG.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_TARGET.into(),
            build::BUILD_TARGET.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_TARGET_ARCH.into(),
            build::BUILD_TARGET_ARCH.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_TIME.into(),
            build::BUILD_TIME.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_VERSION.into(),
            build::VERSION.into(),
        );
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
