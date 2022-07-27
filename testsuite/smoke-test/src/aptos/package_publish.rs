// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::aptos::move_test_helpers;
use aptos_vm::move_vm_ext::UpgradePolicy;
use forge::{AptosContext, AptosTest, Result, Test};

/// Tests for new package publishing transaction
pub struct PackagePublish;

impl Test for PackagePublish {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::package-publish"
    }
}

#[async_trait::async_trait]
impl AptosTest for PackagePublish {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let base_path_v1 = base_dir.join("src/aptos/package_publish_modules_v1/");
        let base_path_v2 = base_dir.join("src/aptos/package_publish_modules_v2/");
        let base_path_v3 = base_dir.join("src/aptos/package_publish_modules_v3/");

        move_test_helpers::publish_package(ctx, base_path_v1, "test", UpgradePolicy::compat())
            .await?;
        // v2 is downwards compatible to v1
        move_test_helpers::publish_package(ctx, base_path_v2, "test", UpgradePolicy::compat())
            .await?;
        // v3 is not downwards compatible to v2
        move_test_helpers::publish_package(ctx, base_path_v3, "test", UpgradePolicy::compat())
            .await
            .unwrap_err();

        Ok(())
    }
}
