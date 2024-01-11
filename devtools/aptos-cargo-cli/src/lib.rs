// Copyright © Aptos Foundation

mod cargo;
mod common;

use cargo::Cargo;
use clap::{Args, Parser, Subcommand};
pub use common::SelectedPackageArgs;
use log::trace;

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
    Xclippy(CommonArgs),
    Fmt(CommonArgs),
    Nextest(CommonArgs),
    Test(CommonArgs),
}

impl AptosCargoCommand {
    fn command(&self) -> &'static str {
        match self {
            AptosCargoCommand::Check(_) => "check",
            AptosCargoCommand::Xclippy(_) => "clippy",
            AptosCargoCommand::Fmt(_) => "fmt",
            AptosCargoCommand::Nextest(_) => "nextest",
            AptosCargoCommand::Test(_) => "test",
        }
    }

    fn command_args(&self) -> &CommonArgs {
        match self {
            AptosCargoCommand::Check(args) => args,
            AptosCargoCommand::Xclippy(args) => args,
            AptosCargoCommand::Fmt(args) => args,
            AptosCargoCommand::Nextest(args) => args,
            AptosCargoCommand::Test(args) => args,
        }
    }

    fn extra_opts(&self) -> Option<&[&str]> {
        match self {
            AptosCargoCommand::Xclippy(_) => Some(&[
                "-Dwarnings",
                "-Wclippy::all",
                "-Aclippy::upper_case_acronyms",
                "-Aclippy::enum-variant-names",
                "-Aclippy::result-large-err",
                "-Aclippy::mutable-key-type",
            ]),
            _ => None,
        }
    }

    fn split_args(&self) -> (Vec<String>, Vec<String>) {
        self.command_args().args()
    }

    pub fn execute(&self, package_args: &SelectedPackageArgs) -> anyhow::Result<()> {
        let (mut direct_args, mut push_through_args) = self.split_args();

        trace!("parsed direct_args: {:?}", direct_args);
        trace!("parsed push_through_args: {:?}", push_through_args);

        let packages = package_args.compute_packages()?;

        trace!("affected packages: {:?}", packages);

        for p in packages {
            direct_args.push("-p".into());
            direct_args.push(p);
        }

        if let Some(opts) = self.extra_opts() {
            for &opt in opts {
                push_through_args.push(opt.into());
            }
        }

        trace!("final direct_args: {:?}", direct_args);
        trace!("final push_through_args: {:?}", push_through_args);

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
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}

impl AptosCargoCli {
    pub fn execute(&self) -> anyhow::Result<()> {
        self.cmd.execute(&self.package_args)
    }
}
