// Copyright © Aptos Foundation

use std::fmt::{Display, Formatter};

/// A marker type that can be used in [`std::result::Result`] to indicate
/// that no error can be returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoError {
    /// Private field ensures that `NoError` cannot be instantiated outside of this module.
    _private: (),
}

impl Display for NoError {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("NoError is not supposed to ever be instantiated")
    }
}

impl std::error::Error for NoError {
    fn description(&self) -> &str {
        unreachable!("NoError is not supposed to ever be instantiated")
    }
}

/// A type alias for [`std::result::Result`] that uses [`NoError`] as the error type.
pub type Result<T> = std::result::Result<T, NoError>;
