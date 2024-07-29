// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    manifest_parser::git_repo_cache_path,
    source_package::parsed_manifest::{Dependency, GitInfo},
};
use clap::ValueEnum;
use move_symbol_pool::symbol::Symbol;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::PathBuf};

/// Represents a standard library.
pub enum StdLib {
    AptosFramework,
    AptosStdlib,
    MoveStdlib,
}

impl StdLib {
    /// The well-known git URL for the standard library.
    const STD_GIT_URL: &'static str = "https://github.com/aptos-labs/aptos-core.git";

    /// Returns the dependency for the standard library with the given version.
    pub fn dependency(&self, version: &StdVersion) -> Dependency {
        let local = git_repo_cache_path(Self::STD_GIT_URL, version.rev());
        Dependency {
            local: local.join(self.sub_dir()),
            subst: None,
            version: None,
            digest: None,
            git_info: Some(GitInfo {
                git_url: Symbol::from(StdLib::STD_GIT_URL),
                git_rev: Symbol::from(version.rev()),
                subdir: PathBuf::from(self.sub_dir()),
                download_to: local,
            }),
            node_info: None,
        }
    }

    /// Returns the name of the standard library.
    pub fn as_str(&self) -> &'static str {
        match self {
            StdLib::AptosFramework => "AptosFramework",
            StdLib::AptosStdlib => "AptosStdlib",
            StdLib::MoveStdlib => "MoveStdlib",
        }
    }

    /// Returns the standard library from the given package name, or `None` if the package name is not a standard library.
    pub fn from_package_name(package_name: Symbol) -> Option<StdLib> {
        match package_name.as_str() {
            "AptosFramework" => Some(StdLib::AptosFramework),
            "AptosStdlib" => Some(StdLib::AptosStdlib),
            "MoveStdlib" => Some(StdLib::MoveStdlib),
            _ => None,
        }
    }

    /// Returns the subdirectory of the standard library in the git repository.
    fn sub_dir(&self) -> &'static str {
        match self {
            StdLib::AptosFramework => "aptos-move/framework/aptos-framework",
            StdLib::AptosStdlib => "aptos-move/framework/aptos-stdlib",
            StdLib::MoveStdlib => "aptos-move/framework/move-stdlib",
        }
    }
}

/// Represents a standard library version.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum StdVersion {
    Mainnet,
    Testnet,
    Devnet,
}

impl StdVersion {
    const DEVNET: &'static str = "devnet";
    const MAINNET: &'static str = "mainnet";
    const TESTNET: &'static str = "testnet";

    /// Returns the rev name of the standard library version.
    pub fn rev(&self) -> &'static str {
        match self {
            StdVersion::Mainnet => StdVersion::MAINNET,
            StdVersion::Testnet => StdVersion::TESTNET,
            StdVersion::Devnet => StdVersion::DEVNET,
        }
    }

    /// Returns the standard library version from the given rev name, or `None` if the string is not a standard library version.
    pub fn from_rev(version: &str) -> Option<StdVersion> {
        match version {
            StdVersion::MAINNET => Some(Self::Mainnet),
            StdVersion::TESTNET => Some(Self::Testnet),
            StdVersion::DEVNET => Some(Self::Devnet),
            _ => None,
        }
    }
}

impl Display for StdVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.rev())
    }
}
