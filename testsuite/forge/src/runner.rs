// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// TODO going to remove random seed once cluster deployment supports re-run genesis
use crate::{
    success_criteria::{MetricsThreshold, SuccessCriteria, SystemMetricsThreshold},
    *,
};
use anyhow::{bail, format_err, Error, Result};
use aptos_config::config::{NodeConfig, OverrideNodeConfig};
use aptos_framework::ReleaseBundle;
use clap::{Parser, ValueEnum};
use rand::{rngs::OsRng, Rng, SeedableRng};
use std::{
    fmt::{Display, Formatter},
    io::{self, Write},
    num::NonZeroUsize,
    process,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
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

pub struct ForgeConfig {
    aptos_tests: Vec<Box<dyn AptosTest>>,
    admin_tests: Vec<Box<dyn AdminTest>>,
    network_tests: Vec<Box<dyn NetworkTest>>,

    /// The initial number of validators to spawn when the test harness creates a swarm
    initial_validator_count: NonZeroUsize,

    /// The initial number of fullnodes to spawn when the test harness creates a swarm
    initial_fullnode_count: usize,

    /// The initial version to use when the test harness creates a swarm
    initial_version: InitialVersion,

    /// The initial genesis modules to use when starting a network
    genesis_config: Option<GenesisConfig>,

    /// Optional genesis helm values init function
    genesis_helm_config_fn: Option<GenesisConfigFn>,

    /// Optional validator node config override function
    validator_override_node_config_fn: Option<OverrideNodeConfigFn>,

    /// Optional fullnode node config override function
    fullnode_override_node_config_fn: Option<OverrideNodeConfigFn>,

    multi_region_config: bool,

    /// Transaction workload to run on the swarm
    emit_job_request: EmitJobRequest,

    /// Success criteria
    success_criteria: SuccessCriteria,

    /// The label of existing DBs to use, if None, will create new db.
    existing_db_tag: Option<String>,

    validator_resource_override: NodeResourceOverride,

    fullnode_resource_override: NodeResourceOverride,

    /// Retain debug logs and above for all nodes instead of just the first 5 nodes
    retain_debug_logs: bool,
}

impl ForgeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_aptos_test<T: AptosTest + 'static>(mut self, aptos_test: T) -> Self {
        self.aptos_tests.push(Box::new(aptos_test));
        self
    }

    pub fn with_aptos_tests(mut self, aptos_tests: Vec<Box<dyn AptosTest>>) -> Self {
        self.aptos_tests = aptos_tests;
        self
    }

    pub fn add_admin_test<T: AdminTest + 'static>(mut self, admin_test: T) -> Self {
        self.admin_tests.push(Box::new(admin_test));
        self
    }

    pub fn with_admin_tests(mut self, admin_tests: Vec<Box<dyn AdminTest>>) -> Self {
        self.admin_tests = admin_tests;
        self
    }

    pub fn add_network_test<T: NetworkTest + 'static>(mut self, network_test: T) -> Self {
        self.network_tests.push(Box::new(network_test));
        self
    }

    pub fn with_network_tests(mut self, network_tests: Vec<Box<dyn NetworkTest>>) -> Self {
        self.network_tests = network_tests;
        self
    }

    pub fn with_initial_validator_count(mut self, initial_validator_count: NonZeroUsize) -> Self {
        self.initial_validator_count = initial_validator_count;
        self
    }

    pub fn with_initial_fullnode_count(mut self, initial_fullnode_count: usize) -> Self {
        self.initial_fullnode_count = initial_fullnode_count;
        self
    }

    pub fn with_genesis_helm_config_fn(mut self, genesis_helm_config_fn: GenesisConfigFn) -> Self {
        self.genesis_helm_config_fn = Some(genesis_helm_config_fn);
        self
    }

    pub fn with_validator_override_node_config_fn(mut self, f: OverrideNodeConfigFn) -> Self {
        self.validator_override_node_config_fn = Some(f);
        self
    }

    pub fn with_fullnode_override_node_config_fn(mut self, f: OverrideNodeConfigFn) -> Self {
        self.fullnode_override_node_config_fn = Some(f);
        self
    }

    pub fn with_multi_region_config(mut self) -> Self {
        self.multi_region_config = true;
        self
    }

    pub fn with_validator_resource_override(
        mut self,
        resource_override: NodeResourceOverride,
    ) -> Self {
        self.validator_resource_override = resource_override;
        self
    }

    pub fn with_fullnode_resource_override(
        mut self,
        resource_override: NodeResourceOverride,
    ) -> Self {
        self.fullnode_resource_override = resource_override;
        self
    }

    fn override_node_config_from_fn(config_fn: OverrideNodeConfigFn) -> OverrideNodeConfig {
        let mut override_config = NodeConfig::default();
        let mut base_config = NodeConfig::default();
        config_fn(&mut override_config, &mut base_config);
        OverrideNodeConfig::new(override_config, base_config)
    }

    pub fn build_node_helm_config_fn(&self, retain_debug_logs: bool) -> Option<NodeConfigFn> {
        let validator_override_node_config = self
            .validator_override_node_config_fn
            .clone()
            .map(|config_fn| Self::override_node_config_from_fn(config_fn));
        let fullnode_override_node_config = self
            .fullnode_override_node_config_fn
            .clone()
            .map(|config_fn| Self::override_node_config_from_fn(config_fn));
        let multi_region_config = self.multi_region_config;
        let existing_db_tag = self.existing_db_tag.clone();
        let validator_resource_override = self.validator_resource_override;
        let fullnode_resource_override = self.fullnode_resource_override;

        // Override specific helm values. See reference: terraform/helm/aptos-node/values.yaml
        Some(Arc::new(move |helm_values: &mut serde_yaml::Value| {
            if let Some(override_config) = &validator_override_node_config {
                helm_values["validator"]["config"] = override_config.get_yaml().unwrap();
            }
            if let Some(override_config) = &fullnode_override_node_config {
                helm_values["fullnode"]["config"] = override_config.get_yaml().unwrap();
            }
            if multi_region_config {
                helm_values["multicluster"]["enabled"] = true.into();
                // Create headless services for validators and fullnodes.
                // Note: chaos-mesh will not work with clusterIP services.
                helm_values["service"]["validator"]["internal"]["type"] = "ClusterIP".into();
                helm_values["service"]["validator"]["internal"]["headless"] = true.into();
                helm_values["service"]["fullnode"]["internal"]["type"] = "ClusterIP".into();
                helm_values["service"]["fullnode"]["internal"]["headless"] = true.into();
            }
            if let Some(existing_db_tag) = &existing_db_tag {
                helm_values["validator"]["storage"]["labels"]["tag"] =
                    existing_db_tag.clone().into();
                helm_values["fullnode"]["storage"]["labels"]["tag"] =
                    existing_db_tag.clone().into();
            }

            // validator resource overrides
            if let Some(cpu_cores) = validator_resource_override.cpu_cores {
                helm_values["validator"]["resources"]["requests"]["cpu"] = cpu_cores.into();
                helm_values["validator"]["resources"]["limits"]["cpu"] = cpu_cores.into();
            }
            if let Some(memory_gib) = validator_resource_override.memory_gib {
                helm_values["validator"]["resources"]["requests"]["memory"] =
                    format!("{}Gi", memory_gib).into();
                helm_values["validator"]["resources"]["limits"]["memory"] =
                    format!("{}Gi", memory_gib).into();
            }
            if let Some(storage_gib) = validator_resource_override.storage_gib {
                helm_values["validator"]["storage"]["size"] = format!("{}Gi", storage_gib).into();
            }
            // fullnode resource overrides
            if let Some(cpu_cores) = fullnode_resource_override.cpu_cores {
                helm_values["fullnode"]["resources"]["requests"]["cpu"] = cpu_cores.into();
                helm_values["fullnode"]["resources"]["limits"]["cpu"] = cpu_cores.into();
            }
            if let Some(memory_gib) = fullnode_resource_override.memory_gib {
                helm_values["fullnode"]["resources"]["requests"]["memory"] =
                    format!("{}Gi", memory_gib).into();
                helm_values["fullnode"]["resources"]["limits"]["memory"] =
                    format!("{}Gi", memory_gib).into();
            }
            if let Some(storage_gib) = fullnode_resource_override.storage_gib {
                helm_values["fullnode"]["storage"]["size"] = format!("{}Gi", storage_gib).into();
            }

            if retain_debug_logs {
                helm_values["validator"]["podAnnotations"]["aptos.dev/min-log-level-to-retain"] =
                    serde_yaml::Value::String("debug".to_owned());
                helm_values["fullnode"]["podAnnotations"]["aptos.dev/min-log-level-to-retain"] =
                    serde_yaml::Value::String("debug".to_owned());
                helm_values["validator"]["rust_log"] = "debug,hyper=off".into();
                helm_values["fullnode"]["rust_log"] = "debug,hyper=off".into();
            }
            helm_values["validator"]["config"]["storage"]["rocksdb_configs"]
                ["enable_storage_sharding"] = true.into();
            helm_values["fullnode"]["config"]["storage"]["rocksdb_configs"]
                ["enable_storage_sharding"] = true.into();
            helm_values["validator"]["config"]["indexer_db_config"]["enable_event"] = true.into();
            helm_values["fullnode"]["config"]["indexer_db_config"]["enable_event"] = true.into();
        }))
    }

    pub fn with_initial_version(mut self, initial_version: InitialVersion) -> Self {
        self.initial_version = initial_version;
        self
    }

    pub fn with_genesis_module_bundle(mut self, bundle: ReleaseBundle) -> Self {
        self.genesis_config = Some(GenesisConfig::Bundle(bundle));
        self
    }

    pub fn with_genesis_modules_path(mut self, genesis_modules: String) -> Self {
        self.genesis_config = Some(GenesisConfig::Path(genesis_modules));
        self
    }

    pub fn with_emit_job(mut self, emit_job_request: EmitJobRequest) -> Self {
        self.emit_job_request = emit_job_request;
        self
    }

    pub fn get_emit_job(&self) -> &EmitJobRequest {
        &self.emit_job_request
    }

    pub fn with_success_criteria(mut self, success_criteria: SuccessCriteria) -> Self {
        self.success_criteria = success_criteria;
        self
    }

    pub fn get_success_criteria_mut(&mut self) -> &mut SuccessCriteria {
        &mut self.success_criteria
    }

    pub fn with_existing_db(mut self, tag: String) -> Self {
        self.existing_db_tag = Some(tag);
        self
    }

    pub fn number_of_tests(&self) -> usize {
        self.admin_tests.len() + self.network_tests.len() + self.aptos_tests.len()
    }

    pub fn all_tests(&self) -> Vec<Box<AnyTestRef<'_>>> {
        self.admin_tests
            .iter()
            .map(|t| Box::new(AnyTestRef::Admin(t.as_ref())))
            .chain(
                self.network_tests
                    .iter()
                    .map(|t| Box::new(AnyTestRef::Network(t.as_ref()))),
            )
            .chain(
                self.aptos_tests
                    .iter()
                    .map(|t| Box::new(AnyTestRef::Aptos(t.as_ref()))),
            )
            .collect()
    }
}

