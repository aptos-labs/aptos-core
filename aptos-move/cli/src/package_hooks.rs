// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::stored_package::CachedPackageRegistry;
use aptos_cli_common::load_account_arg;
use aptos_framework::UPGRADE_POLICY_CUSTOM_FIELD;
use futures::executor::block_on;
use move_package::{
    compilation::package_layout::CompiledPackageLayout, package_hooks::PackageHooks,
    source_package::parsed_manifest::CustomDepInfo,
};
use move_symbol_pool::Symbol;
use reqwest::Url;

pub fn register_package_hooks() {
    move_package::package_hooks::register_package_hooks(Box::new(AptosPackageHooks {}))
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
            false,
        )
        .await?;
        let package = registry.get_package(info.package_name).await?;
        package.save_package_to_disk(info.download_to.as_path())
    } else {
        Ok(())
    }
}
