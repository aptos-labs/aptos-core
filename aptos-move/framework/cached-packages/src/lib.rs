// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::ReleaseBundle;
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;

pub mod aptos_framework_sdk_builder;
pub mod aptos_stdlib;
pub mod aptos_token_objects_sdk_builder;
pub mod aptos_token_sdk_builder;

#[cfg(unix)]
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/head.mrb"));
#[cfg(windows)]
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "\\head.mrb"));

static HEAD_RELEASE_BUNDLE: Lazy<ReleaseBundle> = Lazy::new(|| {
    let override_cached_packages_path =
        option_env!("APTOS_OVERRIDE_CACHED_PACKAGES_PATH").map(|it| PathBuf::from(it));

    let head_release_bundle_bytes = match override_cached_packages_path {
        Some(override_cached_packages_path) => {
            fs::read(override_cached_packages_path).expect(
                "APTOS_OVERRIDE_CACHED_PACKAGES_PATH file should be created at the earlier compilation step"
            )
        },
        None => Vec::from(HEAD_RELEASE_BUNDLE_BYTES),
    };

    bcs::from_bytes::<ReleaseBundle>(head_release_bundle_bytes.as_slice()).expect("bcs succeeds")
});

/// Returns the release bundle for the current code.
pub fn head_release_bundle() -> &'static ReleaseBundle {
    &HEAD_RELEASE_BUNDLE
}
