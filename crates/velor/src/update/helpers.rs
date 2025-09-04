// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

/// Some functionality of the Velor CLI relies on some additional binaries. This is
/// where we install them by default. These paths align with the installation script,
/// which is generally how the Linux and Windows users install the CLI.
pub fn get_additional_binaries_dir() -> PathBuf {
    #[cfg(windows)]
    {
        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "".into());
        PathBuf::from(home_dir).join(".velorcli/bin")
    }

    #[cfg(not(windows))]
    {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "".into());
        PathBuf::from(home_dir).join(".local/bin")
    }
}
