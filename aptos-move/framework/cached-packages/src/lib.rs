// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::ReleaseBundle;
use once_cell::sync::Lazy;

pub mod aptos_framework_sdk_builder;
pub mod aptos_stdlib;
pub mod aptos_token_objects_sdk_builder;
pub mod aptos_token_sdk_builder;

#[cfg(unix)]
const HEAD_RELEASE_BUNDLE_STR: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/generated/head.mrb"));
#[cfg(windows)]
const HEAD_RELEASE_BUNDLE_STR: &str =
    include_str!(concat!(env!("OUT_DIR"), "\\generated\\head.mrb"));

static HEAD_RELEASE_BUNDLE: Lazy<ReleaseBundle> =
    Lazy::new(|| ReleaseBundle::read_str(HEAD_RELEASE_BUNDLE_STR).expect("deserialize succeeds"));

/// Returns the release bundle for the current code.
pub fn head_release_bundle() -> &'static ReleaseBundle {
    &HEAD_RELEASE_BUNDLE
}
