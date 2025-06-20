// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod aptos;

pub use aptos::*;
use std::io::{Read, Write};

mod built_package;
pub use built_package::*;

pub mod natives;
mod release_builder;
pub use release_builder::*;
pub mod chunked_publish;
pub mod docgen;
pub mod extended_checks;
pub mod prover;
mod release_bundle;
mod released_framework;

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
pub use release_bundle::*;
pub use released_framework::*;
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
    let mut e = GzEncoder::new(Vec::new(), Compression::best());
    e.write_all(data)?;
    Ok(e.finish()?)
}

pub fn zip_metadata_str(s: &str) -> anyhow::Result<Vec<u8>> {
    zip_metadata(s.as_bytes())
}

pub fn unzip_metadata(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut d = GzDecoder::new(data);
    let mut res = vec![];
    d.read_to_end(&mut res)?;
    Ok(res)
}

pub fn unzip_metadata_str(data: &[u8]) -> anyhow::Result<String> {
    let r = unzip_metadata(data)?;
    let s = String::from_utf8(r)?;
    Ok(s)
}
