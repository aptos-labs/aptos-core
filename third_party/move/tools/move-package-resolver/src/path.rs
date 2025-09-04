// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::{
    ops::Deref,
    path::{Component, Path, PathBuf},
};

/// Wrapper around [`PathBuf`] that represents a canonical path, which is not only normalized,
/// but also absolute and have all symbolic links resolved.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct CanonicalPath(PathBuf);

impl Deref for CanonicalPath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for CanonicalPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl CanonicalPath {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().canonicalize()?;
        Ok(Self(path))
    }
}

/// Normalizes a path by removing all redundant `..` and `.` components.
/// Accepts both relative and absolute paths as input.
///
/// Examples:
/// - `./foo` -> `foo`
/// - `a/b/../c` -> `a/c`
/// - `/foo/../..` -> `/`
/// - `a/../../b` -> `../b`
fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let mut stack = vec![];

    for component in path.components() {
        match &component {
            Component::CurDir => (),
            Component::ParentDir => match stack.last() {
                Some(Component::Prefix(_) | Component::RootDir) => (),
                Some(Component::Normal(_)) => {
                    stack.pop();
                },
                Some(Component::ParentDir) | None => {
                    stack.push(component);
                },
                Some(Component::CurDir) => unreachable!(),
            },
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                stack.push(component);
            },
        }
    }

    stack
        .into_iter()
        .map(|c| c.as_os_str())
        .collect::<PathBuf>()
}

/// Wrapper around [`PathBuf`] that represents a normalized path, which is a path that
/// does not contain any `..` or `.` components.
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct NormalizedPath(PathBuf);

impl NormalizedPath {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(normalize_path(path))
    }
}

impl Deref for NormalizedPath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for NormalizedPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

#[test]
fn test_normalize_path() {
    for (path, expected) in [
        // Relative paths
        (".", ""),
        ("..", ".."),
        ("a", "a"),
        ("a/", "a"),
        ("a/b/c", "a/b/c"),
        ("a/b/c/..", "a/b"),
        ("a/b/../c", "a/c"),
        ("a/b/../../c", "c"),
        ("..", ".."),
        ("a/../..", ".."),
        ("../../..", "../../.."),
        ("a/b/../../../c/d/../e", "../c/e"),
        // Absolute paths
        ("/", "/"),
        ("/.", "/"),
        ("/./", "/"),
        ("/a", "/a"),
        ("/a/", "/a"),
        ("/a/b/..", "/a"),
        ("/a/b/../..", "/"),
        ("/a/b/../../..", "/"),
        ("/a/./b/././c/..", "/a/b"),
        ("/a//b///c", "/a/b/c"),
        ("/..", "/"),
        ("/../..", "/"),
        ("/a/../../b", "/b"),
        ("/a/../../../b", "/b"),
        ("/a/b/c/../../../x/y/..", "/x"),
        // TODO: Add tests for Windows
    ] {
        assert_eq!(normalize_path(Path::new(path)), PathBuf::from(expected));
    }
}
