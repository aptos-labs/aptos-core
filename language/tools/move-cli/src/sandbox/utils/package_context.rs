// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
use crate::{sandbox::utils::OnDiskStateView, DEFAULT_BUILD_DIR};
use anyhow::Result;
use move_package::{compilation::compiled_package::CompiledPackage, BuildConfig};
use std::path::{Path, PathBuf};

/// The PackageContext controls the package that the CLI is executing with respect to, and handles the
/// creation of the `OnDiskStateView` with the package's dependencies.
pub struct PackageContext {
    package: CompiledPackage,
    build_dir: PathBuf,
}

impl PackageContext {
    pub fn new(path: &Path, build_config: &BuildConfig) -> Result<Self> {
        let build_dir = build_config
            .install_dir
            .as_ref()
            .unwrap_or(&PathBuf::from(DEFAULT_BUILD_DIR))
            .clone();
        let package = build_config
            .clone()
            .compile_package(path, &mut Vec::new())?;
        Ok(PackageContext { package, build_dir })
    }

    /// Prepare an OnDiskStateView that is ready to use. Library modules will be preloaded into the
    /// storage if `load_libraries` is true.
    ///
    /// NOTE: this is the only way to get a state view in Move CLI, and thus, this function needs
    /// to be run before every command that needs a state view, i.e., `publish`, `run`,
    /// `view`, and `doctor`.
    pub fn prepare_state(&self, storage_dir: &Path) -> Result<OnDiskStateView> {
        let state = OnDiskStateView::create(self.build_dir.as_path(), storage_dir)?;

        // preload the storage with library modules (if such modules do not exist yet)
        let package = self.package();
        let new_modules: Vec<_> = package
            .dependencies
            .iter()
            .flat_map(|dep| {
                dep.compiled_modules()
                    .iter_modules_owned()
                    .into_iter()
                    .filter(|m| !state.has_module(&m.self_id()))
            })
            .collect();

        let mut serialized_modules = vec![];
        for module in new_modules {
            let self_id = module.self_id();
            let mut module_bytes = vec![];
            module.serialize(&mut module_bytes)?;
            serialized_modules.push((self_id, module_bytes));
        }
        state.save_modules(&serialized_modules)?;

        Ok(state)
    }

    pub fn package(&self) -> &CompiledPackage {
        &self.package
    }
}

impl Default for PackageContext {
    fn default() -> Self {
        Self::new(&std::env::current_dir().unwrap(), &BuildConfig::default()).unwrap()
    }
}
