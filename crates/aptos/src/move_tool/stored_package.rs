// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_framework::{
    natives::code::{ModuleMetadata, PackageMetadata, PackageRegistry, UpgradePolicy},
    unzip_metadata_str,
};
use aptos_rest_client::Client;
use aptos_types::account_address::AccountAddress;
use move_package::compilation::package_layout::CompiledPackageLayout;
use reqwest::Url;
use std::{collections::BTreeMap, fmt, fs, path::Path};

// TODO: this is a first naive implementation of the package registry. Before mainnet
// we need to use tables for the package registry.

/// Represents the package registry at a given account.
pub struct CachedPackageRegistry {
    inner: PackageRegistry,
    bytecode: BTreeMap<String, Vec<u8>>,
}

/// Represents the package metadata found in an registry.
pub struct CachedPackageMetadata<'a> {
    metadata: &'a PackageMetadata,
}

/// Represents the package metadata found in an registry.
pub struct CachedModuleMetadata<'a> {
    metadata: &'a ModuleMetadata,
}

impl fmt::Display for CachedPackageMetadata<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.metadata)?;
        Ok(())
    }
}

impl CachedPackageRegistry {
    /// Creates a new registry.
    pub async fn create(
        url: Url,
        addr: AccountAddress,
        with_bytecode: bool,
    ) -> anyhow::Result<Self> {
        let client = Client::new(url);
        // Need to use a different type to deserialize JSON
        let inner = client
            .get_account_resource_bcs::<PackageRegistry>(addr, "0x1::code::PackageRegistry")
            .await?
            .into_inner();
        let mut bytecode = BTreeMap::new();
        if with_bytecode {
            for pack in &inner.packages {
                for module in &pack.modules {
                    let bytes = client
                        .get_account_module(addr, &module.name)
                        .await?
                        .into_inner()
                        .bytecode
                        .0;
                    bytecode.insert(module.name.clone(), bytes);
                }
            }
        }
        Ok(Self { inner, bytecode })
    }

    /// Returns the list of packages in this registry by name.
    pub fn package_names(&self) -> Vec<&str> {
        self.inner
            .packages
            .iter()
            .map(|p| p.name.as_str())
            .collect()
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

    /// Gets the bytecode associated with the module.
    pub async fn get_bytecode(
        &self,
        module_name: impl AsRef<str>,
    ) -> anyhow::Result<Option<&[u8]>> {
        Ok(self
            .bytecode
            .get(module_name.as_ref())
            .map(|v| v.as_slice()))
    }
}

impl CachedPackageMetadata<'_> {
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn upgrade_policy(&self) -> UpgradePolicy {
        self.metadata.upgrade_policy
    }

    pub fn upgrade_number(&self) -> u64 {
        self.metadata.upgrade_number
    }

    pub fn source_digest(&self) -> &str {
        &self.metadata.source_digest
    }

    pub fn manifest(&self) -> anyhow::Result<String> {
        unzip_metadata_str(&self.metadata.manifest)
    }

    pub fn module_names(&self) -> Vec<&str> {
        self.metadata
            .modules
            .iter()
            .map(|s| s.name.as_str())
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

    pub fn save_package_to_disk(&self, path: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(path)?;
        fs::write(
            path.join("Move.toml"),
            unzip_metadata_str(&self.metadata.manifest)?,
        )?;
        let sources_dir = path.join(CompiledPackageLayout::Sources.path());
        fs::create_dir_all(&sources_dir)?;
        for module in &self.metadata.modules {
            match module.source.is_empty() {
                true => {
                    println!("module without code: {}", module.name);
                },
                false => {
                    let source = unzip_metadata_str(&module.source)?;
                    fs::write(sources_dir.join(format!("{}.move", module.name)), source)?;
                },
            };
        }
        Ok(())
    }

    pub fn save_bytecode_to_disk(
        &self,
        path: &Path,
        module_name: &str,
        bytecode: &[u8],
    ) -> anyhow::Result<()> {
        let bytecode_dir = path.join(CompiledPackageLayout::CompiledModules.path());
        fs::create_dir_all(&bytecode_dir)?;
        fs::write(bytecode_dir.join(format!("{}.mv", module_name)), bytecode)?;
        Ok(())
    }

    pub fn verify(&self, package_metadata: &PackageMetadata) -> anyhow::Result<()> {
        let self_metadata = self.metadata;

        if self_metadata.name != package_metadata.name {
            bail!(
                "Package name doesn't match {} : {}",
                package_metadata.name,
                self_metadata.name
            )
        } else if self_metadata.deps != package_metadata.deps {
            bail!(
                "Dependencies don't match {:?} : {:?}",
                package_metadata.deps,
                self_metadata.deps
            )
        } else if self_metadata.modules != package_metadata.modules {
            bail!(
                "Modules don't match {:?} : {:?}",
                package_metadata.modules,
                self_metadata.modules
            )
        } else if self_metadata.manifest != package_metadata.manifest {
            bail!(
                "Manifest doesn't match {:?} : {:?}",
                package_metadata.manifest,
                self_metadata.manifest
            )
        } else if self_metadata.upgrade_policy != package_metadata.upgrade_policy {
            bail!(
                "Upgrade policy doesn't match {:?} : {:?}",
                package_metadata.upgrade_policy,
                self_metadata.upgrade_policy
            )
        } else if self_metadata.extension != package_metadata.extension {
            bail!(
                "Extensions doesn't match {:?} : {:?}",
                package_metadata.extension,
                self_metadata.extension
            )
        } else if self_metadata.source_digest != package_metadata.source_digest {
            bail!(
                "Source digests doesn't match {:?} : {:?}",
                package_metadata.source_digest,
                self_metadata.source_digest
            )
        }

        Ok(())
    }
}

impl CachedModuleMetadata<'_> {
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn zipped_source(&self) -> &[u8] {
        &self.metadata.source
    }

    pub fn zipped_source_map_raw(&self) -> &[u8] {
        &self.metadata.source_map
    }
}
