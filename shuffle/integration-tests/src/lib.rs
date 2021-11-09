// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

mod helper;

use crate::helper::ShuffleTestHelper;
use forge::{AdminContext, AdminTest, Result, Test};
use move_cli::package::cli::UnitTestResult;
use smoke_test::scripts_and_modules::enable_open_publishing;
use tokio::runtime::Runtime;

pub struct SamplePackageEndToEnd;

impl Test for SamplePackageEndToEnd {
    fn name(&self) -> &'static str {
        "shuffle::sample-package-end-to-end"
    }
}

impl AdminTest for SamplePackageEndToEnd {
    fn run<'t>(&self, ctx: &mut AdminContext<'t>) -> Result<()> {
        let helper = bootstrap_shuffle(ctx)?;
        let unit_test_result = shuffle::test::run_move_unit_tests(&helper.project_path())?;
        let exit_status = shuffle::test::run_deno_test(
            helper.home(),
            &helper.project_path(),
            &helper
                .home()
                .get_network_struct_from_toml(shuffle::shared::LOCALHOST_NAME)?,
            helper.network_home().get_test_key_path(),
            helper.network_home().get_test_address()?,
        )?;

        assert!(matches!(unit_test_result, UnitTestResult::Success));
        assert!(exit_status.success());
        Ok(())
    }
}

pub struct TypescriptSdkIntegration;

impl Test for TypescriptSdkIntegration {
    fn name(&self) -> &'static str {
        "shuffle::typescript-sdk-integration"
    }
}

impl AdminTest for TypescriptSdkIntegration {
    fn run<'t>(&self, ctx: &mut AdminContext<'t>) -> Result<()> {
        let helper = bootstrap_shuffle(ctx)?;
        let exit_status = shuffle::test::run_deno_test_at_path(
            helper.home(),
            &helper.project_path(),
            &helper
                .home()
                .get_network_struct_from_toml(shuffle::shared::LOCALHOST_NAME)?,
            helper.network_home().get_test_key_path(),
            helper.network_home().get_test_address()?,
            &helper.project_path().join("integration"),
        )?;
        assert!(exit_status.success());
        Ok(())
    }
}

fn bootstrap_shuffle(ctx: &mut AdminContext<'_>) -> Result<ShuffleTestHelper> {
    let client = ctx.client();
    let factory = ctx.chain_info().transaction_factory();
    enable_open_publishing(&client, &factory, ctx.chain_info().root_account())?;

    let helper = ShuffleTestHelper::new()?;
    helper.create_project()?;

    let new_account = ctx.random_account();
    let tc = ctx.chain_info().treasury_compliance_account();
    helper.create_accounts(tc, new_account, factory, client)?;

    let rt = Runtime::new().unwrap();
    let handle = rt.handle().clone();
    handle.block_on(helper.deploy_project(ctx.chain_info().rest_api()))?;
    Ok(helper)
}
