// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::aptos::move_test_helpers;
use forge::{AptosContext, AptosTest, Result, Test};

pub struct ModulePublish;

impl Test for ModulePublish {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::module-publish"
    }
}

#[async_trait::async_trait]
impl AptosTest for ModulePublish {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let base_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/aptos/move_modules/");

        // module publish should call init_module by default to create the resource
        move_test_helpers::publish_code(ctx, base_path.clone()).await?;
        ctx.client()
            .get_account_resource(
                ctx.root_account().address(),
                "0xA550C18::HelloWorld::ModuleData",
            )
            .await
            .unwrap();

        // republish should fail
        move_test_helpers::publish_code(ctx, base_path.clone())
            .await
            .unwrap_err();

        Ok(())
    }
}
