// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ReleaseBundle;
use once_cell::sync::Lazy;
use std::path::PathBuf;

static TESTNET_RELEASE_BUNDLE: Lazy<ReleaseBundle> = Lazy::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("releases")
        .join("testnet.mrb");
    let bytes = std::fs::read(path).expect("testnet.mrb exists");
    bcs::from_bytes::<ReleaseBundle>(&bytes).expect("bcs succeeds")
});

/// Returns the release bundle with which the last testnet was build or updated.
pub fn testnet_release_bundle() -> &'static ReleaseBundle {
    &TESTNET_RELEASE_BUNDLE
}
