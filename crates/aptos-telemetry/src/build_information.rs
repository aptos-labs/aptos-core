// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{service::TelemetryEvent, utils};
use shadow_rs::shadow;
use std::collections::BTreeMap;

/// Build information event name
const APTOS_NODE_BUILD_INFORMATION: &str = "APTOS_NODE_BUILD_INFORMATION";

/// Build information keys
const BUILD_BRANCH: &str = "build_branch";
const BUILD_CARGO_VERSION: &str = "build_cargo_version";
const BUILD_CHAIN_ID: &str = "build_chain_id";
const BUILD_CLAP_VERSION: &str = "build_clap_version";
const BUILD_COMMIT_HASH: &str = "build_commit_hash";
const BUILD_OS: &str = "build_os";
const BUILD_PKG_VERSION: &str = "build_pkg_version";
const BUILD_PROJECT_NAME: &str = "build_project_name";
const BUILD_RUST_CHANNEL: &str = "build_rust_channel";
const BUILD_RUST_VERSION: &str = "build_rust_version";
const BUILD_TAG: &str = "build_tag";
const BUILD_TARGET: &str = "build_target";
const BUILD_TARGET_ARCH: &str = "build_target_arch";
const BUILD_TIME: &str = "build_time";
const BUILD_VERSION: &str = "build_version";

// Get access to BUILD information
shadow!(build);

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
    let mut build_information: BTreeMap<String, String> = BTreeMap::new();
    collect_build_info(chain_id, &mut build_information);
    build_information
}

/// Collects the build info and appends it to the given map
pub(crate) fn collect_build_info(
    chain_id: Option<String>,
    build_information: &mut BTreeMap<String, String>,
) {
    build_information.insert(BUILD_BRANCH.into(), build::BRANCH.into());
    build_information.insert(BUILD_CARGO_VERSION.into(), build::CARGO_VERSION.into());
    utils::insert_optional_value(build_information, BUILD_CHAIN_ID, chain_id);
    build_information.insert(BUILD_CLAP_VERSION.into(), build::CLAP_LONG_VERSION.into());
    build_information.insert(BUILD_COMMIT_HASH.into(), build::COMMIT_HASH.into());
    build_information.insert(BUILD_OS.into(), build::BUILD_OS.into());
    build_information.insert(BUILD_PKG_VERSION.into(), build::PKG_VERSION.into());
    build_information.insert(BUILD_PROJECT_NAME.into(), build::PROJECT_NAME.into());
    build_information.insert(BUILD_RUST_CHANNEL.into(), build::RUST_CHANNEL.into());
    build_information.insert(BUILD_RUST_VERSION.into(), build::RUST_VERSION.into());
    build_information.insert(BUILD_TAG.into(), build::TAG.into());
    build_information.insert(BUILD_TARGET.into(), build::BUILD_TARGET.into());
    build_information.insert(BUILD_TARGET_ARCH.into(), build::BUILD_TARGET_ARCH.into());
    build_information.insert(BUILD_TIME.into(), build::BUILD_TIME.into());
    build_information.insert(BUILD_VERSION.into(), build::VERSION.into());
}
