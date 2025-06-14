// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod manifest;
mod named_address;
mod package_name;
mod util;

pub use manifest::{
    AddressAssignment, BuildOptions, Dependency, PackageLocation, PackageManifest, PackageMetadata,
    Version,
};
pub use named_address::NamedAddress;
pub use package_name::PackageName;
pub use util::render_error;

pub fn parse_package_manifest(s: &str) -> Result<PackageManifest, toml::de::Error> {
    toml::from_str(s)
}
