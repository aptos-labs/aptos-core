// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn convert_svg_to_string(svg_file_path: &Path) -> Result<String> {
    fs::read_to_string(svg_file_path).map_err(|e| e.into())
}
