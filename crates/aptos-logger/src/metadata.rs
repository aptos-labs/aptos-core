// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, FromRepr};

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
    Display,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "ERROR")]
    Error = 0,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "WARN")]
    Warn = 1,
    /// The "info" level.
    ///
    /// Designates useful information.
    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "INFO")]
    Info = 2,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "DEBUG")]
    Debug = 3,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "TRACE")]
    Trace = 4,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_log_level_from_string() {
        assert_eq!(Level::Error, Level::from_str("ERROR").unwrap());
        assert_eq!(Level::Error, Level::from_str("Error").unwrap());
        assert_eq!(Level::Error, Level::from_str("error").unwrap());

        assert_eq!(Level::Warn, Level::from_str("WARN").unwrap());
        assert_eq!(Level::Warn, Level::from_str("wArN").unwrap());
        assert_eq!(Level::Warn, Level::from_str("warn").unwrap());

        assert_eq!(Level::Info, Level::from_str("INFO").unwrap());
        assert_eq!(Level::Info, Level::from_str("infO").unwrap());
        assert_eq!(Level::Info, Level::from_str("info").unwrap());

        assert_eq!(Level::Debug, Level::from_str("DEBUG").unwrap());
        assert_eq!(Level::Debug, Level::from_str("Debug").unwrap());
        assert_eq!(Level::Debug, Level::from_str("debug").unwrap());

        assert_eq!(Level::Trace, Level::from_str("TRACE").unwrap());
        assert_eq!(Level::Trace, Level::from_str("trAce").unwrap());
        assert_eq!(Level::Trace, Level::from_str("trace").unwrap());

        assert!(Level::from_str("ERR").is_err());
        assert!(Level::from_str("err_or").is_err());
        assert!(Level::from_str("INF").is_err());
        assert!(Level::from_str("tracey").is_err());
    }

    #[test]
    fn test_log_level_to_string() {
        assert_eq!(String::from("ERROR"), Level::Error.to_string());
        assert_eq!(String::from("WARN"), Level::Warn.to_string());
        assert_eq!(String::from("INFO"), Level::Info.to_string());
        assert_eq!(String::from("DEBUG"), Level::Debug.to_string());
        assert_eq!(String::from("TRACE"), Level::Trace.to_string());
    }
}
