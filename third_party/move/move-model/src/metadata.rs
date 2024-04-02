// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use move_command_line_common::env::read_bool_env_var;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Display, Formatter},
    str::FromStr,
};

const UNSTABLE_MARKER: &str = "-unstable";

/// Metadata about a compilation result. To maintain serialization
/// compatibility, this uses a free-form string to represent compiler version
/// and language version.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompilationMetadata {
    /// The version of the compiler, as a string.
    pub compiler_version: String,
    /// The version of the language, as a string.
    pub language_version: String,
    /// A flag indicating whether, at time of creation, the compilation
    /// result was considered as unstable. Unstable code may have restrictions
    /// for deployment on production networks.
    pub unstable: bool,
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

/// Represents a compiler version
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
pub enum CompilerVersion {
    /// The legacy v1 Move compiler
    V1,
    /// The v2 compiler, starting with 2.0. Each new released version of the compiler
    /// should get an enum entry here.
    V2_0,
}

impl Default for CompilerVersion {
    /// We allow the default to be set via an environment variable.
    fn default() -> Self {
        static MOVE_COMPILER_V2: Lazy<bool> = Lazy::new(|| read_bool_env_var("MOVE_COMPILER_V2"));
        if *MOVE_COMPILER_V2 {
            Self::V2_0
        } else {
            Self::V1
        }
    }
}

impl FromStr for CompilerVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Strip unstable marker as it is not relevant for parsing.
        let s1 = s.replace(UNSTABLE_MARKER, "");
        match s1.as_str() {
            // For legacy reasons, also support v1 and v2
            "1" | "v1" => Ok(Self::V1),
            "2" | "v2" | "2.0" => Ok(Self::V2_0),
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
            CompilerVersion::V2_0 => true,
        }
    }

    /// Check whether the compiler version supports the given language version,
    /// generates an error if not.
    pub fn check_language_support(self, version: LanguageVersion) -> anyhow::Result<()> {
        match self {
            CompilerVersion::V1 => {
                if version != LanguageVersion::V1 {
                    bail!("compiler v1 does only support Move language version 1")
                } else {
                    Ok(())
                }
            },
            CompilerVersion::V2_0 => Ok(()),
        }
    }
}

/// Represents a language version.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
pub enum LanguageVersion {
    /// The version of Move at around the genesis of the Aptos network end
    /// of '22. This is the original Diem Move plus the extension of inline
    /// functions with lambda parameters.
    V1,
    /// The upcoming (currently unstable) 2.0 version of Move. The following
    /// experimental language features are supported so far:
    ///
    /// - Access control specifiers as described in AIP-56.
    /// - receiver style (method style) function calls with auto-referencing
    V2_0,
}

impl Default for LanguageVersion {
    fn default() -> Self {
        static MOVE_LANGUAGE_V2: Lazy<bool> = Lazy::new(|| read_bool_env_var("MOVE_LANGUAGE_V2"));
        if *MOVE_LANGUAGE_V2 {
            Self::V2_0
        } else {
            Self::V1
        }
    }
}

impl FromStr for LanguageVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Strip unstable marker as it is not relevant for parsing.
        let s1 = s.replace(UNSTABLE_MARKER, "");
        match s1.as_str() {
            "1" => Ok(Self::V1),
            "2" | "2.0" => Ok(Self::V2_0),
            _ => bail!(
                "unrecognized language version `{}` (supported versions: `1`, `2`, `2.0`)",
                s
            ),
        }
    }
}

impl LanguageVersion {
    /// Whether the language version is unstable. An unstable version
    /// should not be allowed on production networks.
    pub fn unstable(self) -> bool {
        match self {
            LanguageVersion::V1 => false,
            LanguageVersion::V2_0 => true,
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
            },
            if self.unstable() { UNSTABLE_MARKER } else { "" }
        )
    }
}
