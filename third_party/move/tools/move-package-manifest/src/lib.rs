// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod manifest;

pub use manifest::{
    AddressAssignment, BuildInfo, Dependency, PackageInfo, PackageLocation, PackageManifest,
    Version,
};

pub fn parse_package_manifest(s: &str) -> Result<PackageManifest, toml::de::Error> {
    toml::from_str(s)
}
