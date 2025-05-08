// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod manifest;
mod util;

pub use manifest::{
    AddressAssignment, BuildInfo, Dependency, PackageInfo, PackageLocation, PackageManifest,
    Version,
};
pub use util::render_error;

pub fn parse_package_manifest(s: &str) -> Result<PackageManifest, toml::de::Error> {
    toml::from_str(s)
}
