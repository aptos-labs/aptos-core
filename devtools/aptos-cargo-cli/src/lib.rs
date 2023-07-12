// Copyright © Aptos Foundation

mod cargo;
mod common;

use cargo::Cargo;
use clap::{Args, Parser, Subcommand};
pub use common::SelectedPackageArgs;

#[derive(Args, Clone, Debug)]
#[command(disable_help_flag = true)]
pub struct CommonArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

impl CommonArgs {
    fn args(&self) -> (Vec<String>, Vec<String>) {
        if let Some(index) = self.args.iter().position(|arg| arg == "--") {
            let (left, right) = self.args.split_at(index);
            (left.to_vec(), right[1..].to_vec())
        } else {
            (self.args.clone(), vec![])
        }
    }
}

#[derive(Clone, Subcommand, Debug)]
pub enum AptosCargoCommand {
    Check(CommonArgs),
    Clippy(CommonArgs),
    Fmt(CommonArgs),
    Nextest(CommonArgs),
    Test(CommonArgs),
}

impl AptosCargoCommand {
    fn command(&self) -> &'static str {
        match self {
            AptosCargoCommand::Check(_) => "check",
            AptosCargoCommand::Clippy(_) => "clippy",
            AptosCargoCommand::Fmt(_) => "fmt",
            AptosCargoCommand::Nextest(_) => "nextest",
            AptosCargoCommand::Test(_) => "test",
        }
    }

    fn command_args(&self) -> &CommonArgs {
        match self {
            AptosCargoCommand::Check(args) => args,
            AptosCargoCommand::Clippy(args) => args,
            AptosCargoCommand::Fmt(args) => args,
            AptosCargoCommand::Nextest(args) => args,
            AptosCargoCommand::Test(args) => args,
        }
    }

    fn split_args(&self) -> (Vec<String>, Vec<String>) {
        self.command_args().args()
    }

    pub fn execute(&self, package_args: &SelectedPackageArgs) -> anyhow::Result<()> {
        let (mut direct_args, push_through_args) = self.split_args();

        let packages = package_args.compute_packages()?;

        for p in packages {
            direct_args.push("-p".into());
            direct_args.push(p);
        }

        Cargo::command(self.command())
            .args(direct_args)
            .pass_through(push_through_args)
            .run();
        Ok(())
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(author, version)]
pub struct AptosCargoCli {
    #[command(subcommand)]
    cmd: AptosCargoCommand,
    #[command(flatten)]
    package_args: SelectedPackageArgs,
}

impl AptosCargoCli {
    pub fn execute(&self) -> anyhow::Result<()> {
        self.cmd.execute(&self.package_args)
    }
}
