// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::ReleaseBundle;
use once_cell::sync::Lazy;

pub mod aptos_framework_sdk_builder;
pub mod aptos_stdlib;
pub mod aptos_token_sdk_builder;

const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/head.mrb"));

static HEAD_RELEASE_BUNDLE: Lazy<ReleaseBundle> = Lazy::new(|| {
    bcs::from_bytes::<ReleaseBundle>(HEAD_RELEASE_BUNDLE_BYTES).expect("bcs succeeds")
});

/// Returns the release bundle for the current code.
pub fn head_release_bundle() -> &'static ReleaseBundle {
    &HEAD_RELEASE_BUNDLE
}

/// Placeholder for returning the release bundle for the last devnet release(?).
/// TODO: this is currently only used to differentiate between GenesisOptions::Fresh
/// and GenesisOptions::Compiled. It is not clear what the difference should be.
/// For now, we return the same as with head_release_bundle.
pub fn devnet_release_bundle() -> &'static ReleaseBundle {
    &HEAD_RELEASE_BUNDLE
}
