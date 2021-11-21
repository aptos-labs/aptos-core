// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

mod helper;

use crate::helper::ShuffleTestHelper;
use forge::{AdminContext, AdminTest, Result, Test};
use move_cli::package::cli::UnitTestResult;
use shuffle::shared::DevApiClient;
use smoke_test::scripts_and_modules::enable_open_publishing;
use std::str::FromStr;
use tokio::runtime::Runtime;
use url::Url;

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
            helper.network(),
            helper.network_home().get_latest_account_key_path(),
            helper.network_home().get_latest_address()?,
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
            helper.network(),
            helper.network_home().get_latest_account_key_path(),
            helper.network_home().get_latest_address()?,
            &helper.project_path().join("integration"),
        )?;
        assert!(exit_status.success());
        Ok(())
    }
}

fn bootstrap_shuffle(ctx: &mut AdminContext<'_>) -> Result<ShuffleTestHelper> {
    let client = ctx.client();
    let dev_api_client = DevApiClient::new(
        reqwest::Client::new(),
        Url::from_str(ctx.chain_info().rest_api())?,
    )?;
    let factory = ctx.chain_info().transaction_factory();
    enable_open_publishing(&client, &factory, ctx.chain_info().root_account())?;

    let helper = ShuffleTestHelper::new(ctx.chain_info())?;
    helper.create_project()?;

    // let mut account = ctx.random_account(); // TODO: Support arbitrary addresses
    let mut account = ShuffleTestHelper::hardcoded_0x2416_account(&client)?;
    let tc = ctx.chain_info().treasury_compliance_account();
    let rt = Runtime::new().unwrap();
    let handle = rt.handle().clone();

    handle.block_on(helper.create_account(tc, &account, factory, &dev_api_client))?;
    handle.block_on(helper.deploy_project(&mut account, ctx.chain_info().rest_api()))?;
    Ok(helper)
}
