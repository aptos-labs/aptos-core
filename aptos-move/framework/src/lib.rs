// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod aptos;

pub use aptos::*;
use std::io::{Read, Write};

mod generated;
pub use generated::aptos_framework_sdk_builder;
pub use generated::aptos_stdlib;
pub use generated::aptos_token_sdk_builder;

mod built_package;
pub use built_package::*;

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

pub fn zip_metadata(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::best());
    gz.write_all(data)?;
    let result = gz.finish()?;
    Ok(result)
}

pub fn unzip_metadata(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut gz = GzDecoder::new(data);
    let mut result = vec![];
    gz.read_to_end(&mut result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::{ReleaseBundle, ReleaseTarget};

    #[test]
    fn current_release_bundle_up_to_date() {
        let tempdir = tempfile::tempdir().unwrap();
        let actual_name = tempdir
            .path()
            .to_path_buf()
            .join(ReleaseTarget::Head.file_name());
        ReleaseTarget::Head
            .create_release(true, Some(actual_name.clone()))
            .unwrap();
        let actual = ReleaseBundle::read(actual_name).unwrap();
        assert!(
            crate::head_release_bundle() == &actual,
            "Generated framework artifacts out-of-date. Please `cargo run -p framework -- release`"
        );
    }
}
