// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::load_account_arg;
use crate::move_tool::CachedPackageRegistry;
use framework::UPGRADE_POLICY_CUSTOM_FIELD;
use futures::executor::block_on;
use move_deps::move_package::compilation::package_layout::CompiledPackageLayout;
use move_deps::move_package::package_hooks::PackageHooks;
use move_deps::move_package::source_package::parsed_manifest::CustomDepInfo;
use move_deps::move_symbol_pool::Symbol;
use reqwest::Url;

pub fn register_package_hooks() {
    move_deps::move_package::package_hooks::register_package_hooks(Box::new(AptosPackageHooks {}))
}

struct AptosPackageHooks {}

impl PackageHooks for AptosPackageHooks {
    fn custom_package_info_fields(&self) -> Vec<String> {
        vec![UPGRADE_POLICY_CUSTOM_FIELD.to_string()]
    }

    fn custom_dependency_key(&self) -> Option<String> {
        Some("aptos".to_string())
    }

    fn resolve_custom_dependency(
        &self,
        _dep_name: Symbol,
        info: &CustomDepInfo,
    ) -> anyhow::Result<()> {
        block_on(maybe_download_package(info))
    }
}

async fn maybe_download_package(info: &CustomDepInfo) -> anyhow::Result<()> {
    if !info
        .download_to
        .join(CompiledPackageLayout::BuildInfo.path())
        .exists()
    {
        let registry = CachedPackageRegistry::create(
            Url::parse(info.node_url.as_str())?,
            load_account_arg(info.package_address.as_str())?,
        )
        .await?;
        let package = registry.get_package(info.package_name).await?;
        package.save_package_to_disk(info.download_to.as_path())
    } else {
        Ok(())
    }
}
