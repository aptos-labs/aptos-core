// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::{EnumString, FromRepr};

/// Associated metadata with every log to identify what kind of log and where it came from
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Metadata {
    /// The level of verbosity of the event
    #[serde(skip_serializing)]
    level: Level,

    /// The part of the system where the event occurred
    package: &'static str,

    /// The name of the Rust module where the event occurred
    // do not emit this field into the json logs since source.location is already more useful
    #[serde(skip)]
    module_path: &'static str,

    /// The source code file path and line number together as 'file_path:line'
    file: &'static str,
}

impl Metadata {
    pub const fn new(
        level: Level,
        target: &'static str,
        module_path: &'static str,
        source_path: &'static str,
    ) -> Self {
        Self {
            level,
            package: target,
            module_path,
            file: source_path,
        }
    }

    pub fn enabled(&self) -> bool {
        crate::logger::enabled(self)
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn target(&self) -> &'static str {
        self.package
    }

    pub fn module_path(&self) -> &'static str {
        self.module_path
    }

    pub fn source_path(&self) -> &'static str {
        self.file
    }
}

/// Logging levels, used for stratifying logs, and disabling less important ones for performance reasons
#[repr(usize)]
#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Hash,
    Serialize,
    Deserialize,
    FromRepr,
    EnumString,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    #[strum(ascii_case_insensitive)]
    Error = 0,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    #[strum(ascii_case_insensitive)]
    Warn = 1,
    /// The "info" level.
    ///
    /// Designates useful information.
    #[strum(ascii_case_insensitive)]
    Info = 2,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    #[strum(ascii_case_insensitive)]
    Debug = 3,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    #[strum(ascii_case_insensitive)]
    Trace = 4,
}

impl fmt::Display for Level {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let level_str = match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        };
        fmt.pad(level_str)
    }
}
