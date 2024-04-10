// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod cargo;
mod common;

use cargo::Cargo;
use clap::{command, Args, Parser, Subcommand};
pub use common::SelectedPackageArgs;
use determinator::Utf8Paths0;
use log::{debug, trace};

// The CLI package name to match against for targeted CLI tests
const APTOS_CLI_PACKAGE_NAME: &str = "aptos";

// The relevant file paths to monitor when deciding to run the targeted compiler V2 tests
const TARGETED_COMPILER_V2_FILE_PATHS: [&str; 7] = [
    "aptos-move/aptos-transactional-test-harness",
    "aptos-move/e2e-move-tests",
    "aptos-move/framework",
    "aptos-move/move-examples",
    "third_party/move",
    ".github/workflows/move-test-compiler-v2.yaml",
    ".github/actions/move-tests-compiler-v2",
];

// The relevant packages to test when running the targeted compiler V2 tests
const TARGETED_COMPILER_V2_PACKAGES_TO_TEST: [&str; 2] = ["aptos-framework", "e2e-move-tests"];

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
    TargetedCLITests(CommonArgs),
    TargetedCompilerV2Tests(CommonArgs),
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
            AptosCargoCommand::TargetedCLITests(args) => args,
            AptosCargoCommand::TargetedCompilerV2Tests(args) => args,
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
            AptosCargoCommand::TargetedCLITests(_) => {
                // Run the targeted CLI tests (if necessary).
                // First, start by calculating the affected packages.
                let packages = package_args.compute_target_packages()?;

                // Check if the affected packages contains the Aptos CLI
                let mut cli_affected = false;
                for package_path in packages {
                    // Extract the package name from the full path
                    let package_name = get_package_name_from_path(&package_path);

                    // Check if the package is the Aptos CLI
                    if package_name == APTOS_CLI_PACKAGE_NAME {
                        cli_affected = true; // The Aptos CLI was affected
                        break;
                    }
                }

                // If the Aptos CLI is affected, run the targeted CLI tests
                if cli_affected {
                    println!("Running the targeted CLI tests...");
                    return run_targeted_cli_tests();
                }

                // Otherwise, skip the CLI tests
                println!("Skipping CLI tests as the Aptos CLI package was not affected!");
                Ok(())
            },
            AptosCargoCommand::TargetedCompilerV2Tests(_) => {
                // Run the targeted compiler v2 tests (if necessary).
                // Start by calculating the changed files and affected packages.
                let (_, _, changed_files) = package_args.identify_changed_files()?;
                let (direct_args, push_through_args, packages) =
                    self.get_args_and_affected_packages(package_args)?;

                // Check if the changed files contain any of the relevant compiler v2 file paths
                let mut relevant_file_changed = false;
                for file_path in changed_files.into_iter() {
                    for compiler_v2_file_path in TARGETED_COMPILER_V2_FILE_PATHS.iter() {
                        if file_path.to_string().contains(compiler_v2_file_path) {
                            relevant_file_changed = true; // A relevant file was changed
                            break;
                        }
                    }
                }

                // Check if the affected packages contains the relevant compiler v2 packages
                let mut relevant_packages_changed = false;
                for package_path in packages {
                    // Extract the package name from the full path
                    let package_name = get_package_name_from_path(&package_path);

                    // Check if the package is a relevant compiler v2 package
                    if TARGETED_COMPILER_V2_PACKAGES_TO_TEST.contains(&package_name.as_str()) {
                        relevant_packages_changed = true; // A relevant package was changed
                        break;
                    }
                }

                // If relevant files or packages were changed, run the targeted compiler v2 tests
                if relevant_file_changed || relevant_packages_changed {
                    println!("Running the targeted compiler v2 tests...");
                    return run_targeted_compiler_v2_tests(direct_args, push_through_args);
                }

                // Otherwise, skip the targeted compiler v2 tests
                println!("Skipping targeted compiler v2 tests because no relevant files or packages were affected!");
                Ok(())
            },
            AptosCargoCommand::TargetedUnitTests(_) => {
                // Run the targeted unit tests (if necessary).
                // Start by calculating the affected packages.
                let (direct_args, push_through_args, packages) =
                    self.get_args_and_affected_packages(package_args)?;

                // Filter out the ignored packages
                let mut packages_to_test = vec![];
                for package_path in packages {
                    // Extract the package name from the full path
                    let package_name = get_package_name_from_path(&package_path);

                    // Only add the package if it is not in the ignore list
                    if TARGETED_TEST_PACKAGES_TO_IGNORE.contains(&package_name.as_str()) {
                        debug!(
                            "Ignoring package when running targeted-unit-tests: {:?}",
                            package_name
                        );
                    } else {
                        packages_to_test.push(package_path); // Add the package to the list
                    }
                }

                // Create and run the command if we found packages to test
                if !packages_to_test.is_empty() {
                    println!("Running the targeted unit tests...");
                    return run_targeted_unit_tests(
                        packages_to_test,
                        direct_args,
                        push_through_args,
                    );
                }

                // Otherwise, skip the targeted unit tests
                println!("Skipping targeted unit tests because no test packages were affected!");
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
        command.run(false);

        Ok(())
    }
}

/// Returns the package name from the given package path
fn get_package_name_from_path(package_path: &str) -> String {
    package_path.split('#').last().unwrap().to_string()
}

/// Runs the targeted CLI tests. This includes building and testing the CLI.
fn run_targeted_cli_tests() -> anyhow::Result<()> {
    // First, run the CLI tests
    let mut command = Cargo::command("test");
    command.args(["-p", APTOS_CLI_PACKAGE_NAME]);
    command.run(false);

    // Next, build the CLI binary
    let mut command = Cargo::command("build");
    command.args(["-p", APTOS_CLI_PACKAGE_NAME]);
    command.run(false);

    // Finally, run the CLI --help command. Here, we ignore the exit status
    // because the CLI will return a non-zero exit status when running --help.
    let mut command = Cargo::command("run");
    command.args(["-p", APTOS_CLI_PACKAGE_NAME]);
    command.run(true);

    Ok(())
}

/// Runs the targeted compiler v2 tests
fn run_targeted_compiler_v2_tests(
    mut direct_args: Vec<String>,
    push_through_args: Vec<String>,
) -> anyhow::Result<()> {
    // Add the compiler v2 packages to test to the arguments
    for package in TARGETED_COMPILER_V2_PACKAGES_TO_TEST.iter() {
        direct_args.push("-p".into());
        direct_args.push(package.to_string());
    }

    // Create the command to run the compiler v2 tests
    let mut command = Cargo::command("nextest");
    command.args(["run"]);
    command.args(direct_args).pass_through(push_through_args);

    // Run the compiler v2 tests
    command.run(false);
    Ok(())
}

/// Runs the targeted unit tests
fn run_targeted_unit_tests(
    packages_to_test: Vec<String>,
    mut direct_args: Vec<String>,
    push_through_args: Vec<String>,
) -> anyhow::Result<()> {
    // Add each package to the arguments
    for package in packages_to_test {
        direct_args.push("-p".into());
        direct_args.push(package);
    }

    // Create the command to run the unit tests
    let mut command = Cargo::command("nextest");
    command.args(["run"]);
    command.args(direct_args).pass_through(push_through_args);

    // Run the unit tests
    command.run(false);
    Ok(())
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
