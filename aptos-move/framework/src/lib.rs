// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod aptos;

pub use aptos::*;

mod built_package;
pub use built_package::*;

mod error_map;
pub mod natives;
mod release_builder;
pub use release_builder::*;
mod release_bundle;
pub use release_bundle::*;

use anyhow::bail;
use miniz_oxide::deflate::compress_to_vec;
use miniz_oxide::inflate::decompress_to_vec;
use std::path::PathBuf;

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative.into())
}

pub(crate) fn path_relative_to_crate(path: PathBuf) -> PathBuf {
    let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.strip_prefix(crate_path).unwrap_or(&path).to_path_buf()
}

pub fn zip_metadata(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    Ok(compress_to_vec(data, 10))
}

pub fn unzip_metadata(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    match decompress_to_vec(data) {
        Ok(r) => Ok(r),
        Err(e) => bail!("decompression error: {:?}", e),
    }
}
