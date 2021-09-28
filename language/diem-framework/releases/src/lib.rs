// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use include_dir::{include_dir, Dir, DirEntry};
use move_binary_format::file_format::CompiledModule;
use move_bytecode_utils::Modules;
use move_command_line_common::files::MOVE_ERROR_DESC_EXTENSION;
use move_package::{
    compilation::compiled_package::{CompiledPackage, OnDiskCompiledPackage},
    source_package::manifest_parser::parse_move_manifest_from_file,
};
use once_cell::sync::Lazy;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Release {
    DPN,
    Experimental,
}

impl Release {
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::DPN => "DPN",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseFetcher {
    release: Release,
    release_name: String,
}

static RELEASES_MAP: Lazy<BTreeMap<Release, Dir>> = Lazy::new(|| {
    let mut map = BTreeMap::new();
    map.insert(Release::DPN, include_dir!("../DPN/releases/artifacts"));
    map.insert(
        Release::Experimental,
        include_dir!("../experimental/releases/artifacts"),
    );
    map
});

impl ReleaseFetcher {
    /// Create a new release fetcher for the given `Release` and `release_name`.
    pub fn new(release: Release, release_name: &str) -> Self {
        Self {
            release,
            release_name: release_name.to_string(),
        }
    }

    /// Fetch the current release of the given `Release`
    pub fn current(release: Release) -> Self {
        Self::new(release, "current")
    }

    /// Load the serialized modules from the specified release.
    pub fn module_blobs(&self) -> Result<Vec<Vec<u8>>> {
        Ok(self
            .modules()?
            .into_iter()
            .map(|module| {
                let mut bytes = vec![];
                module.serialize(&mut bytes).unwrap();
                bytes
            })
            .collect())
    }

    /// Load the modules for the specified release. Returned in dependency order.
    pub fn modules(&self) -> Result<Vec<CompiledModule>> {
        let modules = RELEASES_MAP[&self.release]
            .get_dir(&self.release_name)
            .ok_or_else(|| {
                anyhow::format_err!(
                    "Unable to find release name '{}', for release '{}'",
                    &self.release_name,
                    self.release.to_string()
                )
            })?
            .find("**/*modules/*.mv")?
            .filter_map(|file_module| match file_module {
                DirEntry::Dir(_) => None,
                DirEntry::File(file) => Some(CompiledModule::deserialize(file.contents()).unwrap()),
            })
            .collect::<Vec<_>>();
        let x = Modules::new(modules.iter())
            .compute_dependency_graph()
            .compute_topological_order()?
            .into_iter()
            .map(Clone::clone)
            .collect();
        Ok(x)
    }

    pub fn list_releases(release: &Release) -> Vec<String> {
        RELEASES_MAP[release]
            .dirs()
            .iter()
            .map(|dir| {
                dir.path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect()
    }

    pub fn error_descriptions(&self) -> Result<Vec<u8>> {
        let mut errmap_path = PathBuf::from(&self.release_name);
        errmap_path.push("error_description");
        errmap_path.push("error_description");
        errmap_path.set_extension(MOVE_ERROR_DESC_EXTENSION);

        match RELEASES_MAP[&self.release].get_file(errmap_path) {
            Some(file) => Ok(file.contents().to_vec()),
            None => anyhow::bail!("release {} not found", &self.release_name),
        }
    }

    /// Load the serialized modules from the specified release.
    pub fn package(&self) -> Result<CompiledPackage> {
        let root_path = Path::new(std::env!("CARGO_MANIFEST_DIR")).join(&self.release_name);
        let package_name = parse_move_manifest_from_file(&root_path)?.package.name;
        let path = root_path
            .join("releases")
            .join("artifacts")
            .join(&self.release_name)
            .join("build")
            .join(package_name.as_str());
        Ok(OnDiskCompiledPackage::from_path(&path)
            .unwrap()
            .into_compiled_package()
            .unwrap())
    }
}
