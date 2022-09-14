// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::release_builder::RELEASE_BUNDLE_EXTENSION;
use crate::release_bundle::ReleaseBundle;
use crate::{path_in_crate, BuildOptions, ReleaseOptions};
use clap::ArgEnum;
use move_deps::move_command_line_common::address::NumericalAddress;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

// ===============================================================================================
// Release Targets

/// Represents the available release targets. `Current` is in sync with the current client branch,
/// which is ensured by tests.
#[derive(ArgEnum, Clone, Copy, Debug)]
pub enum ReleaseTarget {
    Head,
    Devnet,
    Testnet,
    Mainnet,
}

impl Display for ReleaseTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ReleaseTarget::Head => "head",
            ReleaseTarget::Devnet => "devnet",
            ReleaseTarget::Testnet => "testnet",
            ReleaseTarget::Mainnet => "mainnet",
        };
        write!(f, "{}", str)
    }
}

impl FromStr for ReleaseTarget {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "head" => Ok(ReleaseTarget::Head),
            "devnet" => Ok(ReleaseTarget::Devnet),
            "testnet" => Ok(ReleaseTarget::Testnet),
            "mainnet" => Ok(ReleaseTarget::Mainnet),
            _ => Err("Invalid target. Valid values are: head, devnet, testnet, mainnet"),
        }
    }
}

impl ReleaseTarget {
    /// Returns the package directories (relative to `framework`), in the order
    /// they need to be published, as well as an optional path to the file where
    /// rust bindings generated from the package should be stored.
    pub fn packages(self) -> Vec<(&'static str, Option<&'static str>)> {
        let result = vec![
            ("move-stdlib", None),
            ("aptos-stdlib", None),
            (
                "aptos-framework",
                Some("cached-packages/src/aptos_framework_sdk_builder.rs"),
            ),
            (
                "aptos-token",
                Some("cached-packages/src/aptos_token_sdk_builder.rs"),
            ),
        ];
        // Currently we don't have experimental packages only included in particular targets.
        result
    }

    /// Returns the file name under which this particular target's release buundle is stored.
    /// For example, for `Head` the file name will be `head.mrb`.
    pub fn file_name(self) -> String {
        format!("{}.{}", self, RELEASE_BUNDLE_EXTENSION)
    }

    /// Loads the release bundle for this particular target.
    pub fn load_bundle(self) -> anyhow::Result<ReleaseBundle> {
        let path = path_in_crate("releases").join(self.file_name());
        ReleaseBundle::read(path)
    }

    pub fn create_release(self, out: Option<PathBuf>) -> anyhow::Result<()> {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let packages = self
            .packages()
            .into_iter()
            .map(|(path, binding_path)| {
                (crate_dir.join(path), binding_path.unwrap_or("").to_owned())
            })
            .collect::<Vec<_>>();
        let options = ReleaseOptions {
            build_options: BuildOptions {
                with_srcs: true,
                with_abis: true,
                with_source_maps: false,
                with_error_map: true,
                named_addresses: Default::default(),
                install_dir: None,
            },
            packages: packages.iter().map(|(path, _)| path.to_owned()).collect(),
            rust_bindings: packages
                .into_iter()
                .map(|(_, binding)| {
                    if !binding.is_empty() {
                        crate_dir.join(binding).display().to_string()
                    } else {
                        binding
                    }
                })
                .collect(),
            output: if let Some(path) = out {
                path
            } else {
                // Place in current directory
                PathBuf::from(self.file_name())
            },
        };

        #[cfg(unix)]
        {
            options.create_release()
        }
        #[cfg(windows)]
        {
            // Windows requires to set the stack because the package compiler puts too much on the
            // stack for the default size.  A quick internet search has shown the new thread with
            // a custom stack size is the easiest course of action.
            const STACK_SIZE: usize = 4 * 1024 * 1024;
            let child_thread = std::thread::Builder::new()
                .name("Framework-release".to_string())
                .stack_size(STACK_SIZE)
                .spawn(|| options.create_release())
                .expect("Expected to spawn release thread");
            child_thread
                .join()
                .expect("Expected to join release thread")
        }
    }
}

// ===============================================================================================
// Legacy Named Addresses

// Some older Move tests work directly on sources, skipping the package system. For those
// we define the relevant address aliases here.

static NAMED_ADDRESSES: Lazy<BTreeMap<String, NumericalAddress>> = Lazy::new(|| {
    let mut result = BTreeMap::new();
    let zero = NumericalAddress::parse_str("0x0").unwrap();
    let one = NumericalAddress::parse_str("0x1").unwrap();
    let two = NumericalAddress::parse_str("0x2").unwrap();
    let resources = NumericalAddress::parse_str("0xA550C18").unwrap();
    result.insert("std".to_owned(), one);
    result.insert("aptos_std".to_owned(), one);
    result.insert("aptos_framework".to_owned(), one);
    result.insert("aptos_token".to_owned(), two);
    result.insert("core_resources".to_owned(), resources);
    result.insert("vm_reserved".to_owned(), zero);
    result
});

pub fn named_addresses() -> &'static BTreeMap<String, NumericalAddress> {
    &NAMED_ADDRESSES
}
