// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use std::{
    fmt::{self, Display},
    ops::Deref,
};
use url::Url;

/// Canonicalized identity of a git repository, derived from a [`Url`].
/// - Ignores the scheme
/// - Converts host & path to lowercase
/// - Keeps port, but only if it is non-default
/// - Trims trailing slashes and `.git` suffix
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct CanonicalGitIdentity(String);

impl CanonicalGitIdentity {
    pub fn new(git_url: &Url) -> Result<Self> {
        let host = git_url
            .host_str()
            .ok_or_else(|| anyhow!("invalid git URL, unable to extract host: {}", git_url))?
            .to_ascii_lowercase();

        let port = match git_url.port() {
            Some(port) => match (git_url.scheme(), port) {
                ("http", 80) | ("https", 443) | ("ssh", 22) => "".to_string(),
                _ => format!(":{}", port),
            },
            None => "".to_string(),
        };

        let path = git_url.path().to_ascii_lowercase();
        let path = path.trim_end_matches("/").trim_end_matches(".git");

        Ok(Self(format!("{}{}{}", host, port, path)))
    }
}

impl Deref for CanonicalGitIdentity {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for CanonicalGitIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[test]
fn test_canonical_git_identity() {
    let canonical = "github.com/foo/bar";

    for url in [
        "https://github.com/foo/bar.git",
        "https://github.com/foo/bar/",
        "https://github.com/foo/bar.git/",
        "https://GITHUB.com/foo/bar",
        "https://github.com/Foo/bar",
        "ssh://github.com/Foo/bar",
    ] {
        assert_eq!(
            &*CanonicalGitIdentity::new(&Url::parse(url).unwrap()).unwrap(),
            canonical
        );
    }

    #[allow(clippy::single_element_loop)]
    for url in ["https://github.com/foo/bar.git/abc"] {
        assert_ne!(
            &*CanonicalGitIdentity::new(&Url::parse(url).unwrap()).unwrap(),
            canonical
        );
    }
}

/// Canonicalized identity of a node, derived from a [`Url`].
/// - Ignores the scheme
/// - Converts host & path to lowercase
/// - Keeps port, but only if it is non-default
/// - Trims trailing slashes
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct CanonicalNodeIdentity(String);

impl CanonicalNodeIdentity {
    pub fn new(node_url: &Url) -> Result<Self> {
        let host = node_url
            .host_str()
            .ok_or_else(|| anyhow!("invalid node URL, unable to extract host: {}", node_url))?
            .to_ascii_lowercase();

        let port = match node_url.port() {
            Some(port) => match (node_url.scheme(), port) {
                ("http", 80) | ("https", 443) => "".to_string(),
                _ => format!(":{}", port),
            },
            None => "".to_string(),
        };

        let path = node_url.path().to_ascii_lowercase();
        let path = path.trim_end_matches("/");

        Ok(Self(format!("{}{}{}", host, port, path)))
    }
}

impl Deref for CanonicalNodeIdentity {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for CanonicalNodeIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[test]
fn test_canonical_node_url() {
    let canonical = "node.com";

    for url in ["https://node.com", "https://node.com/", "https://nOdE.com"] {
        assert_eq!(
            &*CanonicalNodeIdentity::new(&Url::parse(url).unwrap()).unwrap(),
            canonical
        );
    }

    let canonical = "node.com:1234/foo";
    for url in ["https://NODE.com:1234/foo", "https://node.com:1234/foo/"] {
        assert_eq!(
            &*CanonicalNodeIdentity::new(&Url::parse(url).unwrap()).unwrap(),
            canonical
        );
    }
}
