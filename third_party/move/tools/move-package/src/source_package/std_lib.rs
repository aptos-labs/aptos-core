// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::source_package::parsed_manifest::{Dependency, GitInfo};
use move_command_line_common::env::MOVE_HOME;
use move_symbol_pool::symbol::Symbol;
use std::path::PathBuf;

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
        let local =
            PathBuf::from(MOVE_HOME.clone()).join(format!("{}_{}", self.as_str(), version.rev()));
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
pub enum StdVersion {
    Mainnet,
    Testnet,
    Dev,
}

impl StdVersion {
    /// Returns the rev name of the standard library version.
    pub fn rev(&self) -> &'static str {
        match self {
            StdVersion::Mainnet => "mainnet",
            StdVersion::Testnet => "testnet",
            StdVersion::Dev => "dev",
        }
    }

    /// Returns the standard library version from the given rev name, or `None` if the string is not a standard library version.
    pub fn from_rev(version: &str) -> Option<StdVersion> {
        match version {
            "mainnet" => Some(Self::Mainnet),
            "testnet" => Some(Self::Testnet),
            "dev" => Some(Self::Dev),
            _ => None,
        }
    }
}
