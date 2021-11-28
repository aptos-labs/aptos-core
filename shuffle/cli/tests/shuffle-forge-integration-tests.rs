// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

mod common;

use common::bootstrap_shuffle_project;
use forge::{
    forge_main, AdminContext, AdminTest, ForgeConfig, LocalFactory, Options, Result, Test,
};
use move_cli::package::cli::UnitTestResult;
use shuffle::context::UserContext;

fn main() -> Result<()> {
    let tests = ForgeConfig::default()
        .with_admin_tests(&[&SamplePackageEndToEnd, &TypescriptSdkIntegration]);
    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}

pub struct SamplePackageEndToEnd;

impl Test for SamplePackageEndToEnd {
    fn name(&self) -> &'static str {
        "shuffle::sample-package-end-to-end"
    }
}

impl AdminTest for SamplePackageEndToEnd {
    fn run<'t>(&self, ctx: &mut AdminContext<'t>) -> Result<()> {
        let helper = bootstrap_shuffle_project(ctx)?;
        let unit_test_result = shuffle::test::run_move_unit_tests(&helper.project_path())?;
        let user = UserContext::new(
            "latest",
            helper.network_home().get_latest_address()?,
            &helper.network_home().get_latest_account_key_path(),
        );
        let exit_status = shuffle::test::run_deno_test(
            helper.home(),
            &helper.project_path(),
            helper.network(),
            &[&user],
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
        let helper = bootstrap_shuffle_project(ctx)?;
        let user = UserContext::new(
            "latest",
            helper.network_home().get_latest_address()?,
            &helper.network_home().get_latest_account_key_path(),
        );
        let exit_status = shuffle::test::run_deno_test_at_path(
            helper.home(),
            &helper.project_path(),
            helper.network(),
            &[&user],
            &helper.project_path().join("integration"),
        )?;
        assert!(exit_status.success());
        Ok(())
    }
}
