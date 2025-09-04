// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{NamedAddress, PackageName};
use move_core_types::account_address::AccountAddress;
use move_model::metadata::LanguageVersion;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Debug, Display},
    path::PathBuf,
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
    pub addresses: BTreeMap<NamedAddress, AddressAssignment>,

    /// Dev-only named address bindings defined by the package.
    /// Being dev-only means the bindings are only active in unit tests, not when being compiled regularly.
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
    /// Name of the Move package.
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
#[derive(Clone, Eq, PartialEq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

/// Represents either an unspecified or numerical address.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AddressAssignment {
    /// Unspecified address (`"_"` in `Move.toml`).
    Unspecified,

    /// A specific numerical address.
    Numerical(AccountAddress),
}

/// Build options defined in the `[build]` section of `Move.toml`.
#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildOptions {
    /// Version of the source language used to compile the package.
    ///
    /// TODO: This is currently unused. The exact functionality will be determined once
    ///       we hook it up to the compiler.
    pub language_version: Option<LanguageVersion>,
}

/// Represents a dependency entry in `[dependencies]` or `[dev-dependencies]`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Dependency {
    /// Optional version requirement for the dependency.
    /// Not in use by the package resolver, yet.
    ///
    /// Note: This is intended to be a build-time constraint, and it alone does not guarantee
    ///       that your program will be linked to the specific version of the dependency
    ///       during execution on-chain.
    version: Option<Version>,

    /// Location of the dependency: local, git, or velor (on-chain).
    pub location: PackageLocation,
}

/// Location of a package dependency.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PackageLocation {
    /// Refers to a package stored in the local file system.
    Local { path: PathBuf },

    /// Refers to a package stored in a git repository.
    Git {
        /// URL to the Git repository.
        url: Url,
        /// Optional Git revision to pin the dependency to.
        /// This can be a commit hash, a branch name or a tag name.
        rev: Option<String>,
        /// Optional subdirectory within the Git repository.
        subdir: Option<String>,
    },

    /// Refers to a package published on-chain.
    ///
    // TODO: The current design is tentative. There are issues we plan to resolve later:
    //       - Leaky abstraction -- can we still want to maintain clear Move/Velor separation?
    //       - Replacing `String` w/ more specific data structures
    //         - `node_url`: Should accept both URL and known network names (e.g. "mainnet")
    //         - `package_addr`: May accept both numerical and named addresses
    Velor {
        /// URL to the Velor full-node connected to the network where the package is published.
        node_url: String,

        /// Address of the published package.
        package_addr: AccountAddress,
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

impl<'de> Deserialize<'de> for AddressAssignment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "_" => Self::Unspecified,
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

    local: Option<PathBuf>,

    git: Option<Url>,
    rev: Option<String>,
    subdir: Option<String>,

    velor: Option<String>,
    address: Option<AccountAddress>,
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

        error_on_unneeded_fields!(address, velor);

        let location = match (raw.local, raw.git, raw.velor) {
            (Some(path), None, None) => PackageLocation::Local { path },
            (None, Some(url), None) => PackageLocation::Git {
                url,
                rev: raw.rev,
                subdir: raw.subdir,
            },
            (None, None, Some(node_url)) => match raw.address {
                Some(package_addr) => PackageLocation::Velor {
                    node_url,
                    package_addr,
                },
                None => {
                    return Err(serde::de::Error::custom(
                        "missing field \"address\" for velor dependency",
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
        let s = format!("{}", self);
        s.serialize(serializer)
    }
}

impl Serialize for AddressAssignment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AddressAssignment::Unspecified => "_".serialize(serializer),
            AddressAssignment::Numerical(addr) => addr.serialize(serializer),
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
            PackageLocation::Velor {
                node_url,
                package_addr,
            } => {
                raw.velor = Some(node_url);
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
/***************************************************************************************************
 * Debug/Display
 *
 **************************************************************************************************/
impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}.{}.{}\"", self.major, self.minor, self.patch)
    }
}
