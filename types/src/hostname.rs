// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::{fmt, ops::Deref};

/// Hostname limit: https://www.rfc-editor.org/rfc/rfc1123#page-13
const MAX_HOSTNAME_LENGTH: usize = 255;

/// A basic wrapper around String to hold a size-limited hostname
/// It truncates strings that are longer than expected length
/// to prevent attacks.
#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Hostname {
    inner: String,
}

impl Hostname {
    /// Creates a new size-limited Hostname from [String]
    pub fn new(value: String) -> Self {
        let mut value = value;
        value.truncate(MAX_HOSTNAME_LENGTH);
        Self { inner: value }
    }
}

impl Deref for Hostname {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<String> for Hostname {
    fn from(value: String) -> Self {
        Hostname::new(value)
    }
}

impl From<&str> for Hostname {
    fn from(value: &str) -> Self {
        Hostname::new(value.into())
    }
}

impl fmt::Display for Hostname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::Hostname;
    use rand::{distributions::Alphanumeric, Rng};

    #[test]
    fn test_hostname() {
        let hostname_string = String::from("test-hostname");
        let hostname = Hostname::new(hostname_string.clone());

        assert_eq!(*hostname, hostname_string);

        let too_long_string: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(super::MAX_HOSTNAME_LENGTH + 20)
            .map(char::from)
            .collect();

        let hostname = Hostname::new(too_long_string.clone());

        assert_eq!(hostname.len(), super::MAX_HOSTNAME_LENGTH);

        assert_eq!(*hostname, too_long_string[..super::MAX_HOSTNAME_LENGTH]);
    }
}
