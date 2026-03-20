// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

mod aptos;

pub use aptos::*;
use std::io::Write;

mod built_package;
pub use aptos_framework_natives as natives;
pub use built_package::*;
mod release_builder;
pub use release_builder::*;
pub mod chunked_publish;
pub mod docgen;
pub mod extended_checks;
pub mod prover;
mod release_bundle;

pub use aptos_framework_natives::{unzip_metadata, unzip_metadata_str};
pub use aptos_release_bundle::*;
use flate2::{write::GzEncoder, Compression};
pub use release_bundle::*;
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
