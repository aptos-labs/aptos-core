// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{common::types::load_account_arg, move_tool::CachedPackageRegistry};
use velor_framework::UPGRADE_POLICY_CUSTOM_FIELD;
use futures::executor::block_on;
use move_package::{
    compilation::package_layout::CompiledPackageLayout, package_hooks::PackageHooks,
    source_package::parsed_manifest::CustomDepInfo,
};
use move_symbol_pool::Symbol;
use reqwest::Url;

pub fn register_package_hooks() {
    move_package::package_hooks::register_package_hooks(Box::new(VelorPackageHooks {}))
}

struct VelorPackageHooks {}

impl PackageHooks for VelorPackageHooks {
    fn custom_package_info_fields(&self) -> Vec<String> {
        vec![UPGRADE_POLICY_CUSTOM_FIELD.to_string()]
    }

    fn custom_dependency_key(&self) -> Option<String> {
        Some("velor".to_string())
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
