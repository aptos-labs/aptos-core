// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::path::{CanonicalPath, NormalizedPath};
use anyhow::{anyhow, Result};
use git2::Oid;
use move_core_types::account_address::AccountAddress;
use std::{
    fmt::{self, Display, Write},
    ops::Deref,
    path::Path,
};
use url::Url;

/// Canonicalized URL of a git repository.
/// - Must have a host
/// - Can have an optional port
/// - Trims trailing slashes and `.git` suffix
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct CanonicalGitUrl(Url);

impl CanonicalGitUrl {
    pub fn new(url: &Url) -> Result<Self> {
        let scheme = url.scheme();

        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("invalid git URL, unable to extract host: {}", url))?
            .to_lowercase();

        let port = url.port();

        let path = url.path().trim_end_matches("/").trim_end_matches(".git");

        let mut res = format!("{}://{}", scheme, host);
        if let Some(port) = port {
            res.push_str(&format!(":{}", port));
        }
        res.push_str(path);

        Ok(Self(Url::parse(&res).expect("should always succeed")))
    }
}

impl Display for CanonicalGitUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for CanonicalGitUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[test]
fn test_canonical_git_url() {
    let canonical = Url::parse("https://github.com/foo/bar").unwrap();

    for url in [
        "https://github.com/foo/bar.git",
        "https://github.com/foo/bar/",
        "https://github.com/foo/bar.git/",
        "https://GITHUB.com/foo/bar",
    ] {
        assert_eq!(
            *CanonicalGitUrl::new(&Url::parse(url).unwrap()).unwrap(),
            canonical
        );
    }

    for url in [
        "https://github.com/foo/bar.git/abc",
        "https://github.com/Foo/bar",
    ] {
        assert_ne!(
            *CanonicalGitUrl::new(&Url::parse(url).unwrap()).unwrap(),
            canonical
        );
    }
}

/// Canonicalized URL of a node.
/// - Must have host
/// - Can have an optional port
/// - Trims trailing slashes
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct CanonicalNodeUrl(Url);

impl CanonicalNodeUrl {
    pub fn new(url: &Url) -> Result<Self> {
        let scheme = url.scheme();

        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("invalid git URL, unable to extract host: {}", url))?
            .to_lowercase();

        let port = url.port();

        let path = url.path().trim_end_matches("/");

        let mut res = format!("{}://{}", scheme, host);
        if let Some(port) = port {
            res.push_str(&format!(":{}", port));
        }
        res.push_str(path);

        Ok(Self(Url::parse(&res).expect("should always succeed")))
    }
}

impl Deref for CanonicalNodeUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for CanonicalNodeUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[test]
fn test_canonical_node_url() {
    let canonical = Url::parse("https://node.com").unwrap();

    for url in ["https://node.com", "https://node.com/", "https://nOdE.com"] {
        assert_eq!(
            *CanonicalNodeUrl::new(&Url::parse(url).unwrap()).unwrap(),
            canonical
        );
    }

    let canonical = Url::parse("https://node.com:1234/foo").unwrap();
    for url in ["https://NODE.com:1234/foo", "https://node.com:1234/foo/"] {
        assert_eq!(
            *CanonicalNodeUrl::new(&Url::parse(url).unwrap()).unwrap(),
            canonical
        );
    }
}

/// Source location of a package, with canonicalized paths and URLs.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum SourceLocation {
    Local {
        path: CanonicalPath,
    },
    OnChain {
        node_url: CanonicalNodeUrl,
        package_addr: AccountAddress,
    },
    Git {
        url: CanonicalGitUrl,
        commit_id: Oid,
        subdir: NormalizedPath,
    },
}

/// Unique identifier for a package.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageIdentity {
    pub name: String,
    pub location: SourceLocation,
}

impl SourceLocation {
    pub fn fmt_strip_root_path(
        &self,
        f: &mut impl Write,
        strip_root_path: Option<&Path>,
    ) -> fmt::Result {
        match self {
            SourceLocation::Local { path } => {
                write!(f, "local:{}", match strip_root_path {
                    Some(root_path) => path.strip_prefix(root_path).unwrap().display(),
                    None => path.display(),
                })
            },
            SourceLocation::OnChain {
                node_url,
                package_addr,
            } => {
                write!(f, "onchain:{}::{}", node_url, package_addr)
            },
            SourceLocation::Git {
                url,
                commit_id,
                subdir,
            } => {
                write!(f, "git:{}@{}/{}", url, commit_id, subdir.display())
            },
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_strip_root_path(f, None)
    }
}
