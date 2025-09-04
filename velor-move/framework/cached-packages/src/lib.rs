// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_framework::ReleaseBundle;
use once_cell::sync::Lazy;

pub mod velor_framework_sdk_builder;
pub mod velor_stdlib;
pub mod velor_token_objects_sdk_builder;
pub mod velor_token_sdk_builder;

#[cfg(unix)]
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/head.mrb"));
#[cfg(windows)]
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "\\head.mrb"));

static HEAD_RELEASE_BUNDLE: Lazy<ReleaseBundle> = Lazy::new(|| {
    bcs::from_bytes::<ReleaseBundle>(HEAD_RELEASE_BUNDLE_BYTES).expect("bcs succeeds")
});

/// Returns the release bundle for the current code.
pub fn head_release_bundle() -> &'static ReleaseBundle {
    &HEAD_RELEASE_BUNDLE
}
