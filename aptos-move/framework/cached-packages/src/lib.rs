// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::ReleaseBundle;
// use once_cell::sync::Lazy; //////// 0L ///////

pub mod aptos_framework_sdk_builder;
pub mod aptos_stdlib;
pub mod aptos_token_objects_sdk_builder;
pub mod aptos_token_sdk_builder;

//////// 0L ///////
// #[cfg(unix)]
// const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/head.mrb"));
// #[cfg(windows)]
// const HEAD_RELEASE_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "\\head.mrb"));

//////// 0L ///////
// static HEAD_RELEASE_BUNDLE: Lazy<ReleaseBundle> = Lazy::new(|| {
//     bcs::from_bytes::<ReleaseBundle>(HEAD_RELEASE_BUNDLE_BYTES).expect("bcs succeeds")
// });

//////// 0L ///////
// /// Returns the release bundle for the current code.
// pub fn head_release_bundle() -> &'static ReleaseBundle {
//     &HEAD_RELEASE_BUNDLE
// }

//////// 0L ///////
/// Returns the release bundle for the current code.
pub fn head_release_bundle() -> ReleaseBundle {
    let mrb_path = std::env::var("MRB_PATH").expect("failed to read env.var. MRB_PATH");
    let mrb_bytes = std::fs::read(mrb_path).expect("unable to read head.mrb file");
    let rls_bundle = bcs::from_bytes::<ReleaseBundle>(&mrb_bytes).expect("bcs succeeds");
    rls_bundle
}
