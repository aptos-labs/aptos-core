// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Prebuilt packages are automatically compiled by build.rs during cargo build.
// To skip the build step (e.g., for debugging), set SKIP_PREBUILT_PACKAGES_BUILD=1.

use aptos_sdk::bcs;
use aptos_transaction_generator_lib::{
    entry_point_trait::PreBuiltPackages, publishing::prebuild_packages::PrebuiltPackagesBundle,
};
use once_cell::sync::Lazy;

/// Bytes of all pre-built packages (compiled by build.rs).
#[cfg(unix)]
const PREBUILT_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/prebuilt.mpb"));
#[cfg(windows)]
const PREBUILT_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "\\prebuilt.mpb"));

/// Pre-built deserialized data: for each package, stores package metadata, compiled modules and
/// scripts.
static PREBUILT_BUNDLE: Lazy<PrebuiltPackagesBundle> = Lazy::new(|| {
    bcs::from_bytes::<PrebuiltPackagesBundle>(PREBUILT_BUNDLE_BYTES)
        .expect("prebuilt.mpb can be deserialized")
});

#[derive(Debug)]
pub struct PreBuiltPackagesImpl;

impl PreBuiltPackages for PreBuiltPackagesImpl {
    fn package_bundle(&self) -> &PrebuiltPackagesBundle {
        &PREBUILT_BUNDLE
    }
}
