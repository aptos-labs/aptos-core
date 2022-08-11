// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod aptos;
pub use aptos::*;

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

use std::path::PathBuf;

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative.into())
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
            .join(ReleaseTarget::Current.file_name());
        ReleaseTarget::Current
            .create_release(Some(actual_name.clone()))
            .unwrap();
        let actual = ReleaseBundle::read(actual_name).unwrap();
        assert!(
            crate::current_release_bundle() == &actual,
            "Generated framework artifacts out-of-date. Please `cargo run -p framework -- release`"
        );
    }
}
