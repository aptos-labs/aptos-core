// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_rest_client::Client;
use aptos_types::account_address::AccountAddress;
use aptos_types::transaction::ScriptABI;
use framework::natives::code::{ModuleMetadata, PackageMetadata, PackageRegistry, UpgradePolicy};
use framework::unzip_metadata;
use move_deps::move_package::compilation::package_layout::CompiledPackageLayout;
use reqwest::Url;
use std::fs;
use std::path::PathBuf;

// TODO: this is a first naive implementation of the package registry. Before mainnet
// we need to use tables for the package registry.

/// Represents the package registry at a given account.
pub struct CachedPackageRegistry {
    inner: PackageRegistry,
}

/// Represents the package metadata found in an registry.
pub struct CachedPackageMetadata<'a> {
    metadata: &'a PackageMetadata,
}

/// Represents the package metadata found in an registry.
pub struct CachedModuleMetadata<'a> {
    metadata: &'a ModuleMetadata,
}

impl CachedPackageRegistry {
    /// Creates a new registry.
    pub async fn create(url: Url, addr: AccountAddress) -> anyhow::Result<Self> {
        let client = Client::new(url);
        Ok(Self {
            inner: client
                .get_resource::<PackageRegistry>(addr, "0x1::code::PackageRegistry")
                .await?
                .into_inner(),
        })
    }

    /// Returns the list of packages in this registry by name.
    pub fn package_names(&self) -> Vec<String> {
        self.inner.packages.iter().map(|p| p.name.clone()).collect()
    }

    /// Finds the metadata for the given module in the registry by its unique name.
    pub async fn get_module<'a>(
        &self,
        name: impl AsRef<str>,
    ) -> anyhow::Result<CachedModuleMetadata<'_>> {
        let name = name.as_ref();
        for package in &self.inner.packages {
            for module in &package.modules {
                if module.name == name {
                    return Ok(CachedModuleMetadata { metadata: module });
                }
            }
        }
        bail!("module `{}` not found", name)
    }

    /// Finds the metadata for the given package in the registry by its unique name.
    pub async fn get_package<'a>(
        &self,
        name: impl AsRef<str>,
    ) -> anyhow::Result<CachedPackageMetadata<'_>> {
        let name = name.as_ref();
        for package in &self.inner.packages {
            if package.name == name {
                return Ok(CachedPackageMetadata { metadata: package });
            }
        }
        bail!("package `{}` not found", name)
    }
}

impl<'a> CachedPackageMetadata<'a> {
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn upgrade_policy(&self) -> UpgradePolicy {
        self.metadata.upgrade_policy
    }

    pub fn build_info(&self) -> &str {
        &self.metadata.build_info
    }

    pub fn manifest(&self) -> &str {
        &self.metadata.manifest
    }

    pub fn error_map_raw(&self) -> &[u8] {
        &self.metadata.error_map
    }

    pub fn abis(&self) -> &[Vec<u8>] {
        self.metadata.abis.as_slice()
    }

    pub fn module_names(&self) -> Vec<String> {
        self.metadata
            .modules
            .iter()
            .map(|s| s.name.clone())
            .collect()
    }

    pub fn module(&self, name: impl AsRef<str>) -> anyhow::Result<CachedModuleMetadata<'_>> {
        let name = name.as_ref();
        for module in &self.metadata.modules {
            if module.name == name {
                return Ok(CachedModuleMetadata { metadata: module });
            }
        }
        bail!("module `{}` not found", name)
    }

    pub fn save_package_to_disk(
        &self,
        path: PathBuf,
        with_derived_artifacts: bool,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(&path)?;
        fs::write(path.join("Move.toml"), &self.metadata.manifest)?;
        fs::write(
            path.join("error_description.errmap"),
            &self.metadata.error_map,
        )?;
        fs::write(path.join("BuildInfo.yaml"), &self.metadata.build_info)?;
        let sources_dir = path.join(CompiledPackageLayout::Sources.path());
        fs::create_dir_all(&sources_dir)?;
        for module in &self.metadata.modules {
            let source = std::str::from_utf8(&unzip_metadata(&module.source)?)?.to_string();
            fs::write(sources_dir.join(format!("{}.move", module.name)), source)?;
        }
        if with_derived_artifacts {
            let abis_dir = path.join(CompiledPackageLayout::CompiledABIs.path());
            for abi_blob in &self.metadata.abis {
                let abi = bcs::from_bytes::<ScriptABI>(abi_blob.as_slice())?;
                let path = match abi {
                    ScriptABI::TransactionScript(abi) => {
                        PathBuf::from(format!("{}.abi", abi.name()))
                    }
                    ScriptABI::ScriptFunction(abi) => {
                        PathBuf::from(abi.module_name().name().as_str())
                            .join(format!("{}.abi", abi.name()))
                    }
                };
                let dest = abis_dir.join(path);
                fs::create_dir_all(&dest.parent().unwrap())?;
                fs::write(dest, abi_blob)?
            }
            let source_map_dir = path.join(CompiledPackageLayout::SourceMaps.path());
            fs::create_dir_all(&source_map_dir)?;
            for module in &self.metadata.modules {
                fs::write(
                    source_map_dir.join(format!("{}.mvsm", module.name)),
                    &module.source_map,
                )?;
            }
        }
        Ok(())
    }
}

impl<'a> CachedModuleMetadata<'a> {
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Returns the zipped source.
    pub fn zipped_source(&self) -> &[u8] {
        &self.metadata.source
    }

    pub fn zipped_source_map_raw(&self) -> &[u8] {
        &self.metadata.source_map
    }
}
