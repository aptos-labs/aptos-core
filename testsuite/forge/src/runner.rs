// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// TODO going to remove random seed once cluster deployment supports re-run genesis
use crate::{
    config::ForgeConfig,
    observer::junit::JunitTestObserver,
    result::{TestResult, TestSummary},
    success_criteria::CriteriaCheckerErrors,
    AdminContext, AdminTest, AptosContext, AptosTest, CoreContext, Factory, NetworkContext,
    NetworkContextSynchronizer, NetworkTest, ShouldFail, Test, TestReport, Version,
    NAMESPACE_CLEANUP_DURATION_BUFFER_SECS,
};
use anyhow::{bail, format_err, Error, Result};
use aptos_config::config::NodeConfig;
use clap::{Parser, ValueEnum};
use rand::{rngs::OsRng, Rng, SeedableRng};
use std::{
    io::{self, Write},
    num::NonZeroUsize,
    process,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use sugars::boxed;
use tokio::runtime::Runtime;

const KUBERNETES_SERVICE_HOST: &str = "KUBERNETES_SERVICE_HOST";
pub const FORGE_RUNNER_MODE: &str = "FORGE_RUNNER_MODE";

#[derive(Debug, Parser)]
#[clap(about = "Forged in Fire", styles = aptos_cli_common::aptos_cli_style())]
pub struct Options {
    /// The FILTER string is tested against the name of all tests, and only those tests whose names
    /// contain the filter are run.
    filter: Option<String>,
    #[clap(long = "exact")]
    /// Exactly match filters rather than by substring
    filter_exact: bool,
    #[allow(dead_code)]
    #[clap(long, default_value = "1", env = "RUST_TEST_THREADS")]
    /// NO-OP: unsupported option, exists for compatibility with the default test harness
    /// Number of threads used for running tests in parallel
    test_threads: NonZeroUsize,
    #[allow(dead_code)]
    #[clap(short = 'q', long)]
    /// NO-OP: unsupported option, exists for compatibility with the default test harness
    quiet: bool,
    #[allow(dead_code)]
    #[clap(long)]
    /// NO-OP: unsupported option, exists for compatibility with the default test harness
    nocapture: bool,
    #[clap(long)]
    /// List all tests
    pub list: bool,
    #[clap(long)]
    /// List or run ignored tests
    ignored: bool,
    #[clap(long)]
    /// Include ignored tests when listing or running tests
    include_ignored: bool,
    /// Configure formatting of output:
    ///   pretty = Print verbose output;
    ///   terse = Display one character per test;
    ///   (json is unsupported, exists for compatibility with the default test harness)
    #[clap(long, value_enum, ignore_case = true, default_value_t = Format::Pretty)]
    format: Format,
    #[allow(dead_code)]
    #[clap(short = 'Z')]
    /// NO-OP: unsupported option, exists for compatibility with the default test harness
    /// -Z unstable-options Enable nightly-only flags:
    ///                     unstable-options = Allow use of experimental features
    z_unstable_options: Option<String>,
    #[allow(dead_code)]
    #[clap(long)]
    /// NO-OP: unsupported option, exists for compatibility with the default test harness
    /// Show captured stdout of successful tests
    show_output: bool,
    /// Retain debug logs and above for all nodes instead of just the first 5 nodes
    #[clap(long, default_value = "false", env = "FORGE_RETAIN_DEBUG_LOGS")]
    retain_debug_logs: bool,
    /// Optional path to write junit xml test report
    #[clap(long, env = "FORGE_JUNIT_XML_PATH")]
    junit_xml_path: Option<String>,
}

impl Options {
    pub fn parse() -> Self {
        Parser::parse()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum Format {
    #[default]
    Pretty,
    Terse,
    Json,
}

pub fn forge_main<F: Factory>(tests: ForgeConfig, factory: F, options: &Options) -> Result<()> {
    let forge = Forge::new(options, tests, Duration::from_secs(30), factory);

    if options.list {
        forge.list()?;

        return Ok(());
    }

    match forge.run() {
        Ok(..) => Ok(()),
        Err(e) => {
            eprintln!("Failed to run tests:\n{}", e);
            process::exit(101); // Exit with a non-zero exit code if tests failed
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitialVersion {
    Oldest,
    Newest,
}

pub type NodeConfigFn = Arc<dyn Fn(&mut serde_yaml::Value) + Send + Sync>;
pub type GenesisConfigFn = Arc<dyn Fn(&mut serde_yaml::Value) + Send + Sync>;
/// override_config, base_config (see OverrideNodeConfig)
pub type OverrideNodeConfigFn = Arc<dyn Fn(&mut NodeConfig, &mut NodeConfig) + Send + Sync>;

#[derive(Clone, Copy, Default)]
pub struct NodeResourceOverride {
    pub cpu_cores: Option<usize>,
    pub memory_gib: Option<usize>,
    pub storage_gib: Option<usize>,
}

// Workaround way to implement all_tests, for:
// error[E0658]: cannot cast `dyn interface::admin::AdminTest` to `dyn interface::test::Test`, trait upcasting coercion is experimental
pub enum AnyTestRef<'a> {
    Aptos(&'a dyn AptosTest),
    Admin(&'a dyn AdminTest),
    Network(&'a dyn NetworkTest),
}

impl Test for AnyTestRef<'_> {
    fn name(&self) -> &'static str {
        match self {
            AnyTestRef::Aptos(t) => t.name(),
            AnyTestRef::Admin(t) => t.name(),
            AnyTestRef::Network(t) => t.name(),
        }
    }

    fn ignored(&self) -> bool {
        match self {
            AnyTestRef::Aptos(t) => t.ignored(),
            AnyTestRef::Admin(t) => t.ignored(),
            AnyTestRef::Network(t) => t.ignored(),
        }
    }

    fn should_fail(&self) -> ShouldFail {
        match self {
            AnyTestRef::Aptos(t) => t.should_fail(),
            AnyTestRef::Admin(t) => t.should_fail(),
            AnyTestRef::Network(t) => t.should_fail(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForgeRunnerMode {
    Local,
    K8s,
}

impl FromStr for ForgeRunnerMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "local" => Ok(ForgeRunnerMode::Local),
            "k8s" => Ok(ForgeRunnerMode::K8s),
            _ => Err(format_err!("Invalid runner mode: {}", s)),
        }
    }
}

impl ForgeRunnerMode {
    pub fn try_from_env() -> Result<ForgeRunnerMode> {
        if let Ok(runner_mode) = std::env::var(FORGE_RUNNER_MODE) {
            Ok(ForgeRunnerMode::from_str(&runner_mode)?)
        } else if std::env::var(KUBERNETES_SERVICE_HOST).is_ok() {
            Ok(ForgeRunnerMode::K8s)
        } else {
            Ok(ForgeRunnerMode::Local)
        }
    }
}

pub struct Forge<'cfg, F> {
    options: &'cfg Options,
    tests: ForgeConfig,
    global_duration: Duration,
    factory: F,
}

impl<'cfg, F: Factory> Forge<'cfg, F> {
    pub fn new(
        options: &'cfg Options,
        tests: ForgeConfig,
        global_duration: Duration,
        factory: F,
    ) -> Self {
        Self {
            options,
            tests,
            global_duration,
            factory,
        }
    }

    pub fn list(&self) -> Result<()> {
        for test in self.filter_tests(&self.tests.all_tests()) {
            println!("{}: test", test.name());
        }

        if self.options.format == Format::Pretty {
            println!();
            println!(
                "{} tests",
                self.filter_tests(&self.tests.all_tests()).count()
            );
        }

        Ok(())
    }

    /// Get the initial version based on test configuration
    pub fn initial_version(&self) -> Version {
        let versions = self.factory.versions();
        match self.tests.initial_version {
            InitialVersion::Oldest => versions.min(),
            InitialVersion::Newest => versions.max(),
        }
        .expect("There has to be at least 1 version")
    }

    pub fn run(&self) -> Result<TestReport> {
        let test_count = self.filter_tests(&self.tests.all_tests()).count();
        let filtered_out = test_count.saturating_sub(self.tests.all_tests().len());
        let retain_debug_logs = self.options.retain_debug_logs || self.tests.retain_debug_logs;

        let mut report = TestReport::new();
        let mut summary = TestSummary::new(test_count, filtered_out);

        // Optionally write junit xml test report for external processing
        if let Some(junit_xml_path) = self.options.junit_xml_path.as_ref() {
            let junit_observer = JunitTestObserver::new(
                self.tests.get_suite_name().unwrap_or("local".to_string()),
                junit_xml_path.to_owned(),
            );
            summary.add_observer(boxed!(junit_observer));
        }
        summary.write_starting_msg()?;

        if test_count > 0 {
            println!(
                "Starting Swarm with supported versions: {:?}",
                self.factory
                    .versions()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
            );
            let initial_version = self.initial_version();
            // The genesis version should always match the initial node version
            let genesis_version = initial_version.clone();
            let runtime = Runtime::new().unwrap(); // TODO: new multithreaded?
            let mut rng = ::rand::rngs::StdRng::from_seed(OsRng.gen());
            let mut swarm = runtime.block_on(self.factory.launch_swarm(
                &mut rng,
                self.tests.initial_validator_count,
                self.tests.initial_fullnode_count,
                &initial_version,
                &genesis_version,
                self.tests.genesis_config.as_ref(),
                self.global_duration + Duration::from_secs(NAMESPACE_CLEANUP_DURATION_BUFFER_SECS),
                self.tests.genesis_helm_config_fn.clone(),
                self.tests.build_node_helm_config_fn(retain_debug_logs),
                self.tests.existing_db_tag.clone(),
            ))?;

            // Run AptosTests
            for test in self.filter_tests(&self.tests.aptos_tests) {
                let mut aptos_ctx = AptosContext::new(
                    CoreContext::from_rng(&mut rng),
                    swarm.chain_info().into_aptos_public_info(),
                    &mut report,
                );
                let result = process_test_result(runtime.block_on(test.run(&mut aptos_ctx)));
                report.report_text(result.to_string());
                summary.handle_result(test.details(), result)?;
            }

            // Run AdminTests
            for test in self.filter_tests(&self.tests.admin_tests) {
                let mut admin_ctx = AdminContext::new(
                    CoreContext::from_rng(&mut rng),
                    swarm.chain_info(),
                    &mut report,
                );
                let result = process_test_result(test.run(&mut admin_ctx));
                report.report_text(result.to_string());
                summary.handle_result(test.details(), result)?;
            }

            let logs_location = swarm.logs_location();
            let swarm = Arc::new(tokio::sync::RwLock::new(swarm));
            for test in self.filter_tests(&self.tests.network_tests) {
                let network_ctx = NetworkContext::new(
                    CoreContext::from_rng(&mut rng),
                    swarm.clone(),
                    &mut report,
                    self.global_duration,
                    self.tests.emit_job_request.clone(),
                    self.tests.success_criteria.clone(),
                );
                let handle = network_ctx.runtime.handle().clone();
                let _handle_context = handle.enter();
                let network_ctx = NetworkContextSynchronizer::new(network_ctx, handle.clone());
                let result = process_test_result(handle.block_on(test.run(network_ctx.clone())));
                // explicitly keep network context in scope so that its created tokio Runtime drops after all the stuff has run.
                let NetworkContextSynchronizer { ctx, handle } = network_ctx;
                drop(handle);
                let ctx = Arc::into_inner(ctx).unwrap().into_inner();
                drop(ctx);
                report.report_text(result.to_string());
                summary.handle_result(test.details(), result)?;
            }

            report.print_report();
            summary.finish()?;

            io::stdout().flush()?;
            io::stderr().flush()?;
            if !summary.success() {
                println!();
                println!("Swarm logs can be found here: {}", logs_location);
            }
        }

        summary.write_summary()?;

        if summary.success() {
            Ok(report)
        } else {
            bail!("Tests Failed")
        }
    }

    fn filter_tests<'a, T: Test + ?Sized>(
        &'a self,
        tests: &'a [Box<T>],
    ) -> impl Iterator<Item = &'a Box<T>> {
        tests
            .iter()
            // Filter by ignored
            .filter(
                move |test| match (self.options.include_ignored, self.options.ignored) {
                    (true, _) => true, // Don't filter anything
                    (false, true) => test.ignored(),
                    (false, false) => !test.ignored(),
                },
            )
            // Filter by test name
            .filter(move |test| {
                if let Some(filter) = &self.options.filter {
                    if self.options.filter_exact {
                        test.name() == &filter[..]
                    } else {
                        test.name().contains(&filter[..])
                    }
                } else {
                    true
                }
            })
    }
}

fn process_test_result(result: Result<()>) -> TestResult {
    match result {
        Ok(()) => TestResult::Successful,
        Err(e) => {
            let test_result = e
                .downcast()
                .map(|e: CriteriaCheckerErrors| e.into())
                .unwrap_or_else(|e| TestResult::InfraFailure(format!("Error: {:?}", e)));
            let is_triggerd_by_github_actions =
                std::env::var("FORGE_TRIGGERED_BY").unwrap_or_default() == "github-actions";
            if is_triggerd_by_github_actions {
                // ::error:: is github specific syntax to set an error on the job that is highlighted as described here https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions#setting-an-error-message
                println!("::error::{:?}", test_result);
            }
            test_result
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_forge_runner_mode_from_env() {
        // HACK we really should not be setting env variables in test

        // Store the env variables before we mutate them
        let original_forge_runner_mode = std::env::var(FORGE_RUNNER_MODE);
        let original_kubernetes_service_host = std::env::var(KUBERNETES_SERVICE_HOST);

        // Test the default locally
        std::env::remove_var(FORGE_RUNNER_MODE);
        std::env::remove_var(KUBERNETES_SERVICE_HOST);
        let default_local_runner_mode = ForgeRunnerMode::try_from_env();

        std::env::remove_var(FORGE_RUNNER_MODE);
        std::env::set_var(KUBERNETES_SERVICE_HOST, "1.1.1.1");
        let default_kubernetes_runner_mode = ForgeRunnerMode::try_from_env();

        std::env::set_var(FORGE_RUNNER_MODE, "local");
        std::env::set_var(KUBERNETES_SERVICE_HOST, "1.1.1.1");
        let local_runner_mode = ForgeRunnerMode::try_from_env();

        std::env::set_var(FORGE_RUNNER_MODE, "k8s");
        std::env::remove_var(KUBERNETES_SERVICE_HOST);
        let k8s_runner_mode = ForgeRunnerMode::try_from_env();

        std::env::set_var(FORGE_RUNNER_MODE, "durian");
        std::env::remove_var(KUBERNETES_SERVICE_HOST);
        let invalid_runner_mode = ForgeRunnerMode::try_from_env();

        // Reset the env variables after running
        match original_forge_runner_mode {
            Ok(mode) => std::env::set_var(FORGE_RUNNER_MODE, mode),
            Err(_) => std::env::remove_var(FORGE_RUNNER_MODE),
        }
        match original_kubernetes_service_host {
            Ok(service_host) => std::env::set_var(KUBERNETES_SERVICE_HOST, service_host),
            Err(_) => std::env::remove_var(KUBERNETES_SERVICE_HOST),
        }

        assert_eq!(default_local_runner_mode.unwrap(), ForgeRunnerMode::Local);
        assert_eq!(
            default_kubernetes_runner_mode.unwrap(),
            ForgeRunnerMode::K8s
        );
        assert_eq!(local_runner_mode.unwrap(), ForgeRunnerMode::Local);
        assert_eq!(k8s_runner_mode.unwrap(), ForgeRunnerMode::K8s);
        assert_eq!(
            invalid_runner_mode.unwrap_err().to_string(),
            "Invalid runner mode: durian"
        );
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Options::command().debug_assert()
}
