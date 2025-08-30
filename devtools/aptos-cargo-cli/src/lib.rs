// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod cargo;
mod common;

use crate::common::PACKAGE_NAME_DELIMITER;
use camino::Utf8PathBuf;
use cargo::Cargo;
use clap::{command, Args, Parser, Subcommand};
pub use common::SelectedPackageArgs;
use determinator::Utf8Paths0;
use log::{debug, trace};

// Useful package name constants for targeted tests
const APTOS_CLI_PACKAGE_NAME: &str = "aptos";

// Relevant file paths to monitor when deciding to run the targeted tests.
// Note: these paths should be relative to the root of the `aptos-core` repository,
// and will be transformed into UTF-8 paths for cross-platform compatibility.
const RELEVANT_FILE_PATHS_FOR_COMPILER_V2: [&str; 5] = [
    "aptos-move/aptos-transactional-test-harness",
    "aptos-move/e2e-move-tests",
    "aptos-move/framework",
    "aptos-move/move-examples",
    "third_party/move",
];
const RELEVANT_FILE_PATHS_FOR_EXECUTION_PERFORMANCE_TESTS: [&str; 5] = [
    ".github/workflows/execution-performance.yaml",
    ".github/workflows/workflow-run-execution-performance.yaml",
    "aptos-move/e2e-benchmark",
    "execution/aptos-executor-benchmark",
    "testsuite/single_node_performance.py",
];
const RELEVANT_FILE_PATHS_FOR_FRAMEWORK_UPGRADE_TESTS: [&str; 4] = [
    ".github",
    "testsuite",
    "aptos-move/aptos-release-builder",
    "aptos-move/framework",
];

// Relevant packages to monitor when deciding to run the targeted tests
const RELEVANT_PACKAGES_FOR_COMPILER_V2: [&str; 2] = ["aptos-framework", "e2e-move-tests"];
const RELEVANT_PACKAGES_FOR_EXECUTION_PERFORMANCE_TESTS: [&str; 2] =
    ["aptos-executor-benchmark", "aptos-move-e2e-benchmark"];
const RELEVANT_PACKAGES_FOR_FRAMEWORK_UPGRADE_TESTS: [&str; 2] =
    ["aptos-framework", "aptos-release-builder"];

