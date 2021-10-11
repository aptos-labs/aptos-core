// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum CompiledPackageLayout {
    BuildInfo,
    Root,
    Sources,
    SourceMaps,
    CompiledModules,
    CompiledScripts,
    CompiledDocs,
    CompiledABIs,
}

impl CompiledPackageLayout {
    pub fn path(&self) -> &Path {
        let path = match self {
            Self::BuildInfo => "BuildInfo.yaml",
            Self::Root => "build",
            Self::Sources => "sources",
            Self::SourceMaps => "source_maps",
            Self::CompiledModules => "bytecode_modules",
            Self::CompiledScripts => "bytecode_scripts",
            Self::CompiledDocs => "docs",
            Self::CompiledABIs => "abis",
        };
        Path::new(path)
    }

    pub fn from_sibling_path(&self, current_path: &Path) -> Option<PathBuf> {
        let pkg_root = Self::traverse_to_build_root(current_path)?;
        Some(pkg_root.join(self.path()))
    }

    pub fn traverse_to_build_root(path: &Path) -> Option<&Path> {
        for path in path.ancestors() {
            match path.parent() {
                Some(parent) if parent.ends_with(Self::Root.path()) => return Some(path),
                _ => (),
            }
        }

        None
    }
}
