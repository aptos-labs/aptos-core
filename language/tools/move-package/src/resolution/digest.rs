// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::source_package::parsed_manifest::PackageDigest;

pub fn compute_digest(paths: &[PathBuf]) -> Result<PackageDigest> {
    let mut hasher = Sha256::new();

    for path in paths {
        if path.is_file() {
            let contents = std::fs::read(path)?;
            hasher.update(contents);
        } else {
            for entry in walkdir::WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if entry.file_type().is_file() {
                    let contents = std::fs::read(path)?;
                    hasher.update(contents);
                }
            }
        }
    }

    Ok(PackageDigest::from(format!("{:X}", hasher.finalize())))
}
