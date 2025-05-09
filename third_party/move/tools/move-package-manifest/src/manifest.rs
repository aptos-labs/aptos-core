// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// TODO: Check symbols

/***************************************************************************************************
 * Manifest Definition
 *
 **************************************************************************************************/
#[derive(Default, Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub package: PackageInfo,

    #[serde(default)]
    pub addresses: BTreeMap<String, AddressAssignment>,

    #[serde(default, rename = "dev-addresses")]
    pub dev_addresses: BTreeMap<String, AccountAddress>,

    pub build: Option<BuildInfo>,

    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,

    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, Dependency>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageInfo {
    /// Name of the Move package.
    pub name: String,
    pub version: Version,

    #[serde(default)]
    pub authors: BTreeSet<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AddressAssignment {
    Wildcard,
    Numerical(AccountAddress),
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildInfo {
    pub language_version: Option<Version>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Dependency {
    version: Option<Version>,

    // FIXME: addr subst
    addr_subst: BTreeMap<String, String>,

    location: PackageLocation,
    // REVISIT: digest
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PackageLocation {
    Local {
        path: String,
    },
    Git {
        url: String,
        rev: Option<String>,
        subdir: Option<String>,
    },
    // REVISIT: Potentially leaky abstraction -- do we still want this to be platform agnostic?
    Aptos {
        node_url: String,
        package_addr: String,
    },
}

/***************************************************************************************************
 * Custom Deserializer Implementations
 *
 **************************************************************************************************/
impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let make_err = || serde::de::Error::custom("Invalid version format");

        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(make_err());
        }
        let major = parts[0].parse().map_err(|_| make_err())?;
        let minor = parts[1].parse().map_err(|_| make_err())?;
        let patch = parts[2].parse().map_err(|_| make_err())?;

        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

impl<'de> Deserialize<'de> for AddressAssignment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "_" => Self::Wildcard,
            _ => Self::Numerical(
                AccountAddress::from_hex_literal(&s)
                    .map_err(|_| serde::de::Error::custom("Invalid account address"))?,
            ),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDependency {
    version: Option<Version>,

    #[serde(default)]
    addr_subst: BTreeMap<String, String>,

    local: Option<String>,

    git: Option<String>,
    rev: Option<String>,
    subdir: Option<String>,

    aptos: Option<String>,
    address: Option<String>,
}

impl<'de> Deserialize<'de> for Dependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawDependency::deserialize(deserializer)?;

        let location = match (raw.local, raw.git, raw.aptos) {
            (Some(path), None, None) => PackageLocation::Local { path },
            (None, Some(url), None) => PackageLocation::Git {
                url,
                rev: raw.rev,
                subdir: raw.subdir,
            },
            (None, None, Some(node_url)) => match raw.address {
                Some(package_addr) => PackageLocation::Aptos {
                    node_url,
                    package_addr,
                },
                None => {
                    return Err(serde::de::Error::custom(
                        "missing field \"address\" for aptos dependency",
                    ))
                },
            },
            _ => {
                return Err(serde::de::Error::custom(
                    "dependency cannot have have multiple locations",
                ));
            },
        };

        Ok(Dependency {
            version: raw.version,
            addr_subst: raw.addr_subst,
            location,
        })
    }
}

/***************************************************************************************************
 * Default Values
 *
 **************************************************************************************************/
impl Default for Version {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
        }
    }
}

impl Default for PackageInfo {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            version: Default::default(),
            authors: BTreeSet::new(),
            license: None,
        }
    }
}
