// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_release_bundle::ReleaseBundle;
use once_cell::sync::Lazy;

pub mod aptos_framework_sdk_builder;
pub mod aptos_stdlib;
pub mod aptos_token_objects_sdk_builder;
pub mod aptos_token_sdk_builder;

#[cfg(not(feature = "move-harness-with-test-only"))]
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!("head.mrb");
#[cfg(feature = "move-harness-with-test-only")]
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!("head-test-only.mrb");

static HEAD_RELEASE_BUNDLE: Lazy<ReleaseBundle> = Lazy::new(|| {
    bcs::from_bytes::<ReleaseBundle>(HEAD_RELEASE_BUNDLE_BYTES).expect("bcs succeeds")
});

/// Returns the release bundle for the current code.
pub fn head_release_bundle() -> &'static ReleaseBundle {
    &HEAD_RELEASE_BUNDLE
}
