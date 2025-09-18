// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::path::{CanonicalPath, NormalizedPath};
use git2::Oid;
use move_core_types::account_address::AccountAddress;
use move_package_cache::{CanonicalGitIdentity, CanonicalNodeIdentity};
use std::{
    fmt::{self, Write},
    path::Path,
};

/// Source location of a package, with canonicalized paths and URLs.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum SourceLocation {
    Local {
        path: CanonicalPath,
    },
    OnChain {
        node: CanonicalNodeIdentity,
        package_addr: AccountAddress,
    },
    Git {
        repo: CanonicalGitIdentity,
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
    /// Formats the source location, stripping the root path if provided.
    pub fn fmt_strip_root_path(
        &self,
        f: &mut impl Write,
        strip_root_path: Option<&Path>,
    ) -> fmt::Result {
        match self {
            SourceLocation::Local { path } => {
                write!(f, "local:{}", match strip_root_path {
                    Some(root_path) => {
                        if let Ok(stripped) = path.strip_prefix(root_path) {
                            stripped.display()
                        } else {
                            path.display()
                        }
                    },
                    None => path.display(),
                })
            },
            SourceLocation::OnChain { node, package_addr } => {
                write!(f, "onchain:{}::{}", node, package_addr)
            },
            SourceLocation::Git {
                repo,
                commit_id,
                subdir,
            } => {
                write!(f, "git:{}@{}/{}", repo, commit_id, subdir.display())
            },
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_strip_root_path(f, None)
    }
}
