// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::ReleaseBundle;
use once_cell::sync::Lazy;

pub mod aptos_framework_sdk_builder;
pub mod aptos_stdlib;
pub mod aptos_token_objects_sdk_builder;
pub mod aptos_token_sdk_builder;

#[cfg(unix)]
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/head.mrb"));
#[cfg(windows)]
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "\\head.mrb"));

static HEAD_RELEASE_BUNDLE: Lazy<ReleaseBundle> = Lazy::new(|| {
    #[cfg(debug_assertions)]
    let head_release_bundle_bytes = {
        match std::env::var("APTOS_FRAMEWORK_BUILD_PATH").ok() {
            Some(cached_framework_path) => std::fs::read(cached_framework_path).expect(
                "APTOS_FRAMEWORK_BUILD_PATH file is created at the earlier compilation step",
            ),
            None => Vec::from(HEAD_RELEASE_BUNDLE_BYTES),
        }
    };
    #[cfg(not(debug_assertions))]
    let head_release_bundle_bytes = Vec::from(HEAD_RELEASE_BUNDLE_BYTES);

    bcs::from_bytes::<ReleaseBundle>(head_release_bundle_bytes.as_slice()).expect("bcs succeeds")
});

/// Returns the release bundle for the current code.
pub fn head_release_bundle() -> &'static ReleaseBundle {
    &HEAD_RELEASE_BUNDLE
}
