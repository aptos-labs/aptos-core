// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::{fs, fs::File, path::Path};

pub fn convert_svg_to_string(svg_file_path: &Path) -> Result<String> {
    fs::read_to_string(svg_file_path).map_err(|e| e.into())
}

pub fn create_file_with_parents<P: AsRef<Path>>(path: P) -> Result<File, std::io::Error> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    File::create(path)
}
