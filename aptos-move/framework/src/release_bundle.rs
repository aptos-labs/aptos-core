// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{built_package::BuiltPackage, path_in_crate};
use aptos_release_bundle::{ReleaseBundle, ReleasePackage};
use move_command_line_common::files::{extension_equals, find_filenames, MOVE_EXTENSION};

/// Creates a new released package from a built package.
pub fn new_release_package(package: BuiltPackage) -> anyhow::Result<ReleasePackage> {
    let metadata = package.extract_metadata()?;
    Ok(ReleasePackage::new(metadata, package.extract_code()))
}

/// Extension trait for `ReleaseBundle` providing methods that depend on the
/// framework crate's source layout.
pub trait ReleaseBundleExt {
    /// Returns the Move source file names which are involved in this bundle.
    fn files(&self) -> anyhow::Result<Vec<String>>;
}

impl ReleaseBundleExt for ReleaseBundle {
    fn files(&self) -> anyhow::Result<Vec<String>> {
        assert!(
            !self.source_dirs.is_empty(),
            "release bundle has no source path information"
        );
        let mut result = vec![];
        for path in &self.source_dirs {
            let path = path_in_crate(path);
            let mut files = find_filenames(&[&path], |p| extension_equals(p, MOVE_EXTENSION))?;
            result.append(&mut files);
        }
        Ok(result)
    }
}