// The targeted unit test packages to ignore (these will be run separately, by other jobs)
const TARGETED_UNIT_TEST_PACKAGES_TO_IGNORE: [&str; 3] =
    ["aptos-testcases", "smoke-test", "aptos-keyless-circuit"];

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
    CheckMergeBase(CommonArgs),
    Xclippy(CommonArgs),
    Fmt(CommonArgs),
    Nextest(CommonArgs),
    TargetedCLITests(CommonArgs),
    TargetedCompilerV2Tests(CommonArgs),
    TargetedExecutionPerformanceTests(CommonArgs),
    TargetedFrameworkUpgradeTests(CommonArgs),
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
            AptosCargoCommand::CheckMergeBase(args) => args,
            AptosCargoCommand::Xclippy(args) => args,
            AptosCargoCommand::Fmt(args) => args,
            AptosCargoCommand::Nextest(args) => args,
            AptosCargoCommand::TargetedCLITests(args) => args,
            AptosCargoCommand::TargetedCompilerV2Tests(args) => args,
            AptosCargoCommand::TargetedExecutionPerformanceTests(args) => args,
            AptosCargoCommand::TargetedFrameworkUpgradeTests(args) => args,
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
                let affected_package_paths = package_args.compute_target_packages()?;
                output_affected_packages(affected_package_paths)
            },
            AptosCargoCommand::ChangedFiles(_) => {
                // Calculate and display the changed files
                let (_, _, changed_files) = package_args.identify_changed_files()?;
                output_changed_files(changed_files)
            },
            AptosCargoCommand::CheckMergeBase(_) => {
                // Check the merge base
                package_args.check_merge_base()
            },
            AptosCargoCommand::TargetedCLITests(_) => {
                // Run the targeted CLI tests (if necessary).
                // First, start by calculating the affected packages.
                let (direct_args, _, affected_package_paths) =
                    self.get_args_and_affected_packages(package_args)?;

                // Check if the affected packages contains the Aptos CLI
                let mut cli_affected = false;
                for package_path in affected_package_paths {
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
                    return run_targeted_cli_tests(direct_args);
                }

                // Otherwise, skip the CLI tests
                println!("Skipping CLI tests as the Aptos CLI package was not affected!");
                Ok(())
            },
            AptosCargoCommand::TargetedCompilerV2Tests(_) => {
                // Run the targeted compiler v2 tests (if necessary).
                // Start by calculating the changed files and affected packages.
                let (_, _, changed_files) = package_args.identify_changed_files()?;
                let (direct_args, push_through_args, affected_package_paths) =
                    self.get_args_and_affected_packages(package_args)?;

                // Determine if any relevant files or packages were changed
                let relevant_changes_detected = detect_relevant_changes(
                    RELEVANT_FILE_PATHS_FOR_COMPILER_V2.to_vec(),
                    RELEVANT_PACKAGES_FOR_COMPILER_V2.to_vec(),
                    changed_files,
                    affected_package_paths,
                );

                // If relevant changes were detected, run the targeted compiler v2 tests
                if relevant_changes_detected {
                    println!("Running the targeted compiler v2 tests...");
                    return run_targeted_compiler_v2_tests(direct_args, push_through_args);
                }

                // Otherwise, skip the targeted compiler v2 tests
                println!("Skipping targeted compiler v2 tests because no relevant files or packages were affected!");
                Ok(())
            },
            AptosCargoCommand::TargetedExecutionPerformanceTests(_) => {
                // Determine if the execution performance tests should be run.
                // Start by calculating the changed files and affected packages.
                let (_, _, changed_files) = package_args.identify_changed_files()?;
                let (_, _, affected_package_paths) =
                    self.get_args_and_affected_packages(package_args)?;

                // Determine if any relevant files or packages were changed
                let relevant_changes_detected = detect_relevant_changes(
                    RELEVANT_FILE_PATHS_FOR_EXECUTION_PERFORMANCE_TESTS.to_vec(),
                    RELEVANT_PACKAGES_FOR_EXECUTION_PERFORMANCE_TESTS.to_vec(),
                    changed_files,
                    affected_package_paths,
                );

                // Output if relevant changes were detected that require the execution performance
                // test. This will be consumed by Github Actions and used to run the test.
                println!(
                    "Execution performance test required: {}",
                    relevant_changes_detected
                );

                Ok(())
            },
            AptosCargoCommand::TargetedFrameworkUpgradeTests(_) => {
                // Determine if the framework upgrade tests should be run.
                // Start by calculating the changed files and affected packages.
                let (_, _, changed_files) = package_args.identify_changed_files()?;
                let (_, _, affected_package_paths) =
                    self.get_args_and_affected_packages(package_args)?;

                // Determine if any relevant files or packages were changed
                #[allow(unused_assignments)]
                let relevant_changes_detected = detect_relevant_changes(
                    RELEVANT_FILE_PATHS_FOR_FRAMEWORK_UPGRADE_TESTS.to_vec(),
                    RELEVANT_PACKAGES_FOR_FRAMEWORK_UPGRADE_TESTS.to_vec(),
                    changed_files,
                    affected_package_paths,
                );

                // Output if relevant changes were detected that require the framework upgrade
                // test. This will be consumed by Github Actions and used to run the test.
                println!(
                    "Framework upgrade test required: {}",
                    relevant_changes_detected
                );

                Ok(())
            },
            AptosCargoCommand::TargetedUnitTests(_) => {
                // Run the targeted unit tests (if necessary).
                // Start by calculating the affected packages.
                let (direct_args, push_through_args, affected_package_paths) =
                    self.get_args_and_affected_packages(package_args)?;

                // Filter out the ignored packages
                let mut packages_to_test = vec![];
                for package_path in affected_package_paths {
                    // Extract the package name from the full path
                    let package_name = get_package_name_from_path(&package_path);

                    // Only add the package if it is not in the ignore list
                    if TARGETED_UNIT_TEST_PACKAGES_TO_IGNORE.contains(&package_name.as_str()) {
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
                let (mut direct_args, mut push_through_args, affected_package_paths) =
                    self.get_args_and_affected_packages(package_args)?;

                // Add each affected package to the arguments
                for package_path in affected_package_paths {
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

/// Returns true iff relevant changes are detected. This includes: (i) changes
/// to relevant file paths; or (ii) changes to relevant packages.
fn detect_relevant_changes(
    relevant_file_paths: Vec<&str>,
    relevant_package_names: Vec<&str>,
    changed_file_paths: Utf8Paths0,
    affected_package_paths: Vec<String>,
) -> bool {
    // Transform the relevant file paths into UTF-8 paths
    let relevant_file_paths: Vec<Utf8PathBuf> =
        relevant_file_paths.iter().map(Utf8PathBuf::from).collect();

    // Check if the changed files contain any of the relevant paths
    for changed_file_path in changed_file_paths.into_iter() {
        for relevant_file_path in &relevant_file_paths {
            if changed_file_path.starts_with(relevant_file_path.as_path()) {
                return true; // A relevant file was changed
            }
        }
    }

    // Check if the affected packages contain any of the relevant packages
    for package_path in affected_package_paths {
        // Extract the package name from the full path
        let package_name = get_package_name_from_path(&package_path);

        // Check if the package is a relevant package
        if relevant_package_names.contains(&package_name.as_str()) {
            return true; // A relevant package was changed
        }
    }

    false // No relevant changes detected
}

/// Returns the package name from the given package path
fn get_package_name_from_path(package_path: &str) -> String {
    // Verify the package path contains a package delimiter
    if !package_path.contains(PACKAGE_NAME_DELIMITER) {
        panic!(
            "Package path missing delimiter ({}): {}",
            PACKAGE_NAME_DELIMITER, package_path
        );
    }

    // Next, split the package path on the delimiter
    match package_path.split(PACKAGE_NAME_DELIMITER).last() {
        Some(package_name) => {
            if package_name.is_empty() {
                panic!("Failed to extract package name from path: {}", package_path);
            } else {
                package_name.to_string()
            }
        },
        None => panic!(
            "Failed to split package path on delimiter ({}): {:}",
            PACKAGE_NAME_DELIMITER, package_path
        ),
    }
}

/// Runs the targeted CLI tests
fn run_targeted_cli_tests(direct_args: Vec<String>) -> anyhow::Result<()> {
    // First, run the CLI tests
    let mut command = Cargo::command("test");
    command
        .args(direct_args.clone())
        .args(["-p", APTOS_CLI_PACKAGE_NAME]);
    command.run(false);

    // Next, build the CLI binary
    let mut command = Cargo::command("build");
    command
        .args(direct_args.clone())
        .args(["-p", APTOS_CLI_PACKAGE_NAME]);
    command.run(false);

    // Finally, run the CLI --help command. Here, we ignore the exit status
    // because the CLI will return a non-zero exit status when running --help.
    let mut command = Cargo::command("run");
    command
        .args(direct_args)
        .args(["-p", APTOS_CLI_PACKAGE_NAME]);
    command.run(true);

    Ok(())
}

/// Runs the targeted compiler v2 tests
fn run_targeted_compiler_v2_tests(
    mut direct_args: Vec<String>,
    push_through_args: Vec<String>,
) -> anyhow::Result<()> {
    // Add the compiler v2 packages to test to the arguments
    for package in RELEVANT_PACKAGES_FOR_COMPILER_V2.iter() {
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
    command.args(["--no-tests=warn"]); // Don't fail if no tests are run!
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
        println!("Affected packages detected ({:?} total):", packages.len());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_relevant_changes() {
        // Create relevant paths and packages for testing
        let relevant_file_paths = vec![".github/actions/", "aptos-move/", "Cargo.lock", "crates/"];
        let relevant_package_names = vec!["aptos-node", "e2e-move-tests"];

        // Verify that no changes are detected
        let changed_file_paths = Utf8Paths0::from_bytes(b"developer-docs-site/").unwrap();
        let affected_package_paths =
            vec!["file:///home/aptos-core/crates/test-crate#test-crate".into()];
        let relevant_changes_detected = detect_relevant_changes(
            relevant_file_paths.clone(),
            relevant_package_names.clone(),
            changed_file_paths,
            affected_package_paths,
        );
        assert!(!relevant_changes_detected);

        // Verify that file changes are detected correctly
        let changed_file_path =
            Utf8Paths0::from_bytes(b".github///actions/test-action/action.yaml").unwrap();
        let relevant_changes_detected = detect_relevant_changes(
            relevant_file_paths.clone(),
            relevant_package_names.clone(),
            changed_file_path,
            vec![], // No affected packages
        );
        assert!(relevant_changes_detected);

        // Verify that package changes are detected correctly
        let affected_package_paths =
            vec!["file:///home/aptos-core/crates/aptos-node#aptos-node".into()];
        let relevant_changes_detected = detect_relevant_changes(
            relevant_file_paths.clone(),
            relevant_package_names.clone(),
            Utf8Paths0::from_bytes(b"").unwrap(), // No changed files
            affected_package_paths,
        );
        assert!(relevant_changes_detected);

        // Verify that both file and package changes are detected correctly
        let changed_file_path = Utf8Paths0::from_bytes(b"Cargo.lock").unwrap();
        let affected_package_paths =
            vec!["file:///home/aptos-core/crates/e2e-move-tests#e2e-move-tests".into()];
        let relevant_changes_detected = detect_relevant_changes(
            relevant_file_paths.clone(),
            relevant_package_names.clone(),
            changed_file_path,
            affected_package_paths,
        );
        assert!(relevant_changes_detected);
    }

    #[test]
    fn test_detect_relevant_changes_file_paths() {
        // Create relevant file paths for testing
        let relevant_file_paths = vec![".github/actions/", "aptos-move/", "Cargo.lock", "crates/"];

        // Verify that no changes are detected
        let changed_file_paths = vec![
            ".githubb/",
            "aptos-nove/file.txt",
            "Cargo.lockity",
            "/my/crates/",
        ];
        for changed_file_path in changed_file_paths {
            // Convert the changed file path to a UTF-8 path
            let changed_file_path = Utf8Paths0::from_bytes(changed_file_path.as_bytes()).unwrap();

            // Verify that no changes are detected
            let relevant_changes_detected = detect_relevant_changes(
                relevant_file_paths.clone(),
                vec![], // No relevant packages
                changed_file_path,
                vec![], // No affected packages
            );
            assert!(!relevant_changes_detected);
        }

        // Verify that file changes are detected correctly
        let changed_file_paths = vec![
            ".github///actions/test-action/action.yaml",
            "aptos-move/file.txt",
            "Cargo.lock",
            "crates/",
        ];
        for changed_file_path in changed_file_paths {
            // Convert the changed file path to a UTF-8 path
            let changed_file_path = Utf8Paths0::from_bytes(changed_file_path.as_bytes()).unwrap();

            // Verify changes are detected
            let relevant_changes_detected = detect_relevant_changes(
                relevant_file_paths.clone(),
                vec![], // No relevant packages
                changed_file_path,
                vec![], // No affected packages
            );
            assert!(relevant_changes_detected);
        }
    }

    #[test]
    fn test_detect_relevant_changes_package_paths() {
        // Create relevant package names for testing
        let relevant_package_names = vec!["aptos-node", "e2e-move-tests"];

        // Verify that no changes are detected
        let affected_package_paths = vec![
            "file:///home/aptos-core/aptos-mode/tests/e2e-move-tests#test-crate",
            "file:///home/aptos-core/crates/test-crate#other-test-crate",
            "file:///home/aptos-core/crates/other-crate#other-crate",
            "file:///home/aptos-core/aptos-node#other-node-crate",
        ];
        for affected_package_path in affected_package_paths {
            // Verify that no changes are detected
            let relevant_changes_detected = detect_relevant_changes(
                vec![], // No relevant file paths
                relevant_package_names.clone(),
                Utf8Paths0::from_bytes(b"").unwrap(), // No changed files
                vec![affected_package_path.into()],
            );
            assert!(!relevant_changes_detected);
        }

        // Verify that package changes are detected correctly
        let affected_package_paths = vec![
            "file:///home/aptos-core/crates/aptos-node#aptos-node",
            "file:///home/aptos-core/crates/e2e-move-tests#e2e-move-tests",
        ];
        for affected_package_path in affected_package_paths {
            // Verify changes are detected
            let relevant_changes_detected = detect_relevant_changes(
                vec![], // No relevant file paths
                relevant_package_names.clone(),
                Utf8Paths0::from_bytes(b"").unwrap(), // No changed files
                vec![affected_package_path.into()],
            );
            assert!(relevant_changes_detected);
        }
    }

    #[test]
    fn test_get_package_name_from_path() {
        // Create a fully qualified test package path
        let package_name = "test-package-name".to_string();
        let package_path = format!(
            "file:///home/aptos-core/devtools/aptos-cargo-cli#{}",
            package_name
        );

        // Extract the package name from the path and check it
        assert_eq!(get_package_name_from_path(&package_path), package_name);

        // Create a relative test package path
        let package_path = format!("#{}", package_name);

        // Extract the package name from the path and check it
        assert_eq!(get_package_name_from_path(&package_path), package_name);
    }

    #[test]
    #[should_panic(expected = "Failed to extract package name from path")]
    fn test_get_package_name_from_path_empty() {
        // Create a test package path with an empty package name
        let package_path = "file:///home/aptos-core/devtools/aptos-cargo-cli#";

        // Extract the package name from the path (this should panic)
        get_package_name_from_path(package_path);
    }

    #[test]
    #[should_panic(expected = "Package path missing delimiter")]
    fn test_get_package_name_from_path_missing_delimiter() {
        // Create a test package path without a package name
        let package_path = "file:///home/aptos-core/devtools/aptos-cargo-cli";

        // Extract the package name from the path (this should panic)
        get_package_name_from_path(package_path);
    }
}
