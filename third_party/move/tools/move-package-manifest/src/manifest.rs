// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{NamedAddress, PackageName};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};
use url::Url;

/***************************************************************************************************
 * Manifest Definition
 *
 **************************************************************************************************/
/// Represents the full parsed contents of a `Move.toml` manifest file.
#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    /// Metadata about the package itself, such as package name etc.
    pub package: PackageMetadata,

    /// Named address mappings defined by the package.
    #[serde(default)]
    pub addresses: BTreeMap<NamedAddress, AddressOrWildcard>,

    /// Dev-only named address defined by the package.
    #[serde(default, rename = "dev-addresses")]
    pub dev_addresses: BTreeMap<NamedAddress, AccountAddress>,

    /// Build options.
    pub build: Option<BuildOptions>,

    /// Regular (non-dev) package dependencies.
    #[serde(default)]
    pub dependencies: BTreeMap<PackageName, Dependency>,

    /// Dev-only package dependencies.
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: BTreeMap<PackageName, Dependency>,
}

/// Metadata defined in the `[package]` section of `Move.toml`.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMetadata {
    /// Named of the Move package.
    pub name: PackageName,

    /// Version of the package.
    pub version: Version,

    /// List of authors.
    #[serde(default, deserialize_with = "deserialize_unique_vec")]
    pub authors: Vec<String>,

    /// Optional license string for the package.
    pub license: Option<String>,

    /// Optional upgrade policy for the package.
    pub upgrade_policy: Option<UpgradePolicy>,
}

/// Upgrade policy for a Move package, controlling whether upgrades are allowed and if so, what rules to follow.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum UpgradePolicy {
    /// Indicates that upgrades are allowed, but must be compatible with the current version.
    #[serde(rename = "compatible")]
    Compatible,

    /// Indicates that the package is immutable and should not be upgraded.
    #[serde(rename = "immutable")]
    Immutable,
}

/// A semantic version consisting of major, minor, and patch components.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

/// Represents either a wildcard or a numerical address.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AddressOrWildcard {
    /// Wildcard address (`"_"` in `Move.toml`).
    Wildcard,

    /// A specific numerical address.
    Numerical(AccountAddress),
}

/// Build options defined in the `[build]` section of `Move.toml`.
#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildOptions {
    pub language_version: Option<Version>,
}

/// Represents a dependency entry in `[dependencies]` or `[dev-dependencies]`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Dependency {
    /// Optional version requirement for the dependency.
    version: Option<Version>,

    /// Location of the dependency: local, git, or aptos (on-chain).
    location: PackageLocation,
}

/// Location of a package dependency.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PackageLocation {
    /// Refers to a package stored in the local file system.
    Local { path: String },

    /// Refers to a package stored in a git repository.
    Git {
        /// URL to the Git repository.
        url: Url,
        /// Optional Git revision to pin the dependency to.
        rev: Option<String>,
        /// Optional subdirectory within the Git repository.
        subdir: Option<String>,
    },

    /// Refers to a package published on-chain.
    // REVISIT: Potentially leaky abstraction -- do we still want this to be platform agnostic?
    Aptos {
        /// URL to the Aptos full-node connected to the network where the package is published.
        node_url: String,

        /// Address of the published package.
        package_addr: String,
    },
}

/***************************************************************************************************
 * Custom Deserializer Implementations
 *
 **************************************************************************************************/
pub fn deserialize_unique_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct UniqueVecVisitor;

    impl<'de> serde::de::Visitor<'de> for UniqueVecVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a list of unique strings")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut seen = BTreeSet::new();
            let mut values = Vec::new();

            while let Some(value) = seq.next_element::<String>()? {
                if !seen.insert(value.clone()) {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate entry: {}",
                        value
                    )));
                }
                values.push(value);
            }

            Ok(values)
        }
    }

    deserializer.deserialize_seq(UniqueVecVisitor)
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let make_err = || {
            serde::de::Error::custom(
                "Invalid version -- a version in the format 'x.y.z' (e.g. '1.2.3')",
            )
        };

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

impl<'de> Deserialize<'de> for AddressOrWildcard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "_" => Self::Wildcard,
            _ => Self::Numerical(
                AccountAddress::from_str(&s)
                    .map_err(|_| serde::de::Error::custom("Invalid account address"))?,
            ),
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDependency {
    version: Option<Version>,

    local: Option<String>,

    git: Option<Url>,
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

        macro_rules! error_on_unneeded_fields {
            ($field_name:ident, $tag_name:ident) => {
                if (raw.$field_name.is_some() && raw.$tag_name.is_none()) {
                    return Err(serde::de::Error::custom(format!(
                        "redundant field \"{}\" -- only needed for \"{}\" dependencies",
                        stringify!($field_name),
                        stringify!($tag_name),
                    )));
                }
            };
        }

        error_on_unneeded_fields!(rev, git);
        error_on_unneeded_fields!(subdir, git);

        error_on_unneeded_fields!(address, aptos);

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
            (None, None, None) => {
                return Err(serde::de::Error::custom(
                    "no package location specified for dependency",
                ));
            },
            _ => {
                return Err(serde::de::Error::custom(
                    "dependency cannot have have multiple locations",
                ));
            },
        };

        Ok(Dependency {
            version: raw.version,
            location,
        })
    }
}

/***************************************************************************************************
 * Custom Serializer Implementations
 *
 **************************************************************************************************/
impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}.{}.{}", self.major, self.minor, self.patch);
        s.serialize(serializer)
    }
}

impl Serialize for AddressOrWildcard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AddressOrWildcard::Wildcard => "_".serialize(serializer),
            AddressOrWildcard::Numerical(addr) => addr.serialize(serializer),
        }
    }
}

impl Dependency {
    fn into_raw(self) -> RawDependency {
        let mut raw = RawDependency {
            version: self.version,
            ..Default::default()
        };

        match self.location {
            PackageLocation::Local { path } => raw.local = Some(path),
            PackageLocation::Git { url, rev, subdir } => {
                raw.git = Some(url);
                raw.rev = rev;
                raw.subdir = subdir;
            },
            PackageLocation::Aptos {
                node_url,
                package_addr,
            } => {
                raw.aptos = Some(node_url);
                raw.address = Some(package_addr);
            },
        }

        raw
    }

    fn to_raw(&self) -> RawDependency {
        self.clone().into_raw()
    }
}

impl Serialize for Dependency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_raw().serialize(serializer)
    }
}

/***************************************************************************************************
 * Default Values
 *
 **************************************************************************************************/
#[allow(clippy::derivable_impls)]
impl Default for Version {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
        }
    }
}

impl Default for PackageMetadata {
    fn default() -> Self {
        Self {
            name: PackageName::new("some_package").unwrap(),
            version: Default::default(),
            authors: vec![],
            license: None,
            upgrade_policy: None,
        }
    }
}
