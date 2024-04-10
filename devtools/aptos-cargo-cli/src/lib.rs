// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod cargo;
mod common;

use cargo::Cargo;
use clap::{Args, Parser, Subcommand};
pub use common::SelectedPackageArgs;
use determinator::Utf8Paths0;
use log::{debug, trace};

// The targeted unit test packages to ignore (these will be run separately, by other jobs)
const TARGETED_TEST_PACKAGES_TO_IGNORE: [&str; 2] = ["aptos-testcases", "smoke-test"];

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
    AffectedPackages(CommonArgs),
    ChangedFiles(CommonArgs),
    Check(CommonArgs),
    Xclippy(CommonArgs),
    Fmt(CommonArgs),
    Nextest(CommonArgs),
    TargetedUnitTests(CommonArgs),
    Test(CommonArgs),
}

impl AptosCargoCommand {
    fn command(&self) -> &'static str {
        match self {
            AptosCargoCommand::Check(_) => "check",
            AptosCargoCommand::Xclippy(_) => "clippy",
            AptosCargoCommand::Fmt(_) => "fmt",
            AptosCargoCommand::Nextest(_) => "nextest",
            AptosCargoCommand::TargetedUnitTests(_) => "nextest", // Invoke the nextest command directly
            AptosCargoCommand::Test(_) => "test",
            command => panic!("Unsupported command attempted! Command: {:?}", command),
        }
    }

    fn command_args(&self) -> &CommonArgs {
        match self {
            AptosCargoCommand::AffectedPackages(args) => args,
            AptosCargoCommand::ChangedFiles(args) => args,
            AptosCargoCommand::Check(args) => args,
            AptosCargoCommand::Xclippy(args) => args,
            AptosCargoCommand::Fmt(args) => args,
            AptosCargoCommand::Nextest(args) => args,
            AptosCargoCommand::TargetedUnitTests(args) => args,
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

    fn get_args_and_affected_packages(
        &self,
        package_args: &SelectedPackageArgs,
    ) -> anyhow::Result<(Vec<String>, Vec<String>, Vec<String>)> {
        // Parse the args
        let (direct_args, push_through_args) = self.parse_args();

        // Compute the affected packages
        let packages = package_args.compute_target_packages()?;
        trace!("affected packages: {:?}", packages);

        // Return the parsed args and packages
        Ok((direct_args, push_through_args, packages))
    }

    fn parse_args(&self) -> (Vec<String>, Vec<String>) {
        // Parse the args
        let (direct_args, push_through_args) = self.command_args().args();

        // Trace log for debugging
        trace!("parsed direct_arg`s: {:?}", direct_args);
        trace!("parsed push_through_args: {:?}", push_through_args);

        (direct_args, push_through_args)
    }

    pub fn execute(&self, package_args: &SelectedPackageArgs) -> anyhow::Result<()> {
        match self {
            AptosCargoCommand::AffectedPackages(_) => {
                // Calculate and display the affected packages
                let packages = package_args.compute_target_packages()?;
                output_affected_packages(packages)
            },
            AptosCargoCommand::ChangedFiles(_) => {
                // Calculate and display the changed files
                let (_, _, changed_files) = package_args.identify_changed_files()?;
                output_changed_files(changed_files)
            },
            AptosCargoCommand::TargetedUnitTests(_) => {
                // Calculate and run the targeted unit tests.
                // Start by fetching the arguments and affected packages.
                let (mut direct_args, push_through_args, packages) =
                    self.get_args_and_affected_packages(package_args)?;

                // Add each affected package to the arguments, but filter out
                // the packages that should not be run as unit tests.
                let mut found_package_to_test = false;
                for package_path in packages {
                    // Extract the package name from the full path
                    let package_name = package_path.split('#').last().unwrap();

                    // Only add the package if it is not in the ignore list
                    if TARGETED_TEST_PACKAGES_TO_IGNORE.contains(&package_name) {
                        debug!(
                            "Ignoring package when running targeted-unit-tests: {:?}",
                            package_name
                        );
                    } else {
                        // Add the arguments for the package
                        direct_args.push("-p".into());
                        direct_args.push(package_path);

                        // Mark that we found a package to test
                        found_package_to_test = true;
                    }
                }

                // Create and run the command if we found a package to test
                if found_package_to_test {
                    println!("Running the targeted unit tests...");
                    return self.create_and_run_command(direct_args, push_through_args);
                }

                println!("Skipping targeted unit tests because no packages were affected to test.");
                Ok(())
            },
            _ => {
                // Otherwise, we need to parse and run the command.
                // Start by fetching the arguments and affected packages.
                let (mut direct_args, mut push_through_args, packages) =
                    self.get_args_and_affected_packages(package_args)?;

                // Add each affected package to the arguments
                for package_path in packages {
                    direct_args.push("-p".into());
                    direct_args.push(package_path);
                }

                // Add any additional arguments
                if let Some(opts) = self.extra_opts() {
                    for &opt in opts {
                        push_through_args.push(opt.into());
                    }
                }

                // Create and run the command
                self.create_and_run_command(direct_args, push_through_args)
            },
        }
    }

    fn create_and_run_command(
        &self,
        direct_args: Vec<String>,
        push_through_args: Vec<String>,
    ) -> anyhow::Result<()> {
        // Output the final arguments before running the command
        trace!("final direct_args: {:?}", direct_args);
        trace!("final push_through_args: {:?}", push_through_args);

        // Construct and run the final command
        let mut command = Cargo::command(self.command());
        command.args(direct_args).pass_through(push_through_args);
        command.run();

        Ok(())
    }
}

/// Outputs the specified affected packages
fn output_affected_packages(packages: Vec<String>) -> anyhow::Result<()> {
    // Output the affected packages (if they exist)
    if packages.is_empty() {
        println!("No packages were affected!");
    } else {
        println!("Affected packages detected:");
        for package in packages {
            println!("\t{:?}", package)
        }
    }
    Ok(())
}

/// Outputs the changed files from the given package args
fn output_changed_files(changed_files: Utf8Paths0) -> anyhow::Result<()> {
    // Output the results
    let mut changes_detected = false;
    for (index, file) in changed_files.into_iter().enumerate() {
        if index == 0 {
            println!("Changed files detected:"); // Only print this if changes were detected!
            changes_detected = true;
        }
        println!("\t{:?}", file)
    }

    // If no changes were detected, make it obvious
    if !changes_detected {
        println!("No changes were detected!")
    }

    Ok(())
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
