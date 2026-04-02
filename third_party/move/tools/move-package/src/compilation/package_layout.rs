// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum CompiledPackageLayout {
    BuildInfo,
    Root,
    Dependencies,
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
            Self::Dependencies => "dependencies",
            Self::Sources => "sources",
            Self::SourceMaps => "source_maps",
            Self::CompiledModules => "bytecode_modules",
            Self::CompiledScripts => "bytecode_scripts",
            Self::CompiledDocs => "docs",
            Self::CompiledABIs => "abis",
        };
        Path::new(path)
    }

    pub fn path_to_file_after_category(path: &Path) -> PathBuf {
        let mut suffix_components = vec![];
        // reverse iterate until Root is found
        for component in path.components().rev() {
            let component_path: &Path = component.as_ref();
            if component_path == Self::Root.path() {
                break;
            }
            suffix_components.push(component);
        }
        // pop root package name
        suffix_components.pop();
        // pop category
        suffix_components.pop();
        // put the components back in order
        suffix_components.reverse();
        // make the path
        suffix_components.into_iter().collect()
    }
}
