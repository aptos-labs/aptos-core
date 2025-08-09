// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    docgen::DocgenOptions, path_in_crate, release_builder::RELEASE_BUNDLE_EXTENSION,
    release_bundle::ReleaseBundle, BuildOptions, ReleaseOptions,
};
use clap::ValueEnum;
use move_command_line_common::address::NumericalAddress;
use once_cell::sync::Lazy;
use std::{collections::BTreeMap, fmt::Display, path::PathBuf, str::FromStr};

// ===============================================================================================
// Release Targets

/// Represents the available release targets. `Current` is in sync with the current client branch,
/// which is ensured by tests.
#[derive(ValueEnum, Clone, Copy, Debug)]
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
    /// Last element is a boolean on whether to use set_latest_language while building it.
    pub fn packages(self) -> Vec<(&'static str, Option<&'static str>, bool)> {
        let result = vec![
            ("move-stdlib", None, false),
            ("aptos-stdlib", None, false),
            (
                "aptos-framework",
                Some("cached-packages/src/aptos_framework_sdk_builder.rs"),
                true,
            ),
            (
                "aptos-token",
                Some("cached-packages/src/aptos_token_sdk_builder.rs"),
                true,
            ),
            (
                "aptos-token-objects",
                Some("cached-packages/src/aptos_token_objects_sdk_builder.rs"),
                true,
            ),
            ("aptos-experimental", None, true),
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

    pub fn create_release_options(self, with_srcs: bool, out: Option<PathBuf>) -> ReleaseOptions {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let packages = self
            .packages()
            .into_iter()
            .map(|(path, binding_path, latest_language)| {
                (
                    crate_dir.join(path),
                    binding_path.unwrap_or("").to_owned(),
                    latest_language,
                )
            })
            .collect::<Vec<_>>();
        let package_use_latest_language = packages
            .iter()
            .map(|(_, _, latest_language)| *latest_language)
            .collect();
        ReleaseOptions {
            build_options: BuildOptions {
                with_srcs,
                with_abis: true,
                with_docs: true,
                docgen_options: Some(DocgenOptions {
                    include_impl: true,
                    include_specs: true,
                    specs_inlined: false,
                    include_dep_diagram: false,
                    collapsed_sections: true,
                    landing_page_template: Some("doc_template/overview.md".to_string()),
                    references_file: Some("doc_template/references.md".to_string()),
                    output_format: None,
                }),
                skip_fetch_latest_git_deps: true,
                ..BuildOptions::default()
            },
            packages: packages
                .iter()
                .map(|(path, _, _)| path.to_owned())
                .collect(),
            rust_bindings: packages
                .into_iter()
                .map(|(_, binding, _)| {
                    if !binding.is_empty() {
                        crate_dir.join(binding).display().to_string()
                    } else {
                        binding
                    }
                })
                .collect(),
            package_use_latest_language,
            output: if let Some(path) = out {
                path
            } else {
                // Place in current directory
                PathBuf::from(self.file_name())
            },
        }
    }

    pub fn create_release(self, with_srcs: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
        let options = self.create_release_options(with_srcs, out);
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
    let three = NumericalAddress::parse_str("0x3").unwrap();
    let four = NumericalAddress::parse_str("0x4").unwrap();
    let seven = NumericalAddress::parse_str("0x7").unwrap();
    let ten = NumericalAddress::parse_str("0xA").unwrap();
    let resources = NumericalAddress::parse_str("0xA550C18").unwrap();
    result.insert("std".to_owned(), one);
    result.insert("aptos_std".to_owned(), one);
    result.insert("aptos_framework".to_owned(), one);
    result.insert("aptos_token".to_owned(), three);
    result.insert("aptos_token_objects".to_owned(), four);
    result.insert("aptos_experimental".to_owned(), seven);
    result.insert("aptos_fungible_asset".to_owned(), ten);
    result.insert("core_resources".to_owned(), resources);
    result.insert("vm".to_owned(), zero);
    result.insert("vm_reserved".to_owned(), zero);
    result
});

pub fn named_addresses() -> &'static BTreeMap<String, NumericalAddress> {
    &NAMED_ADDRESSES
}
