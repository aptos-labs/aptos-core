// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use legacy_move_compiler::shared::LanguageVersion as CompilerLanguageVersion;
use move_binary_format::file_format_common::{VERSION_DEFAULT, VERSION_DEFAULT_LANG_V2};
use move_command_line_common::env;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Display, Formatter},
    str::FromStr,
};

const UNSTABLE_MARKER: &str = "-unstable";
// 2.2, even though stable, produces several warnings in the frameworks which we first need to fix
// before we can make it the default
pub const LATEST_STABLE_LANGUAGE_VERSION_VALUE: LanguageVersion = LanguageVersion::V2_2;
pub const LATEST_STABLE_COMPILER_VERSION_VALUE: CompilerVersion = CompilerVersion::V2_0;
pub const LATEST_STABLE_LANGUAGE_VERSION: &str = LATEST_STABLE_LANGUAGE_VERSION_VALUE.to_str();
pub const LATEST_STABLE_COMPILER_VERSION: &str = LATEST_STABLE_COMPILER_VERSION_VALUE.to_str();

pub static COMPILATION_METADATA_KEY: &[u8] = "compilation_metadata".as_bytes();

// ================================================================================'
// Metadata for compilation result (WORK IN PROGRESS)

/// Metadata about a compilation result. To maintain serialization
/// stability, this uses a free-form string to represent compiler version
/// and language version, which is interpreted by the `CompilerVersion`
/// and `LanguageVersion` types. This allows to always successfully
/// deserialize the metadata (even older code with newer data), and leave it
/// up to the program how to deal with decoding errors.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompilationMetadata {
    /// A flag indicating whether, at time of creation, the compilation
    /// result was considered as unstable. Unstable code may have restrictions
    /// for deployment on production networks. This flag is true if either the
    /// compiler or language versions are unstable.
    pub unstable: bool,
    /// The version of the compiler, as a string. See
    /// `CompilationVersion::from_str` for supported version strings.
    pub compiler_version: String,
    /// The version of the language, as a string. See
    /// `LanguageVersion::from_str` for supported version strings.
    pub language_version: String,
}

impl CompilationMetadata {
    pub fn new(compiler_version: CompilerVersion, language_version: LanguageVersion) -> Self {
        Self {
            compiler_version: compiler_version.to_string(),
            language_version: language_version.to_string(),
            unstable: compiler_version.unstable() || language_version.unstable(),
        }
    }

    pub fn compiler_version(&self) -> anyhow::Result<CompilerVersion> {
        CompilerVersion::from_str(&self.compiler_version)
    }

    pub fn language_version(&self) -> anyhow::Result<LanguageVersion> {
        LanguageVersion::from_str(&self.language_version)
    }

    /// Returns true of the compilation was created as unstable.
    pub fn created_as_unstable(&self) -> bool {
        self.unstable
    }
}

// ================================================================================'
// Compiler Version

/// Represents a compiler version.
///
/// The versioning scheme is `major.minor`, where for `major = 1` we do not
/// distinguish a minor version. A major version change represents
/// a different/largely refactored compiler. This we have versions `1, 2.0, 2.1, 2.2, .., `.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
pub enum CompilerVersion {
    /// The legacy v1 Move compiler, no longer supported.
    V1,
    /// The v2 compiler, starting with 2.0-unstable. Each new released version of the compiler
    /// should get an enum entry here.
    V2_0,
    /// Upcoming Version 2.1 of the compiler
    V2_1,
}

impl Default for CompilerVersion {
    fn default() -> Self {
        Self::latest_stable()
    }
}

impl FromStr for CompilerVersion {
    type Err = anyhow::Error;

    /// Parses a compiler version. If the caller only provides a major
    /// version number, this chooses the latest stable minor version (if any).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Strip unstable marker as it is not relevant for parsing.
        let s1 = s.replace(UNSTABLE_MARKER, "");
        match s1.as_str() {
            // For legacy reasons, also support v1 and v2
            "1" | "v1" => Ok(Self::V1),
            "2" | "v2" | "2.0" => Ok(Self::V2_0),
            "2.1" => Ok(Self::V2_1),
            _ => bail!(
                "unrecognized compiler version `{}` (supported versions: `1`, `2`, `2.0`)",
                s
            ),
        }
    }
}

impl Display for CompilerVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            match self {
                CompilerVersion::V1 => "1",
                CompilerVersion::V2_0 => "2.0",
                CompilerVersion::V2_1 => "2.1",
            },
            if self.unstable() { UNSTABLE_MARKER } else { "" }
        )
    }
}

impl CompilerVersion {
    /// Return true if this is a stable compiler version. A non-stable version
    /// should not be allowed on production networks.
    pub fn unstable(self) -> bool {
        match self {
            CompilerVersion::V1 => false,
            CompilerVersion::V2_0 => false,
            CompilerVersion::V2_1 => true,
        }
    }

    /// The latest compiler version.
    pub const fn latest() -> Self {
        CompilerVersion::V2_1
    }

