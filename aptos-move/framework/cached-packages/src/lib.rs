// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::ReleaseBundle;
use once_cell::sync::Lazy;

pub mod aptos_framework_sdk_builder;
pub mod aptos_stdlib;
pub mod aptos_token_objects_sdk_builder;
pub mod aptos_token_sdk_builder;

#[cfg(unix)]
//////// 0L ///////
// 0L: MRB_PATH: the path of ol-fw release bundle file "head.mrb" (e.g. /opt/libra-v7/)
// const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/head.mrb"));
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("MRB_PATH"), "/head.mrb"));

#[cfg(windows)]
const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "\\head.mrb"));

static HEAD_RELEASE_BUNDLE: Lazy<ReleaseBundle> = Lazy::new(|| {
    bcs::from_bytes::<ReleaseBundle>(HEAD_RELEASE_BUNDLE_BYTES).expect("bcs succeeds")
});

/// Returns the release bundle for the current code.
pub fn head_release_bundle() -> &'static ReleaseBundle {
    &HEAD_RELEASE_BUNDLE
}
