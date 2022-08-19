// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod aptos;

pub use aptos::*;
use std::io::{Read, Write};

mod built_package;
pub use built_package::*;

mod module_metadata;
pub use module_metadata::*;

mod error_map;
pub mod natives;
mod release_builder;
pub use release_builder::*;
mod release_bundle;
pub use release_bundle::*;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
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

pub fn zip_metadata(data: &[u8]) -> anyhow::Result<String> {
    let mut e = GzEncoder::new(Vec::new(), Compression::best());
    e.write_all(data)?;
    Ok(base64::encode(e.finish()?))
}

pub fn unzip_metadata(data: &str) -> anyhow::Result<Vec<u8>> {
    let bytes = base64::decode(data)?;
    let mut d = GzDecoder::new(bytes.as_slice());
    let mut res = vec![];
    d.read_to_end(&mut res)?;
    Ok(res)
}