    /// The latest stable compiler version.
    pub const fn latest_stable() -> Self {
        LATEST_STABLE_COMPILER_VERSION_VALUE
    }

    /// Check whether the compiler version supports the given language version,
    /// generates an error if not.
    pub fn check_language_support(self, _version: LanguageVersion) -> anyhow::Result<()> {
        match self {
            CompilerVersion::V1 => {
                bail!("compiler v1 is no longer supported")
            },
            _ => Ok(()),
        }
    }

    pub const fn to_str(&self) -> &'static str {
        match self {
            CompilerVersion::V1 => "1",
            CompilerVersion::V2_0 => "2.0",
            CompilerVersion::V2_1 => "2.1",
        }
    }
}

// ================================================================================'
// Language Version

/// Represents a language version.
///
/// The versioning scheme is `major.minor`, where for `major = 1` we do not
/// distinguish a minor version. This we have versions `1, 2.0, 2.1, .., 3.0, 3.1, ..`.
/// Typically, a major version change is given with an addition of larger new language
/// features. There are no breaking changes expected with major version changes,
/// however, there maybe some isolated exceptions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum LanguageVersion {
    /// The version of Move at around the genesis of the Aptos network end
    /// of '22. This is the original Diem Move plus the extension of inline
    /// functions with lambda parameters, as well as a simple form of `for`
    /// loops.
    V1,
    /// The 2.0 version of Move.
    V2_0,
    /// The 2.1 version of Move,
    V2_1,
    /// The 2.2 version of Move
    V2_2,
    /// The currently unstable 2.3 version of Move
    V2_3,
}

impl Default for LanguageVersion {
    fn default() -> Self {
        Self::latest_stable()
    }
}

impl Serialize for LanguageVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LanguageVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl FromStr for LanguageVersion {
    type Err = anyhow::Error;

    /// Parses a language version. If the caller only provides a major
    /// version number, this chooses the latest stable minor version (if any).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Strip unstable marker as it is not relevant for parsing.
        let s1 = s.replace(UNSTABLE_MARKER, "");
        match s1.as_str() {
            "1" => Ok(Self::V1),
            "2.0" => Ok(Self::V2_0),
            "2" | "2.1" => Ok(Self::V2_1),
            "2.2" => Ok(Self::V2_2),
            "2.3" => Ok(Self::V2_3),
            _ => bail!(
                "unrecognized language version \"{}\" (supported versions: \"1\", \"2\", \"2.0-2.3\")",
                s
            ),
        }
    }
}

impl From<LanguageVersion> for CompilerLanguageVersion {
    fn from(val: LanguageVersion) -> Self {
        match val {
            LanguageVersion::V1 => CompilerLanguageVersion::V1,
            LanguageVersion::V2_0 => CompilerLanguageVersion::V2_0,
            LanguageVersion::V2_1 => CompilerLanguageVersion::V2_1,
            LanguageVersion::V2_2 => CompilerLanguageVersion::V2_2,
            LanguageVersion::V2_3 => CompilerLanguageVersion::V2_3,
        }
    }
}

impl LanguageVersion {
    /// Whether the language version is unstable. An unstable version
    /// should not be allowed on production networks.
    pub const fn unstable(self) -> bool {
        use LanguageVersion::*;
        match self {
            V1 | V2_0 | V2_1 | V2_2 => false,
            V2_3 => true,
        }
    }

    /// The latest language version.
    pub const fn latest() -> Self {
        LanguageVersion::V2_3
    }

    /// The latest stable language version.
    pub const fn latest_stable() -> Self {
        LATEST_STABLE_LANGUAGE_VERSION_VALUE
    }

    /// Whether the language version is equal to greater than `ver`
    pub fn is_at_least(&self, ver: LanguageVersion) -> bool {
        *self >= ver
    }

    /// If the bytecode version is not specified, infer it from the language version. For
    /// debugging purposes, respects the MOVE_BYTECODE_VERSION env var as an override.
    pub fn infer_bytecode_version(&self, version: Option<u32>) -> u32 {
        env::get_bytecode_version_from_env(version).unwrap_or(match self {
            LanguageVersion::V1 => VERSION_DEFAULT,
            LanguageVersion::V2_0
            | LanguageVersion::V2_1
            | LanguageVersion::V2_2
            | LanguageVersion::V2_3 => VERSION_DEFAULT_LANG_V2,
        })
    }

    pub const fn to_str(&self) -> &'static str {
        match self {
            LanguageVersion::V1 => "1",
            LanguageVersion::V2_0 => "2.0",
            LanguageVersion::V2_1 => "2.1",
            LanguageVersion::V2_2 => "2.2",
            LanguageVersion::V2_3 => "2.3",
        }
    }
}

impl Display for LanguageVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            match self {
                LanguageVersion::V1 => "1",
                LanguageVersion::V2_0 => "2.0",
                LanguageVersion::V2_1 => "2.1",
                LanguageVersion::V2_2 => "2.2",
                LanguageVersion::V2_3 => "2.3",
            },
            if self.unstable() { UNSTABLE_MARKER } else { "" }
        )
    }
}
