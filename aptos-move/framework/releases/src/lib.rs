// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_package::{
    compilation::compiled_package::{CompiledPackage, OnDiskCompiledPackage},
    source_package::manifest_parser::parse_move_manifest_from_file,
};
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Release {
    Aptos,
}

impl Release {
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::Aptos => "aptos-framework",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseFetcher {
    release: Release,
    release_name: String,
}

impl ReleaseFetcher {
    /// Create a new release fetcher for the given `Release` and `release_name`.
    pub fn new(release: Release, release_name: &str) -> Self {
        Self {
            release,
            release_name: release_name.to_string(),
        }
    }

    /// Fetch the current release of the given `Release`
    pub fn current(release: Release) -> Self {
        Self::new(release, "current")
    }

    /// Load the serialized modules from the specified release.
    pub fn package(&self) -> Result<CompiledPackage> {
        let root_path = Path::new(std::env!("CARGO_MANIFEST_DIR")).join(&self.release_name);
        let package_name = parse_move_manifest_from_file(&root_path)?.package.name;
        let path = root_path
            .join("releases")
            .join("artifacts")
            .join(&self.release_name)
            .join("build")
            .join(package_name.as_str());
        Ok(OnDiskCompiledPackage::from_path(&path)
            .unwrap()
            .into_compiled_package()
            .unwrap())
    }
}
