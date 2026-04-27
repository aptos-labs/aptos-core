// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::bail;
use aptos_framework::{
    natives::code::{ModuleMetadata, PackageDep, PackageMetadata, PackageRegistry, UpgradePolicy},
    unzip_metadata_str,
};
use aptos_rest_client::Client;
use aptos_types::account_address::AccountAddress;
use move_package::compilation::package_layout::CompiledPackageLayout;
use reqwest::Url;
use std::{collections::BTreeMap, fmt, fs, path::Path};
use toml::Value as TV;

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
    pub async fn get_module(
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
    pub async fn get_package(
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
        self.save_package_to_disk_with_node(path, None)
    }

    /// Same as [`save_package_to_disk`], but if `node_url` is provided, the
    /// package's `Move.toml` is rewritten so that any dependency that the
    /// on-chain `PackageMetadata.deps` list identifies (i.e. another
    /// already-published Aptos package) is expressed as an `aptos = "<node>"`
    /// + `address = "<account>"` dependency.
    ///
    /// This lets transitive on-chain dependencies (e.g. `Pyth` referenced by
    /// `LiquidSwap`) be fetched recursively from the same node instead of
    /// failing to resolve as a stale local or git path on disk.
    pub fn save_package_to_disk_with_node(
        &self,
        path: &Path,
        node_url: Option<&str>,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(path)?;
        let manifest_text = unzip_metadata_str(&self.metadata.manifest)?;
        let final_manifest = match node_url {
            Some(url) => {
                rewrite_manifest_with_onchain_deps(&manifest_text, &self.metadata.deps, url)
                    .unwrap_or(manifest_text)
            },
            None => manifest_text,
        };
        fs::write(path.join("Move.toml"), final_manifest)?;
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

/// Rewrite the `[dependencies]` (and `[dev-dependencies]`) sections of the
/// given manifest string so any dep whose name matches a `PackageDep` from
/// the on-chain `PackageMetadata.deps` list is expressed as a node-backed
/// (`aptos = "<node_url>", address = "<account>"`) dependency. Other deps
/// and the rest of the manifest are left untouched.
///
/// Returns `Some(rewritten)` on success and `None` if anything goes wrong
/// while parsing/serializing (callers fall back to the raw on-chain
/// manifest in that case so existing behavior is preserved).
pub fn rewrite_manifest_with_onchain_deps(
    manifest: &str,
    onchain_deps: &[PackageDep],
    node_url: &str,
) -> Option<String> {
    if onchain_deps.is_empty() {
        return None;
    }
    let mut value: TV = toml::from_str(manifest).ok()?;
    let table = value.as_table_mut()?;
    let mut changed = false;
    for section in ["dependencies", "dev-dependencies"] {
        let Some(deps) = table.get_mut(section).and_then(|v| v.as_table_mut()) else {
            continue;
        };
        let dep_names: Vec<String> = deps.keys().cloned().collect();
        for name in dep_names {
            if let Some(onchain) = onchain_deps.iter().find(|d| d.package_name == name) {
                let mut new_dep = toml::value::Table::new();
                new_dep.insert("aptos".to_string(), TV::String(node_url.to_string()));
                new_dep.insert(
                    "address".to_string(),
                    TV::String(format!("0x{}", onchain.account.short_str_lossless())),
                );
                deps.insert(name, TV::Table(new_dep));
                changed = true;
            }
        }
    }
    if !changed {
        return None;
    }
    toml::to_string(&value).ok()
}

#[cfg(test)]
mod manifest_rewrite_tests {
    use super::*;
    use aptos_framework::natives::code::PackageDep;
    use aptos_types::account_address::AccountAddress;

    #[test]
    fn rewrites_local_dep_to_aptos_dep() {
        let manifest = r#"
[package]
name = "LiquidSwap"
version = "0.0.1"

[dependencies]
Pyth = { local = "../pyth" }
AptosFramework = { git = "https://github.com/aptos-labs/aptos-core.git", subdir = "aptos-move/framework/aptos-framework", rev = "main" }
"#;
        let onchain = vec![PackageDep {
            account: AccountAddress::from_hex_literal(
                "0x7e783b349d3e89cf5931af376ebeadbfab855b3fa239b7ada8f5a92fbea6b387",
            )
            .unwrap(),
            package_name: "Pyth".to_string(),
        }];
        let rewritten =
            rewrite_manifest_with_onchain_deps(manifest, &onchain, "https://node/v1").unwrap();
        assert!(
            rewritten.contains("aptos = \"https://node/v1\""),
            "expected on-chain dep, got:\n{}",
            rewritten,
        );
        assert!(
            rewritten
                .contains("0x7e783b349d3e89cf5931af376ebeadbfab855b3fa239b7ada8f5a92fbea6b387"),
            "expected dep address, got:\n{}",
            rewritten,
        );
        assert!(
            rewritten.contains("https://github.com/aptos-labs/aptos-core.git"),
            "untouched git dep should remain, got:\n{}",
            rewritten,
        );
    }

    #[test]
    fn no_rewrite_when_no_onchain_deps() {
        let manifest = "[package]\nname = \"X\"\nversion = \"0.0.0\"\n";
        assert!(rewrite_manifest_with_onchain_deps(manifest, &[], "https://node/v1").is_none());
    }
}
