// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use move_symbol_pool::symbol::Symbol;
use std::{collections::BTreeMap, fmt, fmt::Debug, path::PathBuf};

pub type NamedAddress = Symbol;
pub type PackageName = Symbol;
pub type FileName = Symbol;
pub type PackageDigest = Symbol;

pub type AddressDeclarations = BTreeMap<NamedAddress, Option<AccountAddress>>;
pub type DevAddressDeclarations = BTreeMap<NamedAddress, AccountAddress>;
pub type Version = (u64, u64, u64);
pub type Dependencies = BTreeMap<PackageName, Dependency>;
pub type Substitution = BTreeMap<NamedAddress, SubstOrRename>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SourceManifest {
    pub package: PackageInfo,
    pub addresses: Option<AddressDeclarations>,
    pub dev_address_assignments: Option<DevAddressDeclarations>,
    pub build: Option<BuildInfo>,
    pub dependencies: Dependencies,
    pub dev_dependencies: Dependencies,
}

impl fmt::Display for SourceManifest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "[package]")?;
        writeln!(f, "{}", self.package)?;
        writeln!(f, "[addresses]")?;
        if let Some(address_map) = &self.addresses {
            for (named, addr_opt) in address_map.iter() {
                if let Some(addr) = addr_opt {
                    writeln!(f, "{} = \"{}\"", named.as_str(), addr)?;
                } else {
                    writeln!(f, "{} = \"_\"", named.as_str())?;
                }
            }
        }
        writeln!(f, "[dependencies]")?;
        for (package_name, dep) in self.dependencies.clone().into_iter() {
            writeln!(f, "{} = {{ local = {} }}", package_name, dep)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PackageInfo {
    pub name: PackageName,
    pub version: Version,
    pub authors: Vec<Symbol>,
    pub license: Option<Symbol>,
    pub custom_properties: BTreeMap<Symbol, String>,
}

impl fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "name = \"{}\"", self.name.as_str())?;
        writeln!(
            f,
            "version = \"{}.{}.{}\"",
            self.version.0, self.version.1, self.version.2
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Dependency {
    pub local: PathBuf,
    pub subst: Option<Substitution>,
    pub version: Option<Version>,
    pub digest: Option<PackageDigest>,
    pub git_info: Option<GitInfo>,
    pub node_info: Option<CustomDepInfo>,
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.local.as_os_str())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GitInfo {
    /// The git clone url to download from
    pub git_url: Symbol,
    /// The git revision, AKA, a commit SHA
    pub git_rev: Symbol,
    /// The path under this repo where the move package can be found -- e.g.,
    /// 'language/move-stdlib`
    pub subdir: PathBuf,
    /// Where the git repo is downloaded to.
    pub download_to: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CustomDepInfo {
    /// The url of the node to download from
    pub node_url: Symbol,
    /// The address where the package is published. The representation depends
    /// on the registered node resolver.
    pub package_address: Symbol,
    /// The address where the package is published.
    pub package_name: Symbol,
    /// Where the package is downloaded to.
    pub download_to: PathBuf,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct BuildInfo {
    pub language_version: Option<Version>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SubstOrRename {
    RenameFrom(NamedAddress),
    Assign(AccountAddress),
}
