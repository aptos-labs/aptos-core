extern crate tracing;

use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub use crate::config::{
    load_config, CliOptions, Config, Edition, EmitMode, Verbosity,
};

#[macro_use]
pub mod utils;
pub mod comment;
pub mod config;
pub mod shape;
pub mod string;

/// The various errors that can occur during formatting. Note that not all of
/// these can currently be propagated to clients.
#[derive(Error, Debug)]
pub enum ErrorKind {
    /// Line has exceeded character limit (found, maximum).
    #[error(
        "line formatted, but exceeded maximum width \
         (maximum: {1} (see `max_width` option), found: {0})"
    )]
    LineOverflow(usize, usize),
    /// Line ends in whitespace.
    #[error("left behind trailing whitespace")]
    TrailingWhitespace,
    /// Used a movefmt:: attribute other than skip or skip::macros.
    #[error("invalid attribute")]
    BadAttr,
    /// An io error during reading or writing.
    #[error("io error: {0}")]
    IoError(io::Error),
    /// Error during module resolution.
    /// Parse error occurred when parsing the input.
    #[error("parse error")]
    ParseError,
    /// The user mandated a version and the current version of movefmt does not
    /// satisfy that requirement.
    #[error("version mismatch")]
    VersionMismatch,
    /// If we had formatted the given node, then we would have lost a comment.
    #[error("not formatted because a comment would be lost")]
    LostComment,
    /// Invalid glob pattern in `ignore` configuration option.
    #[error("Invalid glob pattern found in ignore list: {0}")]
    InvalidGlobPattern(ignore::Error),
}

impl From<io::Error> for ErrorKind {
    fn from(e: io::Error) -> ErrorKind {
        ErrorKind::IoError(e)
    }
}

#[derive(Debug)]
pub enum Input {
    File(PathBuf),
    Text(String),
}
