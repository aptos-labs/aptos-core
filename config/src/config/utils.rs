// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use cfg_if::cfg_if;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct RootPath {
    root_path: PathBuf,
}

impl RootPath {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let root_path = if let Some(parent) = path.as_ref().parent() {
            parent.to_path_buf()
        } else {
            PathBuf::from("")
        };

        Self { root_path }
    }

    /// This function assumes that the path is already a directory
    pub fn new_path<P: AsRef<Path>>(path: P) -> Self {
        let root_path = path.as_ref().to_path_buf();
        Self { root_path }
    }

    /// This adds a full path when loading / storing if one is not specified
    pub fn full_path(&self, file_path: &Path) -> PathBuf {
        if file_path.is_relative() {
            self.root_path.join(file_path)
        } else {
            file_path.to_path_buf()
        }
    }
}

/// Returns true iff failpoints are enabled
pub fn are_failpoints_enabled() -> bool {
    cfg_if! {
        if #[cfg(feature = "failpoints")] {
            true
        } else {
            false
        }
    }
}

/// Returns the name of the given config type
pub fn get_config_name<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>()
        .split("::")
        .last()
        .unwrap_or("UnknownConfig")
}

/// Returns true iff the tokio-console feature is enabled
pub fn is_tokio_console_enabled() -> bool {
    cfg_if! {
        if #[cfg(feature = "tokio-console")] {
            true
        } else {
            false
        }
    }
}