// Workaround way to implement all_tests, for:
// error[E0658]: cannot cast `dyn interface::admin::AdminTest` to `dyn interface::test::Test`, trait upcasting coercion is experimental
pub enum AnyTestRef<'a> {
    Aptos(&'a dyn AptosTest),
    Admin(&'a dyn AdminTest),
    Network(&'a dyn NetworkTest),
}

impl<'a> Test for AnyTestRef<'a> {
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

impl Default for ForgeConfig {
    fn default() -> Self {
        let forge_run_mode = ForgeRunnerMode::try_from_env().unwrap_or(ForgeRunnerMode::K8s);
        let success_criteria = if forge_run_mode == ForgeRunnerMode::Local {
            SuccessCriteria::new(600).add_no_restarts()
        } else {
            SuccessCriteria::new(3500)
                .add_no_restarts()
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12.0, 30),
                    // Check that we don't use more than 10 GB of memory for 30% of the time.
                    MetricsThreshold::new_gb(10.0, 30),
                ))
        };
        Self {
            aptos_tests: vec![],
            admin_tests: vec![],
            network_tests: vec![],
            initial_validator_count: NonZeroUsize::new(1).unwrap(),
            initial_fullnode_count: 0,
            initial_version: InitialVersion::Oldest,
            genesis_config: None,
            genesis_helm_config_fn: None,
            validator_override_node_config_fn: None,
            fullnode_override_node_config_fn: None,
            multi_region_config: false,
            emit_job_request: EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
                mempool_backlog: 40000,
            }),
            success_criteria,
            existing_db_tag: None,
            validator_resource_override: NodeResourceOverride::default(),
            fullnode_resource_override: NodeResourceOverride::default(),
            retain_debug_logs: false,
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
                let result = run_test(|| runtime.block_on(test.run(&mut aptos_ctx)));
                report.report_text(result.to_string());
                summary.handle_result(test.name().to_owned(), result)?;
            }

            // Run AdminTests
            for test in self.filter_tests(&self.tests.admin_tests) {
                let mut admin_ctx = AdminContext::new(
                    CoreContext::from_rng(&mut rng),
                    swarm.chain_info(),
                    &mut report,
                );
                let result = run_test(|| test.run(&mut admin_ctx));
                report.report_text(result.to_string());
                summary.handle_result(test.name().to_owned(), result)?;
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
                let result = run_test(|| handle.block_on(test.run(network_ctx.clone())));
                // explicitly keep network context in scope so that its created tokio Runtime drops after all the stuff has run.
                let NetworkContextSynchronizer { ctx, handle } = network_ctx;
                drop(handle);
                let ctx = Arc::into_inner(ctx).unwrap().into_inner();
                drop(ctx);
                report.report_text(result.to_string());
                summary.handle_result(test.name().to_owned(), result)?;
            }

            report.print_report();

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

