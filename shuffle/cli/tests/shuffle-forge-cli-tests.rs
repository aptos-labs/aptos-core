// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

mod common;

use forge::{
    forge_main, AdminContext, AdminTest, ForgeConfig, LocalFactory, Options, Result, Test,
};

use common::{bootstrap_shuffle_project, ShuffleTestHelper};
use smoke_test::scripts_and_modules::enable_open_publishing;
use std::{os::unix::prelude::ExitStatusExt, process::ExitStatus};

pub const BINARY: &str = env!("CARGO_BIN_EXE_shuffle");

fn main() -> Result<()> {
    let tests = ForgeConfig::default().with_admin_tests(&[
        &TransactionsWithoutAccount,
        &TransactionsWithNetworkRawAddressFlags,
    ]);
    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}

pub struct TransactionsWithoutAccount;

impl Test for TransactionsWithoutAccount {
    fn name(&self) -> &'static str {
        "shuffle::transactions-without-account"
    }
}

impl AdminTest for TransactionsWithoutAccount {
    fn run<'t>(&self, ctx: &mut AdminContext<'t>) -> Result<()> {
        let helper = ShuffleTestHelper::new(ctx.chain_info())?;
        let home_path_string = helper.home_path().to_string_lossy().to_string();
        let client = ctx.client();
        let factory = ctx.chain_info().transaction_factory();
        enable_open_publishing(&client, &factory, ctx.chain_info().root_account())?;
        let output = std::process::Command::new(BINARY)
            .args([
                "--home-path",
                home_path_string.as_str(),
                "transactions",
                "--raw",
                "--network",
                "forge",
            ])
            .output()?;
        let std_err = String::from_utf8(output.stderr)?;

        assert!(std_err
            .contains("Error: An account hasn't been created yet! Run shuffle account first\n"));
        assert_eq!(output.status, ExitStatus::from_raw(256));
        Ok(())
    }
}

pub struct TransactionsWithNetworkRawAddressFlags;

impl Test for TransactionsWithNetworkRawAddressFlags {
    fn name(&self) -> &'static str {
        "shuffle::transactions-with-networks-raw-address-flags"
    }
}

impl AdminTest for TransactionsWithNetworkRawAddressFlags {
    fn run<'t>(&self, ctx: &mut AdminContext<'t>) -> Result<()> {
        let helper = bootstrap_shuffle_project(ctx)?;
        let home_path_string = helper.home_path().to_string_lossy().to_string();
        let output = std::process::Command::new(BINARY)
            .args([
                "--home-path",
                home_path_string.as_str(),
                "transactions",
                "--raw",
                "--network",
                "forge",
                "--address",
                helper
                    .network_home()
                    .get_latest_address()?
                    .to_hex_literal()
                    .as_str(),
            ])
            .output()?;
        let std_out = String::from_utf8(output.stdout)?;
        assert_modules_appear_in_txns(std_out.as_str());
        assert_eq!(output.status, ExitStatus::from_raw(0));
        Ok(())
    }
}

fn assert_modules_appear_in_txns(std_out: &str) {
    assert!(std_out.contains(r#""sequence_number":"0""#));
    assert!(std_out.contains(r#""sequence_number":"1""#));
    assert!(std_out.contains(r#""sequence_number":"2""#));
    assert!(std_out.contains(r#""name":"Message""#));
    assert!(std_out.contains(r#""name":"TestNFT""#));
    assert!(std_out.contains(r#""name":"NFT""#));
}
