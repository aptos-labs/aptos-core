// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This file was generated. Do not modify!
//
// To update this code, run `testsuite/benchmark-workloads/generate.py`.

use velor_sdk::bcs;
use velor_transaction_generator_lib::{
    entry_point_trait::PreBuiltPackages, publishing::prebuild_packages::PrebuiltPackagesBundle,
};
use once_cell::sync::Lazy;

/// Bytes of all pre-build packages.
#[rustfmt::skip]
const PREBUILT_BUNDLE_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/prebuilt.mpb"));

/// Pre-built deserialized data: for each package, stores package metadata, compiled modules and
/// scripts.
#[rustfmt::skip]
static PREBUILT_BUNDLE: Lazy<PrebuiltPackagesBundle> = Lazy::new(|| {
    bcs::from_bytes::<PrebuiltPackagesBundle>(PREBUILT_BUNDLE_BYTES)
        .expect("prebuilt.mpb can be deserialized")
});

#[rustfmt::skip]
#[derive(Debug)]
pub struct PreBuiltPackagesImpl;

#[rustfmt::skip]
impl PreBuiltPackages for PreBuiltPackagesImpl {
    fn package_bundle(&self) -> &PrebuiltPackagesBundle {
        &PREBUILT_BUNDLE
    }
}