enum TestResult {
    Ok,
    FailedWithMsg(String),
}

impl Display for TestResult {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            TestResult::Ok => write!(f, "Test Ok"),
            TestResult::FailedWithMsg(msg) => write!(f, "Test Failed: {}", msg),
        }
    }
}

fn run_test<F: FnOnce() -> Result<()>>(f: F) -> TestResult {
    match f() {
        Ok(()) => TestResult::Ok,
        Err(e) => {
            let is_triggerd_by_github_actions =
                std::env::var("FORGE_TRIGGERED_BY").unwrap_or_default() == "github-actions";
            if is_triggerd_by_github_actions {
                // ::error:: is github specific syntax to set an error on the job that is highlighted as described here https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions#setting-an-error-message
                println!("::error::{:?}", e);
            }
            TestResult::FailedWithMsg(format!("{:?}", e))
        },
    }
}

struct TestSummary {
    stdout: StandardStream,
    total: usize,
    filtered_out: usize,
    passed: usize,
    failed: Vec<String>,
}

impl TestSummary {
    fn new(total: usize, filtered_out: usize) -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Auto),
            total,
            filtered_out,
            passed: 0,
            failed: Vec::new(),
        }
    }

    fn handle_result(&mut self, name: String, result: TestResult) -> io::Result<()> {
        write!(self.stdout, "test {} ... ", name)?;
        match result {
            TestResult::Ok => {
                self.passed += 1;
                self.write_ok()?;
            },
            TestResult::FailedWithMsg(msg) => {
                self.failed.push(name);
                self.write_failed()?;
                writeln!(self.stdout)?;

                write!(self.stdout, "Error: {}", msg)?;
            },
        }
        writeln!(self.stdout)?;
        Ok(())
    }

    fn write_ok(&mut self) -> io::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(self.stdout, "ok")?;
        self.stdout.reset()?;
        Ok(())
    }

    fn write_failed(&mut self) -> io::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        write!(self.stdout, "FAILED")?;
        self.stdout.reset()?;
        Ok(())
    }

    fn write_starting_msg(&mut self) -> io::Result<()> {
        writeln!(self.stdout)?;
        writeln!(
            self.stdout,
            "running {} tests",
            self.total - self.filtered_out
        )?;
        Ok(())
    }

    fn write_summary(&mut self) -> io::Result<()> {
        // Print out the failing tests
        if !self.failed.is_empty() {
            writeln!(self.stdout)?;
            writeln!(self.stdout, "failures:")?;
            for name in &self.failed {
                writeln!(self.stdout, "    {}", name)?;
            }
        }

        writeln!(self.stdout)?;
        write!(self.stdout, "test result: ")?;
        if self.failed.is_empty() {
            self.write_ok()?;
        } else {
            self.write_failed()?;
        }
        writeln!(
            self.stdout,
            ". {} passed; {} failed; {} filtered out",
            self.passed,
            self.failed.len(),
            self.filtered_out
        )?;
        writeln!(self.stdout)?;
        Ok(())
    }

    fn success(&self) -> bool {
        self.failed.is_empty()
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
