// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// The fuzzer input was exhausted before generation completed.
    OutOfInput,
    /// The generator hit a state from which it could not continue under
    /// the configured constraints.
    GenerationStuck(&'static str),
    /// An invariant in the generator itself was violated (a bug).
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OutOfInput => f.write_str("fuzzer input exhausted"),
            Error::GenerationStuck(reason) => write!(f, "generation stuck: {}", reason),
            Error::Internal(msg) => write!(f, "internal generator error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<arbitrary::Error> for Error {
    fn from(err: arbitrary::Error) -> Self {
        match err {
            arbitrary::Error::NotEnoughData => Error::OutOfInput,
            other => Error::Internal(other.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
