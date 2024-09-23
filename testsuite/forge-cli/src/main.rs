// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::field_reassign_with_default)]

use anyhow::{format_err, Context, Result};
use aptos_config::config::{
    BootstrappingMode, ConsensusConfig, ContinuousSyncingMode, MempoolConfig, NetbenchConfig,
    NodeConfig, StateSyncConfig,
};
use aptos_forge::{
    args::TransactionTypeArg,
    emitter::NumAccountsMode,
    prometheus_metrics::LatencyBreakdownSlice,
    success_criteria::{
        LatencyBreakdownThreshold, LatencyType, MetricsThreshold, StateProgressThreshold,
        SuccessCriteria, SystemMetricsThreshold,
    },
    ForgeConfig, Options, *,
};
use aptos_logger::{info, Level};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::aptos_stdlib,
    types::on_chain_config::{
        BlockGasLimitType, OnChainConsensusConfig, OnChainExecutionConfig, TransactionShufflerType,
    },
};
use aptos_testcases::{
    compatibility_test::SimpleValidatorUpgrade,
    consensus_reliability_tests::ChangingWorkingQuorumTest,
    forge_setup_test::ForgeSetupTest,
    framework_upgrade::FrameworkUpgrade,
    fullnode_reboot_stress_test::FullNodeRebootStressTest,
    generate_traffic,
    load_vs_perf_benchmark::{
        BackgroundTraffic, LoadVsPerfBenchmark, TransactionWorkload, Workloads,
    },
    modifiers::{CpuChaosTest, ExecutionDelayConfig, ExecutionDelayTest},
    multi_region_network_test::{
        MultiRegionNetworkEmulationConfig, MultiRegionNetworkEmulationTest,
    },
    network_bandwidth_test::NetworkBandwidthTest,
    network_loss_test::NetworkLossTest,
    network_partition_test::NetworkPartitionTest,
    performance_test::PerformanceBenchmark,
    public_fullnode_performance::PFNPerformance,
    quorum_store_onchain_enable_test::QuorumStoreOnChainEnableTest,
    reconfiguration_test::ReconfigurationTest,
    state_sync_performance::{
        StateSyncFullnodeFastSyncPerformance, StateSyncFullnodePerformance,
        StateSyncValidatorPerformance,
    },
    three_region_simulation_test::ThreeRegionSameCloudSimulationTest,
    twin_validator_test::TwinValidatorTest,
    two_traffics_test::TwoTrafficsTest,
    validator_join_leave_test::ValidatorJoinLeaveTest,
    validator_reboot_stress_test::ValidatorRebootStressTest,
    CompositeNetworkTest,
};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use futures::stream::{FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use std::{
    env,
    num::NonZeroUsize,
    ops::DerefMut,
    path::{Path, PathBuf},
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use suites::dag::get_dag_test;
use tokio::{runtime::Runtime, select};
use url::Url;

mod suites;

// Useful constants
const KILOBYTE: usize = 1000;
const MEGABYTE: usize = KILOBYTE * 1000;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, default_value_t = 300)]
    duration_secs: usize,
    #[clap(flatten)]
    options: Options,
    #[clap(long)]
    num_validators: Option<usize>,
    #[clap(long)]
    num_validator_fullnodes: Option<usize>,
    #[clap(
        long,
        help = "Specify a test suite to run",
        default_value = "land_blocking"
    )]
    suite: String,
    #[clap(long, num_args = 0..)]
    changelog: Option<Vec<String>>,

    // subcommand groups
    #[clap(subcommand)]
    cli_cmd: CliCommand,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Subcommands to run forge tests
    #[clap(subcommand)]
    Test(TestCommand),
    /// Subcommands to set up or manage running forge networks
    #[clap(subcommand)]
    Operator(OperatorCommand),
}

#[derive(Subcommand, Debug)]
enum TestCommand {
    /// Run tests using the local swarm backend
    LocalSwarm(LocalSwarm),
    /// Run tests in cluster using the remote kubernetes backend
    K8sSwarm(K8sSwarm),
}

#[derive(Subcommand, Debug)]
enum OperatorCommand {
    /// Set the image tag for a node in the cluster
    SetNodeImageTag(SetNodeImageTag),
    /// Clean up an existing cluster
    CleanUp(CleanUp),
    /// Resize an existing cluster
    Resize(Resize),
}

#[derive(Parser, Debug)]
struct LocalSwarm {
    #[clap(long, help = "directory to build local swarm under")]
    swarmdir: Option<String>,
}

#[derive(Parser, Debug)]
struct K8sSwarm {
    #[clap(long, help = "The kubernetes namespace to use for test")]
    namespace: Option<String>,
    #[clap(
        long,
        help = "The image tag currently is used for validators",
        default_value = "devnet"
    )]
    image_tag: String,
    #[clap(
        long,
        help = "For supported tests, the image tag for validators to upgrade to",
        default_value = "devnet"
    )]
    upgrade_image_tag: String,
    #[clap(
        long,
        help = "Path to flattened directory containing compiled Move modules"
    )]
    move_modules_dir: Option<String>,
    #[clap(
        long,
        help = "If set, uses kubectl port-forward instead of assuming k8s DNS access"
    )]
    port_forward: bool,
    #[clap(
        long,
        help = "If set, reuse the forge testnet active in the specified namespace"
    )]
    reuse: bool,
    #[clap(
        long,
        help = "If set, keeps the forge testnet active in the specified namespace"
    )]
    keep: bool,
    #[clap(long, help = "If set, enables HAProxy for each of the validators")]
    enable_haproxy: bool,
    #[clap(
        long,
        help = "Retain debug logs and above for all nodes instead of just the first 5 nodes"
    )]
    retain_debug_logs: bool,
}

#[derive(Parser, Debug)]
struct SetNodeImageTag {
    #[clap(long, help = "The name of the node StatefulSet to update")]
    stateful_set_name: String,
    #[clap(long, help = "The name of the container to update")]
    container_name: String,
    #[clap(long, help = "The docker image tag to use for the node")]
    image_tag: String,
    #[clap(long, help = "The kubernetes namespace to clean up")]
    namespace: String,
}

#[derive(Parser, Debug)]
struct CleanUp {
    #[clap(
        long,
        help = "The kubernetes namespace to clean up. If unset, attemps to cleanup all by using forge-management configmaps"
    )]
    namespace: Option<String>,
}

#[derive(Parser, Debug)]
struct Resize {
    #[clap(long, help = "The kubernetes namespace to resize")]
    namespace: String,
    #[clap(long, default_value_t = 30)]
    num_validators: usize,
    #[clap(long, default_value_t = 1)]
    num_fullnodes: usize,
    #[clap(
        long,
        help = "Override the image tag used for validators",
        default_value = "devnet"
    )]
    validator_image_tag: String,
    #[clap(
        long,
        help = "Override the image tag used for testnet-specific components",
        default_value = "devnet"
    )]
    testnet_image_tag: String,
    #[clap(
        long,
        help = "Path to flattened directory containing compiled Move modules"
    )]
    move_modules_dir: Option<String>,
    #[clap(
        long,
        help = "If set, dont use kubectl port forward to access the cluster"
    )]
    connect_directly: bool,
    #[clap(long, help = "If set, enables HAProxy for each of the validators")]
    enable_haproxy: bool,
}

// common metrics thresholds:
static SYSTEM_12_CORES_5GB_THRESHOLD: Lazy<SystemMetricsThreshold> = Lazy::new(|| {
    SystemMetricsThreshold::new(
        // Check that we don't use more than 12 CPU cores for 30% of the time.
        MetricsThreshold::new(12.0, 30),
        // Check that we don't use more than 5 GB of memory for 30% of the time.
        MetricsThreshold::new_gb(5.0, 30),
    )
});
static SYSTEM_12_CORES_10GB_THRESHOLD: Lazy<SystemMetricsThreshold> = Lazy::new(|| {
    SystemMetricsThreshold::new(
        // Check that we don't use more than 12 CPU cores for 30% of the time.
        MetricsThreshold::new(12.0, 30),
        // Check that we don't use more than 10 GB of memory for 30% of the time.
        MetricsThreshold::new_gb(10.0, 30),
    )
});

static RELIABLE_PROGRESS_THRESHOLD: Lazy<StateProgressThreshold> =
    Lazy::new(|| StateProgressThreshold {
        max_non_epoch_no_progress_secs: 10.0,
        max_epoch_no_progress_secs: 10.0,
        max_non_epoch_round_gap: 4,
        max_epoch_round_gap: 4,
    });

static PROGRESS_THRESHOLD_20_6: Lazy<StateProgressThreshold> =
    Lazy::new(|| StateProgressThreshold {
        max_non_epoch_no_progress_secs: 20.0,
        max_epoch_no_progress_secs: 20.0,
        max_non_epoch_round_gap: 6,
        max_epoch_round_gap: 6,
    });

static RELIABLE_REAL_ENV_PROGRESS_THRESHOLD: Lazy<StateProgressThreshold> =
    Lazy::new(|| StateProgressThreshold {
        max_non_epoch_no_progress_secs: 30.0,
        max_epoch_no_progress_secs: 30.0,
        max_non_epoch_round_gap: 10,
        max_epoch_round_gap: 10,
    });

/// Make an easy to remember random namespace for your testnet
fn random_namespace<R: Rng>(dictionary: Vec<String>, rng: &mut R) -> Result<String> {
    // Pick four random words
    let random_words = dictionary
        .choose_multiple(rng, 4)
        .cloned()
        .collect::<Vec<String>>();
    Ok(format!("forge-{}", random_words.join("-")))
}

fn main() -> Result<()> {
    let mut logger = aptos_logger::Logger::new();
    logger.channel_size(1000).is_async(false).level(Level::Info);
    logger.build();

    let args = Args::parse();
    let duration = Duration::from_secs(args.duration_secs as u64);
    let suite_name: &str = args.suite.as_ref();

    let runtime = Runtime::new()?;
    match args.cli_cmd {
        // cmd input for test
        CliCommand::Test(ref test_cmd) => {
            // Identify the test suite to run
            let mut test_suite = get_test_suite(suite_name, duration, test_cmd)?;

            // Identify the number of validators and fullnodes to run
            // (if overriding what test has specified)
            if let Some(num_validators) = args.num_validators {
                let num_validators_non_zero = NonZeroUsize::new(num_validators)
                    .context("--num-validators must be positive!")?;
                test_suite = test_suite.with_initial_validator_count(num_validators_non_zero);

                // Verify the number of fullnodes is less than the validators
                if let Some(num_validator_fullnodes) = args.num_validator_fullnodes {
                    if num_validator_fullnodes > num_validators {
                        return Err(format_err!(
                            "Cannot have more fullnodes than validators! Fullnodes: {:?}, validators: {:?}.",
                            num_validator_fullnodes, num_validators
                        ));
                    }
                }
            }
            if let Some(num_validator_fullnodes) = args.num_validator_fullnodes {
                test_suite = test_suite.with_initial_fullnode_count(num_validator_fullnodes)
            }

            // Run the test suite
            match test_cmd {
                TestCommand::LocalSwarm(local_cfg) => {
                    // Loosen all criteria for local runs
                    test_suite.get_success_criteria_mut().min_avg_tps = 400.0;
                    let previous_emit_job = test_suite.get_emit_job().clone();
                    let test_suite =
                        test_suite.with_emit_job(previous_emit_job.mode(EmitJobMode::MaxLoad {
                            mempool_backlog: 5000,
                        }));
                    let swarm_dir = local_cfg.swarmdir.clone();
                    run_forge(
                        duration,
                        test_suite,
                        LocalFactory::from_workspace(swarm_dir)?,
                        &args.options,
                        args.changelog.clone(),
                    )
                },
                TestCommand::K8sSwarm(k8s) => {
                    if let Some(move_modules_dir) = &k8s.move_modules_dir {
                        test_suite = test_suite.with_genesis_modules_path(move_modules_dir.clone());
                    }
                    let namespace = if k8s.namespace.is_none() {
                        let mut rng: ThreadRng = rand::thread_rng();
                        // Lets pick some four letter words ;)
                        let words = random_word::all_len(4)
                            .ok_or_else(|| {
                                format_err!(
                                    "Failed to get namespace, rerun with --namespace <namespace>"
                                )
                            })?
                            .to_vec()
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>();
                        random_namespace(words, &mut rng)?
                    } else {
                        k8s.namespace.clone().unwrap()
                    };
                    let forge_runner_mode =
                        ForgeRunnerMode::try_from_env().unwrap_or(ForgeRunnerMode::K8s);
                    run_forge(
                        duration,
                        test_suite,
                        K8sFactory::new(
                            namespace,
                            k8s.image_tag.clone(),
                            k8s.upgrade_image_tag.clone(),
                            // We want to port forward if we're running locally because local means we're not in cluster
                            k8s.port_forward || forge_runner_mode == ForgeRunnerMode::Local,
                            k8s.reuse,
                            k8s.keep,
                            k8s.enable_haproxy,
                        )
                        .unwrap(),
                        &args.options,
                        args.changelog,
                    )?;
                    Ok(())
                },
            }
        },
        // cmd input for cluster operations
        CliCommand::Operator(op_cmd) => match op_cmd {
            OperatorCommand::SetNodeImageTag(set_stateful_set_image_tag_config) => {
                runtime.block_on(set_stateful_set_image_tag(
                    set_stateful_set_image_tag_config.stateful_set_name,
                    set_stateful_set_image_tag_config.container_name,
                    set_stateful_set_image_tag_config.image_tag,
                    set_stateful_set_image_tag_config.namespace,
                ))?;
                Ok(())
            },
            OperatorCommand::CleanUp(cleanup) => {
                if let Some(namespace) = cleanup.namespace {
                    runtime.block_on(uninstall_testnet_resources(namespace))?;
                } else {
                    runtime.block_on(cleanup_cluster_with_management())?;
                }
                Ok(())
            },
            OperatorCommand::Resize(resize) => {
                runtime.block_on(install_testnet_resources(
                    resize.namespace,
                    resize.num_validators,
                    resize.num_fullnodes,
                    resize.validator_image_tag,
                    resize.testnet_image_tag,
                    resize.move_modules_dir,
                    !resize.connect_directly,
                    resize.enable_haproxy,
                    None,
                    None,
                ))?;
                Ok(())
            },
        },
    }
}

pub fn run_forge<F: Factory>(
    global_duration: Duration,
    tests: ForgeConfig,
    factory: F,
    options: &Options,
    logs: Option<Vec<String>>,
) -> Result<()> {
    let forge = Forge::new(options, tests, global_duration, factory);

    if options.list {
        forge.list()?;

        return Ok(());
    }

    match forge.run() {
        Ok(report) => {
            if let Some(mut changelog) = logs {
                if changelog.len() != 2 {
                    println!("Use: changelog <from> <to>");
                    process::exit(1);
                }
                let to_commit = changelog.remove(1);
                let from_commit = Some(changelog.remove(0));
                send_changelog_message(&report.to_string(), &from_commit, &to_commit);
            }
            Ok(())
        },
        Err(e) => {
            eprintln!("Failed to run tests:\n{}", e);
            Err(e)
        },
    }
}

pub fn send_changelog_message(perf_msg: &str, from_commit: &Option<String>, to_commit: &str) {
    println!(
        "Generating changelog from {:?} to {}",
        from_commit, to_commit
    );
    let changelog = get_changelog(from_commit.as_ref(), to_commit);
    let msg = format!("{}\n\n{}", changelog, perf_msg);
    let slack_url: Option<Url> = env::var("SLACK_URL")
        .map(|u| u.parse().expect("Failed to parse SLACK_URL"))
        .ok();
    if let Some(ref slack_url) = slack_url {
        let slack_client = SlackClient::new();
        if let Err(e) = slack_client.send_message(slack_url, &msg) {
            println!("Failed to send slack message: {}", e);
        }
    }
}

fn get_changelog(prev_commit: Option<&String>, upstream_commit: &str) -> String {
    let github_client = GitHub::new();
    let commits = github_client.get_commits("aptos-labs/aptos-core", upstream_commit);
    match commits {
        Err(e) => {
            println!("Failed to get github commits: {:?}", e);
            format!("*Revision upstream_{}*", upstream_commit)
        },
        Ok(commits) => {
            let mut msg = format!("*Revision {}*", upstream_commit);
            for commit in commits {
                if let Some(prev_commit) = prev_commit {
                    if commit.sha.starts_with(prev_commit) {
                        break;
                    }
                }
                let commit_lines: Vec<_> = commit.commit.message.split('\n').collect();
                let commit_head = commit_lines[0];
                let commit_head = commit_head.replace("[breaking]", "*[breaking]*");
                let short_sha = &commit.sha[..6];
                let email_parts: Vec<_> = commit.commit.author.email.split('@').collect();
                let author = email_parts[0];
                let line = format!("\n>\u{2022} {} _{}_ {}", short_sha, author, commit_head);
                msg.push_str(&line);
            }
            msg
        },
    }
}

// TODO: can we clean this function up?
/// Returns the test suite for the given test name
fn get_test_suite(
    test_name: &str,
    duration: Duration,
    test_cmd: &TestCommand,
) -> Result<ForgeConfig> {
    // Check the test name against the multi-test suites
    match test_name {
        "local_test_suite" => return Ok(local_test_suite()),
        "pre_release" => return Ok(pre_release_suite()),
        "run_forever" => return Ok(run_forever()),
        // TODO(rustielin): verify each test suite
        "k8s_suite" => return Ok(k8s_test_suite()),
        "chaos" => return Ok(chaos_test_suite(duration)),
        _ => {}, // No multi-test suite matches!
    };

    // Otherwise, check the test name against the grouped test suites
    if let Some(test_suite) = get_land_blocking_test(test_name, duration, test_cmd) {
        return Ok(test_suite);
    } else if let Some(test_suite) = get_multi_region_test(test_name) {
        return Ok(test_suite);
    } else if let Some(test_suite) = get_netbench_test(test_name) {
        return Ok(test_suite);
    } else if let Some(test_suite) = get_pfn_test(test_name, duration) {
        return Ok(test_suite);
    } else if let Some(test_suite) = get_realistic_env_test(test_name, duration, test_cmd) {
        return Ok(test_suite);
    } else if let Some(test_suite) = get_state_sync_test(test_name) {
        return Ok(test_suite);
    } else if let Some(test_suite) = get_dag_test(test_name, duration, test_cmd) {
        return Ok(test_suite);
    }

    // Otherwise, check the test name against the ungrouped test suites
    let ungrouped_test_suite = match test_name {
        "epoch_changer_performance" => epoch_changer_performance(),
        "validators_join_and_leave" => validators_join_and_leave(),
        "config" => ForgeConfig::default().add_network_test(ReconfigurationTest),
        "network_partition" => network_partition(),
        "network_bandwidth" => network_bandwidth(),
        "setup_test" => setup_test(),
        "single_vfn_perf" => single_vfn_perf(),
        "validator_reboot_stress_test" => validator_reboot_stress_test(),
        "fullnode_reboot_stress_test" => fullnode_reboot_stress_test(),
        "workload_mix" => workload_mix_test(),
        "account_creation" | "nft_mint" | "publishing" | "module_loading"
        | "write_new_resource" => individual_workload_tests(test_name.into()),
        // not scheduled on continuous
        "load_vs_perf_benchmark" => load_vs_perf_benchmark(),
        "workload_vs_perf_benchmark" => workload_vs_perf_benchmark(),
        // maximizing number of rounds and epochs within a given time, to stress test consensus
        // so using small constant traffic, small blocks and fast rounds, and short epochs.
        // reusing changing_working_quorum_test just for invariants/asserts, but with max_down_nodes = 0.
        "consensus_stress_test" => consensus_stress_test(),
        "changing_working_quorum_test" => changing_working_quorum_test(),
        "changing_working_quorum_test_high_load" => changing_working_quorum_test_high_load(),
        // not scheduled on continuous
        "large_test_only_few_nodes_down" => large_test_only_few_nodes_down(),
        "different_node_speed_and_reliability_test" => different_node_speed_and_reliability_test(),
        "state_sync_slow_processing_catching_up" => state_sync_slow_processing_catching_up(),
        "state_sync_failures_catching_up" => state_sync_failures_catching_up(),
        "twin_validator_test" => twin_validator_test(),
        "large_db_simple_test" => large_db_simple_test(),
        "consensus_only_realistic_env_max_tps" => run_consensus_only_realistic_env_max_tps(),
        "quorum_store_reconfig_enable_test" => quorum_store_reconfig_enable_test(),
        "mainnet_like_simulation_test" => mainnet_like_simulation_test(),
        "gather_metrics" => gather_metrics(),
        _ => return Err(format_err!("Invalid --suite given: {:?}", test_name)),
    };

    Ok(ungrouped_test_suite)
}

/// Provides a forge config that runs the swarm forever (unless killed)
fn run_forever() -> ForgeConfig {
    ForgeConfig::default()
        .add_admin_test(GetMetadata)
        .with_genesis_module_bundle(aptos_cached_packages::head_release_bundle().clone())
        .add_aptos_test(RunForever)
}

fn local_test_suite() -> ForgeConfig {
    ForgeConfig::default()
        .add_aptos_test(FundAccount)
        .add_aptos_test(TransferCoins)
        .add_admin_test(GetMetadata)
        .add_network_test(RestartValidator)
        .add_network_test(EmitTransaction)
        .with_genesis_module_bundle(aptos_cached_packages::head_release_bundle().clone())
}

fn k8s_test_suite() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .add_aptos_test(FundAccount)
        .add_aptos_test(TransferCoins)
        .add_admin_test(GetMetadata)
        .add_network_test(EmitTransaction)
        .add_network_test(FrameworkUpgrade)
        .add_network_test(PerformanceBenchmark)
}

/// Attempts to match the test name to a land-blocking test
fn get_land_blocking_test(
    test_name: &str,
    duration: Duration,
    test_cmd: &TestCommand,
) -> Option<ForgeConfig> {
    let test = match test_name {
        "land_blocking" | "realistic_env_max_load" => {
            realistic_env_max_load_test(duration, test_cmd, 7, 5)
        },
        "compat" => compat(),
        "framework_upgrade" => framework_upgrade(),
        _ => return None, // The test name does not match a land-blocking test
    };
    Some(test)
}

/// Attempts to match the test name to a network benchmark test
fn get_netbench_test(test_name: &str) -> Option<ForgeConfig> {
    let test = match test_name {
        // Network tests without chaos
        "net_bench_no_chaos_1000" => net_bench_no_chaos(MEGABYTE, 1000),
        "net_bench_no_chaos_900" => net_bench_no_chaos(MEGABYTE, 900),
        "net_bench_no_chaos_800" => net_bench_no_chaos(MEGABYTE, 800),
        "net_bench_no_chaos_700" => net_bench_no_chaos(MEGABYTE, 700),
        "net_bench_no_chaos_600" => net_bench_no_chaos(MEGABYTE, 600),
        "net_bench_no_chaos_500" => net_bench_no_chaos(MEGABYTE, 500),
        "net_bench_no_chaos_300" => net_bench_no_chaos(MEGABYTE, 300),
        "net_bench_no_chaos_200" => net_bench_no_chaos(MEGABYTE, 200),
        "net_bench_no_chaos_100" => net_bench_no_chaos(MEGABYTE, 100),
        "net_bench_no_chaos_50" => net_bench_no_chaos(MEGABYTE, 50),
        "net_bench_no_chaos_20" => net_bench_no_chaos(MEGABYTE, 20),
        "net_bench_no_chaos_10" => net_bench_no_chaos(MEGABYTE, 10),
        "net_bench_no_chaos_1" => net_bench_no_chaos(MEGABYTE, 1),

        // Network tests with chaos
        "net_bench_two_region_chaos_1000" => net_bench_two_region_chaos(MEGABYTE, 1000),
        "net_bench_two_region_chaos_500" => net_bench_two_region_chaos(MEGABYTE, 500),
        "net_bench_two_region_chaos_300" => net_bench_two_region_chaos(MEGABYTE, 300),
        "net_bench_two_region_chaos_200" => net_bench_two_region_chaos(MEGABYTE, 200),
        "net_bench_two_region_chaos_100" => net_bench_two_region_chaos(MEGABYTE, 100),
        "net_bench_two_region_chaos_50" => net_bench_two_region_chaos(MEGABYTE, 50),
        "net_bench_two_region_chaos_30" => net_bench_two_region_chaos(MEGABYTE, 30),
        "net_bench_two_region_chaos_20" => net_bench_two_region_chaos(MEGABYTE, 20),
        "net_bench_two_region_chaos_15" => net_bench_two_region_chaos(MEGABYTE, 15),
        "net_bench_two_region_chaos_10" => net_bench_two_region_chaos(MEGABYTE, 10),
        "net_bench_two_region_chaos_1" => net_bench_two_region_chaos(MEGABYTE, 1),

        // Network tests with small messages
        "net_bench_two_region_chaos_small_messages_5" => {
            net_bench_two_region_chaos(100 * KILOBYTE, 50)
        },
        "net_bench_two_region_chaos_small_messages_1" => {
            net_bench_two_region_chaos(100 * KILOBYTE, 10)
        },

        _ => return None, // The test name does not match a network benchmark test
    };
    Some(test)
}

/// Attempts to match the test name to a PFN test
fn get_pfn_test(test_name: &str, duration: Duration) -> Option<ForgeConfig> {
    let test = match test_name {
        "pfn_const_tps" => pfn_const_tps(duration, false, false, true),
        "pfn_const_tps_with_network_chaos" => pfn_const_tps(duration, false, true, false),
        "pfn_const_tps_with_realistic_env" => pfn_const_tps(duration, true, true, false),
        "pfn_performance" => pfn_performance(duration, false, false, true, 7, 1, false),
        "pfn_performance_with_network_chaos" => {
            pfn_performance(duration, false, true, false, 7, 1, false)
        },
        "pfn_performance_with_realistic_env" => {
            pfn_performance(duration, true, true, false, 7, 1, false)
        },
        "pfn_spam_duplicates" => pfn_performance(duration, true, true, true, 7, 7, true),
        _ => return None, // The test name does not match a PFN test
    };
    Some(test)
}

/// Attempts to match the test name to a realistic-env test
fn get_realistic_env_test(
    test_name: &str,
    duration: Duration,
    test_cmd: &TestCommand,
) -> Option<ForgeConfig> {
    let test = match test_name {
        "realistic_env_max_load_large" => realistic_env_max_load_test(duration, test_cmd, 20, 10),
        "realistic_env_load_sweep" => realistic_env_load_sweep_test(),
        "realistic_env_workload_sweep" => realistic_env_workload_sweep_test(),
        "realistic_env_fairness_workload_sweep" => realistic_env_fairness_workload_sweep(),
        "realistic_env_graceful_workload_sweep" => realistic_env_graceful_workload_sweep(),
        "realistic_env_graceful_overload" => realistic_env_graceful_overload(duration),
        "realistic_network_tuned_for_throughput" => realistic_network_tuned_for_throughput_test(),
        _ => return None, // The test name does not match a realistic-env test
    };
    Some(test)
}

/// Attempts to match the test name to a state sync test
fn get_state_sync_test(test_name: &str) -> Option<ForgeConfig> {
    let test = match test_name {
        "state_sync_perf_fullnodes_apply_outputs" => state_sync_perf_fullnodes_apply_outputs(),
        "state_sync_perf_fullnodes_execute_transactions" => {
            state_sync_perf_fullnodes_execute_transactions()
        },
        "state_sync_perf_fullnodes_fast_sync" => state_sync_perf_fullnodes_fast_sync(),
        "state_sync_perf_validators" => state_sync_perf_validators(),
        _ => return None, // The test name does not match a state sync test
    };
    Some(test)
}

/// Attempts to match the test name to a multi-region test
fn get_multi_region_test(test_name: &str) -> Option<ForgeConfig> {
    let test = match test_name {
        "multiregion_benchmark_test" => multiregion_benchmark_test(),
        "three_region_simulation" => three_region_simulation(),
        "three_region_simulation_with_different_node_speed" => {
            three_region_simulation_with_different_node_speed()
        },
        _ => return None, // The test name does not match a multi-region test
    };
    Some(test)
}

fn wrap_with_realistic_env<T: NetworkTest + 'static>(test: T) -> CompositeNetworkTest {
    CompositeNetworkTest::new_with_two_wrappers(
        MultiRegionNetworkEmulationTest::default(),
        CpuChaosTest::default(),
        test,
    )
}

fn mempool_config_practically_non_expiring(mempool_config: &mut MempoolConfig) {
    mempool_config.capacity = 3_000_000;
    mempool_config.capacity_bytes = (3_u64 * 1024 * 1024 * 1024) as usize;
    mempool_config.capacity_per_user = 100_000;
    mempool_config.system_transaction_timeout_secs = 5 * 60 * 60;
    mempool_config.system_transaction_gc_interval_ms = 5 * 60 * 60_000;
}

fn state_sync_config_execute_transactions(state_sync_config: &mut StateSyncConfig) {
    state_sync_config.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ExecuteTransactionsFromGenesis;
    state_sync_config.state_sync_driver.continuous_syncing_mode =
        ContinuousSyncingMode::ExecuteTransactions;
}

fn state_sync_config_apply_transaction_outputs(state_sync_config: &mut StateSyncConfig) {
    state_sync_config.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ApplyTransactionOutputsFromGenesis;
    state_sync_config.state_sync_driver.continuous_syncing_mode =
        ContinuousSyncingMode::ApplyTransactionOutputs;
}

fn state_sync_config_fast_sync(state_sync_config: &mut StateSyncConfig) {
    state_sync_config.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestStates;
    state_sync_config.state_sync_driver.continuous_syncing_mode =
        ContinuousSyncingMode::ApplyTransactionOutputs;
}

fn wrap_with_two_region_env<T: NetworkTest + 'static>(test: T) -> CompositeNetworkTest {
    CompositeNetworkTest::new(
        MultiRegionNetworkEmulationTest::new_with_config(
            MultiRegionNetworkEmulationConfig::two_region(),
        ),
        test,
    )
}

fn run_consensus_only_realistic_env_max_tps() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 300000,
                })
                .txn_expiration_time_secs(5 * 60),
        )
        .add_network_test(CompositeNetworkTest::new(
            MultiRegionNetworkEmulationTest::default(),
            CpuChaosTest::default(),
        ))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            optimize_for_maximum_throughput(config, 20_000, 4_500, 3.0);
            state_sync_config_execute_transactions(&mut config.state_sync);
        }))
        // TODO(ibalajiarun): tune these success critiera after we have a better idea of the test behavior
        .with_success_criteria(
            SuccessCriteria::new(10000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

fn quorum_store_backlog_txn_limit_count(
    config: &mut NodeConfig,
    target_tps: usize,
    vn_latency: f64,
) {
    config
        .consensus
        .quorum_store
        .back_pressure
        .backlog_txn_limit_count = (target_tps as f64 * vn_latency) as u64;
    config
        .consensus
        .quorum_store
        .back_pressure
        .dynamic_max_txn_per_s = 4000;
}

fn optimize_for_maximum_throughput(
    config: &mut NodeConfig,
    target_tps: usize,
    max_txns_per_block: usize,
    vn_latency: f64,
) {
    mempool_config_practically_non_expiring(&mut config.mempool);

    config.consensus.max_sending_block_txns_after_filtering = max_txns_per_block as u64;
    config.consensus.max_sending_block_txns = config
        .consensus
        .max_sending_block_txns
        .max(max_txns_per_block as u64 * 3 / 2);
    config.consensus.max_receiving_block_txns =
        (config.consensus.max_sending_block_txns as f64 * 4.0 / 3.0) as u64;
    config.consensus.max_sending_block_bytes = 10 * 1024 * 1024;
    config.consensus.max_receiving_block_bytes = 12 * 1024 * 1024;
    config.consensus.pipeline_backpressure = vec![];
    config.consensus.chain_health_backoff = vec![];

    quorum_store_backlog_txn_limit_count(config, target_tps, vn_latency);

    config.consensus.quorum_store.sender_max_batch_txns = 500;
    config
        .consensus
        .min_max_txns_in_block_after_filtering_from_backpressure =
        2 * config.consensus.quorum_store.sender_max_batch_txns as u64;
    config.consensus.quorum_store.sender_max_batch_bytes = 4 * 1024 * 1024;
    config.consensus.quorum_store.sender_max_num_batches = 100;
    config.consensus.quorum_store.sender_max_total_txns = 4000;
    config.consensus.quorum_store.sender_max_total_bytes = 8 * 1024 * 1024;
    config.consensus.quorum_store.receiver_max_batch_txns = 1000;
    config.consensus.quorum_store.receiver_max_batch_bytes = 8 * 1024 * 1024;
    config.consensus.quorum_store.receiver_max_num_batches = 200;
    config.consensus.quorum_store.receiver_max_total_txns = 8000;
    config.consensus.quorum_store.receiver_max_total_bytes = 16 * 1024 * 1024;
}

fn large_db_simple_test() -> ForgeConfig {
    large_db_test(10, 500, 300, "10-validators".to_string())
}

fn twin_validator_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(5)
        .add_network_test(TwinValidatorTest)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(5500)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_system_metrics_threshold(SYSTEM_12_CORES_5GB_THRESHOLD.clone())
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

fn state_sync_failures_catching_up() -> ForgeConfig {
    changing_working_quorum_test_helper(
        7,
        300,
        3000,
        2500,
        true,
        false,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 1500,
            always_healthy_nodes: 2,
            max_down_nodes: 1,
            num_large_validators: 2,
            add_execution_delay: false,
            check_period_s: 27,
        },
    )
}

fn state_sync_slow_processing_catching_up() -> ForgeConfig {
    changing_working_quorum_test_helper(
        7,
        300,
        3000,
        2500,
        true,
        true,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 750,
            always_healthy_nodes: 2,
            max_down_nodes: 0,
            num_large_validators: 2,
            add_execution_delay: true,
            check_period_s: 57,
        },
    )
}

fn different_node_speed_and_reliability_test() -> ForgeConfig {
    changing_working_quorum_test_helper(
        20,
        120,
        70,
        50,
        true,
        false,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 30,
            always_healthy_nodes: 6,
            max_down_nodes: 5,
            num_large_validators: 3,
            add_execution_delay: true,
            check_period_s: 27,
        },
    )
}

fn large_test_only_few_nodes_down() -> ForgeConfig {
    changing_working_quorum_test_helper(
        60,
        120,
        100,
        70,
        false,
        false,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 50,
            always_healthy_nodes: 40,
            max_down_nodes: 10,
            num_large_validators: 0,
            add_execution_delay: false,
            check_period_s: 27,
        },
    )
}

fn changing_working_quorum_test_high_load() -> ForgeConfig {
    changing_working_quorum_test_helper(
        16,
        120,
        500,
        300,
        false,
        true,
        true,
        ChangingWorkingQuorumTest {
            min_tps: 50,
            always_healthy_nodes: 0,
            max_down_nodes: 16,
            num_large_validators: 0,
            add_execution_delay: false,
            // Use longer check duration, as we are bringing enough nodes
            // to require state-sync to catch up to have consensus.
            check_period_s: 53,
        },
    )
}

fn changing_working_quorum_test() -> ForgeConfig {
    changing_working_quorum_test_helper(
        16,
        120,
        100,
        70,
        true,
        true,
        true,
        ChangingWorkingQuorumTest {
            min_tps: 15,
            always_healthy_nodes: 0,
            max_down_nodes: 16,
            num_large_validators: 0,
            add_execution_delay: false,
            // Use longer check duration, as we are bringing enough nodes
            // to require state-sync to catch up to have consensus.
            check_period_s: 53,
        },
    )
}

fn consensus_stress_test() -> ForgeConfig {
    changing_working_quorum_test_helper(
        10,
        60,
        100,
        80,
        true,
        false,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 50,
            always_healthy_nodes: 10,
            max_down_nodes: 0,
            num_large_validators: 0,
            add_execution_delay: false,
            check_period_s: 27,
        },
    )
}

fn realistic_env_sweep_wrap(
    num_validators: usize,
    num_fullnodes: usize,
    test: LoadVsPerfBenchmark,
) -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .add_network_test(wrap_with_realistic_env(test))
        // Test inherits the main EmitJobRequest, so update here for more precise latency measurements
        .with_emit_job(
            EmitJobRequest::default().latency_polling_interval(Duration::from_millis(100)),
        )
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(0)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_chain_progress(RELIABLE_REAL_ENV_PROGRESS_THRESHOLD.clone()),
        )
}

fn background_emit_request() -> EmitJobRequest {
    EmitJobRequest::default()
        .num_accounts_mode(NumAccountsMode::TransactionsPerAccount(1))
        .mode(EmitJobMode::ConstTps { tps: 10 })
        .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
}

fn background_traffic_for_sweep(num_cases: usize) -> Option<BackgroundTraffic> {
    Some(BackgroundTraffic {
        traffic: background_emit_request(),
        criteria: std::iter::repeat(9.5)
            .take(num_cases)
            .map(|min_tps| {
                SuccessCriteria::new_float(min_tps)
                    .add_max_expired_tps(0.1)
                    .add_max_failed_submission_tps(0.0)
            })
            .collect(),
    })
}

fn background_traffic_for_sweep_with_latency(criteria: &[(f32, f32)]) -> Option<BackgroundTraffic> {
    Some(BackgroundTraffic {
        traffic: background_emit_request(),
        criteria: criteria
            .iter()
            .map(|(p50, p90)| {
                SuccessCriteria::new_float(9.5)
                    .add_max_expired_tps(0.1)
                    .add_max_failed_submission_tps(0.0)
                    .add_latency_threshold(*p50, LatencyType::P50)
                    .add_latency_threshold(*p90, LatencyType::P90)
            })
            .collect(),
    })
}

fn realistic_env_load_sweep_test() -> ForgeConfig {
    realistic_env_sweep_wrap(20, 10, LoadVsPerfBenchmark {
        test: Box::new(PerformanceBenchmark),
        workloads: Workloads::TPS(vec![10, 100, 1000, 3000, 5000, 7000]),
        criteria: [
            (9, 0.9, 0.9, 1.2, 0),
            (95, 0.9, 1.0, 1.2, 0),
            (950, 1.2, 1.3, 2.0, 0),
            (2900, 1.4, 2.2, 2.5, 0),
            (4800, 2.0, 2.5, 3.0, 0),
            (6700, 2.5, 3.5, 5.0, 0),
            // TODO add 9k or 10k. Allow some expired transactions (high-load)
        ]
        .into_iter()
        .map(
            |(min_tps, max_lat_p50, max_lat_p90, max_lat_p99, max_expired_tps)| {
                SuccessCriteria::new(min_tps)
                    .add_max_expired_tps(max_expired_tps as f64)
                    .add_max_failed_submission_tps(0.0)
                    .add_latency_threshold(max_lat_p50, LatencyType::P50)
                    .add_latency_threshold(max_lat_p90, LatencyType::P90)
                    .add_latency_threshold(max_lat_p99, LatencyType::P99)
            },
        )
        .collect(),
        background_traffic: background_traffic_for_sweep(5),
    })
}

fn realistic_env_workload_sweep_test() -> ForgeConfig {
    realistic_env_sweep_wrap(7, 3, LoadVsPerfBenchmark {
        test: Box::new(PerformanceBenchmark),
        workloads: Workloads::TRANSACTIONS(vec![
            TransactionWorkload::new(TransactionTypeArg::CoinTransfer, 20000),
            TransactionWorkload::new(TransactionTypeArg::NoOp, 20000).with_num_modules(100),
            TransactionWorkload::new(TransactionTypeArg::ModifyGlobalResource, 6000)
                .with_transactions_per_account(1),
            TransactionWorkload::new(TransactionTypeArg::TokenV2AmbassadorMint, 20000)
                .with_unique_senders(),
            // TODO(ibalajiarun): this is disabled due to Forge Stable failure on PosToProposal latency.
            TransactionWorkload::new(TransactionTypeArg::PublishPackage, 200)
                .with_transactions_per_account(1),
        ]),
        // Investigate/improve to make latency more predictable on different workloads
        criteria: [
            (7000, 100, 0.3, 0.5, 0.5, 0.4),
            (8500, 100, 0.3, 0.5, 0.5, 0.4),
            (2000, 300, 0.3, 1.0, 0.6, 1.0),
            (3200, 500, 0.3, 1.0, 0.7, 0.6),
            // TODO - pos-to-proposal is set to high, until it is calibrated/understood.
            (28, 5, 0.3, 5.0, 0.7, 1.0),
        ]
        .into_iter()
        .map(
            |(
                min_tps,
                max_expired,
                batch_to_pos,
                pos_to_proposal,
                proposal_to_ordered,
                ordered_to_commit,
            )| {
                SuccessCriteria::new(min_tps)
                    .add_max_expired_tps(max_expired as f64)
                    .add_max_failed_submission_tps(200.0)
                    .add_latency_breakdown_threshold(LatencyBreakdownThreshold::new_strict(vec![
                        (LatencyBreakdownSlice::QsBatchToPos, batch_to_pos),
                        (LatencyBreakdownSlice::QsPosToProposal, pos_to_proposal),
                        (
                            LatencyBreakdownSlice::ConsensusProposalToOrdered,
                            proposal_to_ordered,
                        ),
                        (
                            LatencyBreakdownSlice::ConsensusOrderedToCommit,
                            ordered_to_commit,
                        ),
                    ]))
            },
        )
        .collect(),
        background_traffic: background_traffic_for_sweep(5),
    })
}

fn realistic_env_fairness_workload_sweep() -> ForgeConfig {
    realistic_env_sweep_wrap(7, 3, LoadVsPerfBenchmark {
        test: Box::new(PerformanceBenchmark),
        workloads: Workloads::TRANSACTIONS(vec![
            // Very high gas
            TransactionWorkload::new(
                TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB,
                100000,
            ),
            TransactionWorkload::new(TransactionTypeArg::VectorPicture30k, 20000),
            TransactionWorkload::new(TransactionTypeArg::SmartTablePicture1MWith256Change, 4000)
                .with_transactions_per_account(1),
        ]),
        criteria: Vec::new(),
        background_traffic: background_traffic_for_sweep_with_latency(&[
            (3.0, 8.0),
            (3.0, 8.0),
            (3.0, 4.0),
        ]),
    })
}

fn realistic_env_graceful_workload_sweep() -> ForgeConfig {
    realistic_env_sweep_wrap(7, 3, LoadVsPerfBenchmark {
        test: Box::new(PerformanceBenchmark),
        workloads: Workloads::TRANSACTIONS(vec![
            // do account generation first, to fill up a storage a bit.
            TransactionWorkload::new_const_tps(TransactionTypeArg::AccountGeneration, 2 * 7000),
            // Very high gas
            TransactionWorkload::new_const_tps(
                TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB,
                3 * 1800,
            ),
            TransactionWorkload::new_const_tps(
                TransactionTypeArg::SmartTablePicture1MWith256Change,
                3 * 14,
            ),
            TransactionWorkload::new_const_tps(
                TransactionTypeArg::SmartTablePicture1MWith1KChangeExceedsLimit,
                3 * 12,
            ),
            TransactionWorkload::new_const_tps(TransactionTypeArg::VectorPicture30k, 3 * 150),
            TransactionWorkload::new_const_tps(TransactionTypeArg::ModifyGlobalFlagAggV2, 3 * 3500),
            // publishing package - executes sequentially
            TransactionWorkload::new_const_tps(TransactionTypeArg::PublishPackage, 3 * 150)
                .with_transactions_per_account(1),
        ]),
        criteria: Vec::new(),
        background_traffic: background_traffic_for_sweep_with_latency(&[
            (4.0, 5.0),
            (2.2, 3.0),
            (3.5, 5.0),
            (4.0, 6.0),
            (2.5, 4.0),
            (3.5, 5.0),
            // TODO - p50 and p90 is set to high, until it is calibrated/understood.
            (3.0, 10.0),
        ]),
    })
    .with_emit_job(
        EmitJobRequest::default()
            .txn_expiration_time_secs(20)
            .init_gas_price_multiplier(5)
            .init_expiration_multiplier(6.0),
    )
}

fn load_vs_perf_benchmark() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(10)
        .add_network_test(LoadVsPerfBenchmark {
            test: Box::new(PerformanceBenchmark),
            workloads: Workloads::TPS(vec![
                200, 1000, 3000, 5000, 7000, 7500, 8000, 9000, 10000, 12000, 15000,
            ]),
            criteria: Vec::new(),
            background_traffic: None,
        })
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(0)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_chain_progress(RELIABLE_REAL_ENV_PROGRESS_THRESHOLD.clone()),
        )
}

fn workload_vs_perf_benchmark() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .add_network_test(LoadVsPerfBenchmark {
            test: Box::new(PerformanceBenchmark),
            workloads: Workloads::TRANSACTIONS(vec![
                TransactionWorkload::new(TransactionTypeArg::NoOp, 20000),
                TransactionWorkload::new(TransactionTypeArg::NoOp, 20000).with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::NoOp, 20000).with_num_modules(1000),
                TransactionWorkload::new(TransactionTypeArg::CoinTransfer, 20000)
                    .with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::CoinTransfer, 20000)
                    .with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::AccountResource32B, 20000)
                    .with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::AccountResource1KB, 20000)
                    .with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::PublishPackage, 20000)
                    .with_unique_senders(),
            ]),
            criteria: Vec::new(),
            background_traffic: None,
        })
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(0)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_chain_progress(RELIABLE_REAL_ENV_PROGRESS_THRESHOLD.clone()),
        )
}

fn realistic_env_graceful_overload(duration: Duration) -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(20)
        .add_network_test(wrap_with_realistic_env(TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 15000 })
                .init_gas_price_multiplier(20),
            inner_success_criteria: SuccessCriteria::new(7500),
        }))
        // First start higher gas-fee traffic, to not cause issues with TxnEmitter setup - account creation
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 1000 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE),
        )
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(900)
                .add_no_restarts()
                .add_wait_for_catchup_s(180) // 3 minutes
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // overload test uses more CPUs than others, so increase the limit
                    // Check that we don't use more than 28 CPU cores for 20% of the time.
                    MetricsThreshold::new(28.0, 20),
                    // Memory starts around 6GB, and grows around 8GB/hr in this test.
                    // Check that we don't use more than final expected memory for more than 20% of the time.
                    MetricsThreshold::new_gb(6.5 + 8.0 * (duration.as_secs_f64() / 3600.0), 20),
                ))
                .add_latency_threshold(10.0, LatencyType::P50)
                .add_latency_threshold(30.0, LatencyType::P90)
                .add_chain_progress(RELIABLE_REAL_ENV_PROGRESS_THRESHOLD.clone()),
        )
}

fn mixed_emit_job() -> EmitJobRequest {
    EmitJobRequest::default()
        .mode(EmitJobMode::MaxLoad {
            mempool_backlog: 10000,
        })
        .transaction_mix(vec![
            // To test both variants, make module publish with such frequency, so that there are
            // similar number of sequential and parallel blocks.
            // For other transactions, make more expensive transactions somewhat rarer.
            (
                TransactionTypeArg::AccountGeneration.materialize_default(),
                10000,
            ),
            (
                TransactionTypeArg::CoinTransfer.materialize_default(),
                10000,
            ),
            (TransactionTypeArg::PublishPackage.materialize_default(), 3),
            (
                TransactionTypeArg::Batch100Transfer.materialize_default(),
                100,
            ),
            (
                TransactionTypeArg::VectorPicture30k.materialize_default(),
                100,
            ),
            (
                TransactionTypeArg::SmartTablePicture30KWith200Change.materialize(
                    1,
                    true,
                    WorkflowProgress::when_done_default(),
                ),
                100,
            ),
            (
                TransactionTypeArg::TokenV2AmbassadorMint.materialize_default(),
                10000,
            ),
            (
                TransactionTypeArg::ModifyGlobalResource.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ModifyGlobalResourceAggV2.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ModifyGlobalFlagAggV2.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ModifyGlobalBoundedAggV2.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ResourceGroupsGlobalWriteTag1KB.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::TokenV1NFTMintAndTransferSequential.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::TokenV1FTMintAndTransfer.materialize_default(),
                10000,
            ),
        ])
}

fn workload_mix_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_initial_fullnode_count(3)
        .add_network_test(PerformanceBenchmark)
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .with_emit_job(mixed_emit_job())
        .with_success_criteria(
            SuccessCriteria::new(3000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

fn individual_workload_tests(test_name: String) -> ForgeConfig {
    let job = EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
        mempool_backlog: 30000,
    });
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_initial_fullnode_count(3)
        .add_network_test(PerformanceBenchmark)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .with_emit_job(
            if test_name == "write_new_resource" {
                let account_creation_type = TransactionType::AccountGeneration {
                    add_created_accounts_to_pool: true,
                    max_account_working_set: 20_000_000,
                    creation_balance: 200_000_000,
                };
                let write_type = TransactionType::CallCustomModules {
                    entry_point: EntryPoints::BytesMakeOrChange {
                        data_length: Some(32),
                    },
                    num_modules: 1,
                    use_account_pool: true,
                };
                job.transaction_mix_per_phase(vec![
                    // warmup
                    vec![(account_creation_type, 1)],
                    vec![(account_creation_type, 1)],
                    vec![(write_type, 1)],
                    // cooldown
                    vec![(write_type, 1)],
                ])
            } else {
                job.transaction_type(match test_name.as_str() {
                    "account_creation" => {
                        TransactionTypeArg::AccountGeneration.materialize_default()
                    },
                    "publishing" => TransactionTypeArg::PublishPackage.materialize_default(),
                    "module_loading" => TransactionTypeArg::NoOp.materialize(
                        1000,
                        false,
                        WorkflowProgress::when_done_default(),
                    ),
                    _ => unreachable!("{}", test_name),
                })
            },
        )
        .with_success_criteria(
            SuccessCriteria::new(match test_name.as_str() {
                "account_creation" => 3600,
                "publishing" => 60,
                "write_new_resource" => 3700,
                "module_loading" => 1800,
                _ => unreachable!("{}", test_name),
            })
            .add_no_restarts()
            .add_wait_for_catchup_s(240)
            .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

fn fullnode_reboot_stress_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .add_network_test(FullNodeRebootStressTest)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .with_success_criteria(SuccessCriteria::new(2000).add_wait_for_catchup_s(600))
}

fn validator_reboot_stress_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(1)
        .add_network_test(ValidatorRebootStressTest {
            num_simultaneously: 2,
            down_time_secs: 5.0,
            pause_secs: 5.0,
        })
        .with_success_criteria(SuccessCriteria::new(2000).add_wait_for_catchup_s(600))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 120.into();
        }))
}

fn apply_config_for_quorum_store_single_node(config: &mut NodeConfig) {
    config
        .consensus
        .quorum_store
        .back_pressure
        .dynamic_max_txn_per_s = 5500;
}

fn single_vfn_perf() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(1).unwrap())
        .with_initial_fullnode_count(1)
        .add_network_test(PerformanceBenchmark)
        .with_success_criteria(
            SuccessCriteria::new(5000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240),
        )
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            apply_config_for_quorum_store_single_node(config);
        }))
}

fn setup_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(1).unwrap())
        .with_initial_fullnode_count(1)
        .add_network_test(ForgeSetupTest)
}

fn network_bandwidth() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(8).unwrap())
        .add_network_test(NetworkBandwidthTest)
}

fn gather_metrics() -> ForgeConfig {
    ForgeConfig::default()
        .add_network_test(GatherMetrics)
        .add_network_test(Delay::new(180))
        .add_network_test(GatherMetrics)
}

/// Creates a netbench configuration for direct send using
/// the specified message size and frequency.
fn create_direct_send_netbench_config(
    message_size: usize,
    message_frequency: u64,
) -> NetbenchConfig {
    // Create the netbench config
    let mut netbench_config = NetbenchConfig::default();

    // Enable direct send network benchmarking
    netbench_config.enabled = true;
    netbench_config.enable_direct_send_testing = true;

    // Configure the message sizes and frequency
    netbench_config.direct_send_data_size = message_size;
    netbench_config.direct_send_per_second = message_frequency;
    netbench_config.max_network_channel_size = message_frequency * 2; // Double the channel size for an additional buffer

    netbench_config
}

/// Performs direct send network benchmarking between 2 validators
/// using the specified message size and frequency.
fn net_bench_no_chaos(message_size: usize, message_frequency: u64) -> ForgeConfig {
    ForgeConfig::default()
        .add_network_test(Delay::new(180))
        .with_initial_validator_count(NonZeroUsize::new(2).unwrap())
        .with_validator_override_node_config_fn(Arc::new(move |config, _| {
            let netbench_config =
                create_direct_send_netbench_config(message_size, message_frequency);
            config.netbench = Some(netbench_config);
        }))
}

/// Performs direct send network benchmarking between 2 validators
/// using the specified message size and frequency, with two-region chaos.
fn net_bench_two_region_chaos(message_size: usize, message_frequency: u64) -> ForgeConfig {
    net_bench_two_region_inner(create_direct_send_netbench_config(
        message_size,
        message_frequency,
    ))
}

/// A simple utility function for creating a ForgeConfig with a
/// two-region environment using the specified netbench config.
fn net_bench_two_region_inner(netbench_config: NetbenchConfig) -> ForgeConfig {
    ForgeConfig::default()
        .add_network_test(wrap_with_two_region_env(Delay::new(180)))
        .with_initial_validator_count(NonZeroUsize::new(2).unwrap())
        .with_validator_override_node_config_fn(Arc::new(move |config, _| {
            config.netbench = Some(netbench_config);
        }))
}

fn three_region_simulation_with_different_node_speed() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .with_initial_fullnode_count(30)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .add_network_test(CompositeNetworkTest::new(
            ExecutionDelayTest {
                add_execution_delay: ExecutionDelayConfig {
                    inject_delay_node_fraction: 0.5,
                    inject_delay_max_transaction_percentage: 40,
                    inject_delay_per_transaction_ms: 2,
                },
            },
            ThreeRegionSameCloudSimulationTest,
        ))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.api.failpoints_enabled = true;
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_execute_transactions(&mut config.state_sync);
        }))
        .with_success_criteria(
            SuccessCriteria::new(1000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

fn three_region_simulation() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(12).unwrap())
        .with_initial_fullnode_count(12)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .add_network_test(ThreeRegionSameCloudSimulationTest)
        // TODO(rustielin): tune these success criteria after we have a better idea of the test behavior
        .with_success_criteria(
            SuccessCriteria::new(3000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

fn network_partition() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
        .add_network_test(NetworkPartitionTest)
        .with_success_criteria(
            SuccessCriteria::new(2500)
                .add_no_restarts()
                .add_wait_for_catchup_s(240),
        )
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            apply_config_for_quorum_store_single_node(config);
        }))
}

fn compat() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(SimpleValidatorUpgrade)
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] =
                SimpleValidatorUpgrade::EPOCH_DURATION_SECS.into();
        }))
}

fn framework_upgrade() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(FrameworkUpgrade)
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] =
                FrameworkUpgrade::EPOCH_DURATION_SECS.into();
        }))
        .with_emit_job(mixed_emit_job())
}

fn epoch_changer_performance() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_initial_fullnode_count(2)
        .add_network_test(PerformanceBenchmark)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 60.into();
        }))
}

/// A default config for running various state sync performance tests
fn state_sync_perf_fullnodes_config() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .with_initial_fullnode_count(4)
}

/// The config for running a state sync performance test when applying
/// transaction outputs in fullnodes.
fn state_sync_perf_fullnodes_apply_outputs() -> ForgeConfig {
    state_sync_perf_fullnodes_config()
        .add_network_test(StateSyncFullnodePerformance)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_apply_transaction_outputs(&mut config.state_sync);
        }))
        .with_success_criteria(SuccessCriteria::new(9000))
}

/// The config for running a state sync performance test when executing
/// transactions in fullnodes.
fn state_sync_perf_fullnodes_execute_transactions() -> ForgeConfig {
    state_sync_perf_fullnodes_config()
        .add_network_test(StateSyncFullnodePerformance)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_execute_transactions(&mut config.state_sync);
        }))
        .with_success_criteria(SuccessCriteria::new(5000))
}

/// The config for running a state sync performance test when fast syncing
/// to the latest epoch.
fn state_sync_perf_fullnodes_fast_sync() -> ForgeConfig {
    state_sync_perf_fullnodes_config()
        .add_network_test(StateSyncFullnodeFastSyncPerformance)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 180.into(); // Frequent epochs
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 30000,
                })
                .transaction_type(TransactionTypeArg::AccountGeneration.materialize_default()), // Create many state values
        )
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_fast_sync(&mut config.state_sync);
        }))
}

/// The config for running a state sync performance test when applying
/// transaction outputs in failed validators.
fn state_sync_perf_validators() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_apply_transaction_outputs(&mut config.state_sync);
        }))
        .add_network_test(StateSyncValidatorPerformance)
        .with_success_criteria(SuccessCriteria::new(5000))
}

/// The config for running a validator join and leave test.
fn validators_join_and_leave() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 60.into();
            helm_values["chain"]["allow_new_validators"] = true.into();
        }))
        .add_network_test(ValidatorJoinLeaveTest)
        .with_success_criteria(
            SuccessCriteria::new(5000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_system_metrics_threshold(SYSTEM_12_CORES_10GB_THRESHOLD.clone())
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

fn realistic_env_max_load_test(
    duration: Duration,
    test_cmd: &TestCommand,
    num_validators: usize,
    num_fullnodes: usize,
) -> ForgeConfig {
    // Check if HAProxy is enabled
    let ha_proxy = if let TestCommand::K8sSwarm(k8s) = test_cmd {
        k8s.enable_haproxy
    } else {
        false
    };

    // Determine if this is a long running test
    let duration_secs = duration.as_secs();
    let long_running = duration_secs >= 2400;

    // resource override for long_running tests
    let resource_override = if long_running {
        NodeResourceOverride {
            storage_gib: Some(1000), // long running tests need more storage
            ..NodeResourceOverride::default()
        }
    } else {
        NodeResourceOverride::default() // no overrides
    };

    let mut success_criteria = SuccessCriteria::new(95)
        .add_system_metrics_threshold(SystemMetricsThreshold::new(
            // Check that we don't use more than 18 CPU cores for 15% of the time.
            MetricsThreshold::new(25.0, 15),
            // Memory starts around 7GB, and grows around 1.4GB/hr in this test.
            // Check that we don't use more than final expected memory for more than 20% of the time.
            MetricsThreshold::new_gb(7.0 + 1.4 * (duration_secs as f64 / 3600.0), 20),
        ))
        .add_no_restarts()
        .add_wait_for_catchup_s(
            // Give at least 60s for catchup, give 10% of the run for longer durations.
            (duration.as_secs() / 10).max(60),
        )
        .add_latency_threshold(3.4, LatencyType::P50)
        .add_latency_threshold(4.5, LatencyType::P70)
        .add_chain_progress(StateProgressThreshold {
            max_non_epoch_no_progress_secs: 15.0,
            max_epoch_no_progress_secs: 15.0,
            max_non_epoch_round_gap: 4,
            max_epoch_round_gap: 4,
        });
    if !ha_proxy {
        success_criteria = success_criteria.add_latency_breakdown_threshold(
            LatencyBreakdownThreshold::new_with_breach_pct(
                vec![
                    (LatencyBreakdownSlice::QsBatchToPos, 0.35),
                    // quorum store backpressure is relaxed, so queueing happens here
                    (LatencyBreakdownSlice::QsPosToProposal, 2.5),
                    // can be adjusted down if less backpressure
                    (LatencyBreakdownSlice::ConsensusProposalToOrdered, 0.85),
                    // can be adjusted down if less backpressure
                    (LatencyBreakdownSlice::ConsensusOrderedToCommit, 1.0),
                ],
                5,
            ),
        )
    }

    // Create the test
    let mempool_backlog = if ha_proxy { 30000 } else { 40000 };
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .add_network_test(wrap_with_realistic_env(TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad { mempool_backlog })
                .init_gas_price_multiplier(20),
            inner_success_criteria: SuccessCriteria::new(
                if ha_proxy {
                    7000
                } else if long_running {
                    // This is for forge stable
                    11000
                } else {
                    // During land time we want to be less strict, otherwise we flaky fail
                    10000
                },
            ),
        }))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            // Have single epoch change in land blocking, and a few on long-running
            helm_values["chain"]["epoch_duration_secs"] =
                (if long_running { 600 } else { 300 }).into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::default_for_genesis())
                    .expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
        }))
        // First start higher gas-fee traffic, to not cause issues with TxnEmitter setup - account creation
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(success_criteria)
        .with_validator_resource_override(resource_override)
        .with_fullnode_resource_override(resource_override)
}

fn realistic_network_tuned_for_throughput_test() -> ForgeConfig {
    // THE MOST COMMONLY USED TUNE-ABLES:
    const USE_CRAZY_MACHINES: bool = false;
    const ENABLE_VFNS: bool = true;
    const VALIDATOR_COUNT: usize = 12;

    // Config is based on these values. The target TPS should be a slight overestimate of
    // the actual throughput to be able to have reasonable queueing but also so throughput
    // will improve as performance improves.
    // Overestimate: causes mempool and/or batch queueing. Underestimate: not enough txns in blocks.
    const TARGET_TPS: usize = 15_000;
    // Overestimate: causes blocks to be too small. Underestimate: causes blocks that are too large.
    // Ideally, want the block size to take 200-250ms of execution time to match broadcast RTT.
    const MAX_TXNS_PER_BLOCK: usize = 3500;
    // Overestimate: causes batch queueing. Underestimate: not enough txns in quorum store.
    // This is validator latency, minus mempool queueing time.
    const VN_LATENCY_S: f64 = 2.5;
    // Overestimate: causes mempool queueing. Underestimate: not enough txns incoming.
    const VFN_LATENCY_S: f64 = 4.0;

    let mut forge_config = ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(VALIDATOR_COUNT).unwrap())
        .add_network_test(MultiRegionNetworkEmulationTest::default())
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
            mempool_backlog: (TARGET_TPS as f64 * VFN_LATENCY_S) as usize,
        }))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            // Increase the state sync chunk sizes (consensus blocks are much larger than 1k)
            optimize_state_sync_for_throughput(config, 15_000);

            optimize_for_maximum_throughput(config, TARGET_TPS, MAX_TXNS_PER_BLOCK, VN_LATENCY_S);

            // Other consensus / Quroum store configs
            config.consensus.quorum_store_pull_timeout_ms = 200;

            // Experimental storage optimizations
            config.storage.rocksdb_configs.enable_storage_sharding = true;

            // Increase the concurrency level
            if USE_CRAZY_MACHINES {
                config.execution.concurrency_level = 48;
            }
        }))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            let mut on_chain_execution_config = OnChainExecutionConfig::default_for_genesis();
            // Need to update if the default changes
            match &mut on_chain_execution_config {
                OnChainExecutionConfig::Missing
                | OnChainExecutionConfig::V1(_)
                | OnChainExecutionConfig::V2(_)
                | OnChainExecutionConfig::V3(_) => {
                    unreachable!("Unexpected on-chain execution config type, if OnChainExecutionConfig::default_for_genesis() has been updated, this test must be updated too.")
                }
                OnChainExecutionConfig::V4(config_v4) => {
                    config_v4.block_gas_limit_type = BlockGasLimitType::NoLimit;
                    config_v4.transaction_shuffler_type = TransactionShufflerType::SenderAwareV2(256);
                }
            }
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(on_chain_execution_config).expect("must serialize");
        }));

    if ENABLE_VFNS {
        forge_config = forge_config
            .with_initial_fullnode_count(VALIDATOR_COUNT)
            .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
                // Increase the state sync chunk sizes (consensus blocks are much larger than 1k)
                optimize_state_sync_for_throughput(config, 15_000);

                // Experimental storage optimizations
                config.storage.rocksdb_configs.enable_storage_sharding = true;

                // Increase the concurrency level
                if USE_CRAZY_MACHINES {
                    config.execution.concurrency_level = 48;
                }
            }));
    }

    if USE_CRAZY_MACHINES {
        forge_config = forge_config
            .with_validator_resource_override(NodeResourceOverride {
                cpu_cores: Some(58),
                memory_gib: Some(200),
                storage_gib: Some(500), // assuming we're using these large marchines for long-running or expensive tests which need more disk
            })
            .with_fullnode_resource_override(NodeResourceOverride {
                cpu_cores: Some(58),
                memory_gib: Some(200),
                storage_gib: Some(500),
            })
            .with_success_criteria(
                SuccessCriteria::new(25000)
                    .add_no_restarts()
                    /* This test runs at high load, so we need more catchup time */
                    .add_wait_for_catchup_s(120),
                /* Doesn't work without event indices
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
                 */
            );
    } else {
        forge_config = forge_config.with_success_criteria(
            SuccessCriteria::new(12000)
                .add_no_restarts()
                /* This test runs at high load, so we need more catchup time */
                .add_wait_for_catchup_s(120),
            /* Doesn't work without event indices
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
            */
        );
    }

    forge_config
}

/// Optimizes the state sync configs for throughput.
/// `max_chunk_size` is the maximum number of transactions to include in a chunk.
fn optimize_state_sync_for_throughput(node_config: &mut NodeConfig, max_chunk_size: u64) {
    let max_chunk_bytes = 40 * 1024 * 1024; // 10x the current limit (to prevent execution fallback)

    // Update the chunk sizes for the data client
    let data_client_config = &mut node_config.state_sync.aptos_data_client;
    data_client_config.max_transaction_chunk_size = max_chunk_size;
    data_client_config.max_transaction_output_chunk_size = max_chunk_size;

    // Update the chunk sizes for the storage service
    let storage_service_config = &mut node_config.state_sync.storage_service;
    storage_service_config.max_transaction_chunk_size = max_chunk_size;
    storage_service_config.max_transaction_output_chunk_size = max_chunk_size;

    // Update the chunk bytes for the storage service
    storage_service_config.max_network_chunk_bytes = max_chunk_bytes;
}

fn pre_release_suite() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .add_network_test(NetworkBandwidthTest)
}

fn chaos_test_suite(duration: Duration) -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .add_network_test(NetworkBandwidthTest)
        .add_network_test(ThreeRegionSameCloudSimulationTest)
        .add_network_test(NetworkLossTest)
        .with_success_criteria(
            SuccessCriteria::new(
                if duration > Duration::from_secs(1200) {
                    100
                } else {
                    1000
                },
            )
            .add_no_restarts()
            .add_system_metrics_threshold(SYSTEM_12_CORES_5GB_THRESHOLD.clone()),
        )
}

pub fn changing_working_quorum_test_helper(
    num_validators: usize,
    epoch_duration: usize,
    target_tps: usize,
    min_avg_tps: usize,
    apply_txn_outputs: bool,
    use_chain_backoff: bool,
    allow_errors: bool,
    test: ChangingWorkingQuorumTest,
) -> ForgeConfig {
    let config = ForgeConfig::default();
    let num_large_validators = test.num_large_validators;
    let max_down_nodes = test.max_down_nodes;
    config
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(
            if max_down_nodes == 0 {
                0
            } else {
                std::cmp::max(2, target_tps / 1000)
            },
        )
        .add_network_test(test)
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = epoch_duration.into();
            helm_values["genesis"]["validator"]["num_validators_with_larger_stake"] =
                num_large_validators.into();
        }))
        .with_validator_override_node_config_fn(Arc::new(move |config, _| {
            config.api.failpoints_enabled = true;
            let block_size = (target_tps / 4) as u64;

            config.consensus.max_sending_block_txns = block_size;
            config.consensus.max_receiving_block_txns = block_size;
            config.consensus.round_initial_timeout_ms = 500;
            config.consensus.round_timeout_backoff_exponent_base = 1.0;
            config.consensus.quorum_store_poll_time_ms = 100;
            config.consensus.rand_rb_config.backoff_policy_max_delay_ms = 1000;

            let mut min_block_txns = block_size;
            let mut chain_health_backoff = ConsensusConfig::default().chain_health_backoff;
            if use_chain_backoff {
                // Generally if we are stress testing the consensus, we don't want to slow it down.
                chain_health_backoff = vec![];
            } else {
                for (i, item) in chain_health_backoff.iter_mut().enumerate() {
                    // as we have lower TPS, make limits smaller
                    item.max_sending_block_txns_after_filtering_override =
                        (block_size / 2_u64.pow(i as u32 + 1)).max(2);
                    min_block_txns =
                        min_block_txns.min(item.max_sending_block_txns_after_filtering_override);
                    // as we have fewer nodes, make backoff triggered earlier:
                    item.backoff_if_below_participating_voting_power_percentage = 90 - i * 5;
                }
            }
            config.consensus.quorum_store.sender_max_batch_txns = min_block_txns as usize;
            config.consensus.quorum_store.receiver_max_batch_txns = min_block_txns as usize;

            config.consensus.chain_health_backoff = chain_health_backoff;

            // Override the syncing mode of all nodes to use transaction output syncing.
            // TODO(joshlind): remove me once we move back to output syncing by default.
            if apply_txn_outputs {
                state_sync_config_apply_transaction_outputs(&mut config.state_sync);
            }
        }))
        .with_fullnode_override_node_config_fn(Arc::new(move |config, _| {
            // Override the syncing mode of all nodes to use transaction output syncing.
            // TODO(joshlind): remove me once we move back to output syncing by default.
            if apply_txn_outputs {
                state_sync_config_apply_transaction_outputs(&mut config.state_sync);
            }
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: target_tps })
                .transaction_mix(vec![
                    (TransactionTypeArg::CoinTransfer.materialize_default(), 80),
                    (
                        TransactionTypeArg::AccountGeneration.materialize_default(),
                        20,
                    ),
                ]),
        )
        .with_success_criteria({
            let success_criteria = SuccessCriteria::new(min_avg_tps)
                .add_no_restarts()
                .add_wait_for_catchup_s(30)
                .add_chain_progress({
                    let max_no_progress_secs = if max_down_nodes == 0 {
                        // very aggressive if no nodes are expected to be down
                        3.0
                    } else if max_down_nodes * 3 + 1 + 2 < num_validators {
                        // number of down nodes is at least 2 below the quorum limit, so
                        // we can still be reasonably aggqressive
                        15.0
                    } else {
                        // number of down nodes is close to the quorum limit, so
                        // make a check a bit looser, as state sync might be required
                        // to get the quorum back.
                        40.0
                    };
                    StateProgressThreshold {
                        max_non_epoch_no_progress_secs: max_no_progress_secs,
                        max_epoch_no_progress_secs: max_no_progress_secs,
                        max_non_epoch_round_gap: 60,
                        max_epoch_round_gap: 60,
                    }
                });

            // If errors are allowed, overwrite the success criteria
            if allow_errors {
                success_criteria.allow_errors()
            } else {
                success_criteria
            }
        })
}

fn large_db_test(
    num_validators: usize,
    target_tps: usize,
    min_avg_tps: usize,
    existing_db_tag: String,
) -> ForgeConfig {
    let config = ForgeConfig::default();
    config
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(std::cmp::max(2, target_tps / 1000))
        .add_network_test(PerformanceBenchmark)
        .with_existing_db(existing_db_tag.clone())
        .with_validator_override_node_config_fn(Arc::new(move |config, _| {
            config.base.working_dir = Some(PathBuf::from("/opt/aptos/data/checkpoint"));
        }))
        .with_fullnode_override_node_config_fn(Arc::new(move |config, _| {
            config.base.working_dir = Some(PathBuf::from("/opt/aptos/data/checkpoint"));
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: target_tps })
                .transaction_mix(vec![
                    (TransactionTypeArg::CoinTransfer.materialize_default(), 75),
                    (
                        TransactionTypeArg::AccountGeneration.materialize_default(),
                        20,
                    ),
                    (
                        TransactionTypeArg::TokenV1NFTMintAndTransferSequential
                            .materialize_default(),
                        5,
                    ),
                ]),
        )
        .with_success_criteria(
            SuccessCriteria::new(min_avg_tps)
                .add_no_restarts()
                .add_wait_for_catchup_s(30)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

fn quorum_store_reconfig_enable_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(20)
        .add_network_test(QuorumStoreOnChainEnableTest {})
        .with_success_criteria(
            SuccessCriteria::new(5000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_system_metrics_threshold(SYSTEM_12_CORES_10GB_THRESHOLD.clone())
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

fn mainnet_like_simulation_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 200_000,
                })
                .txn_expiration_time_secs(5 * 60),
        )
        .add_network_test(CompositeNetworkTest::new(
            MultiRegionNetworkEmulationTest::default(),
            CpuChaosTest::default(),
        ))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        // TODO(ibalajiarun): tune these success critiera after we have a better idea of the test behavior
        .with_success_criteria(
            SuccessCriteria::new(10000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

/// This test runs a network test in a real multi-region setup. It configures
/// genesis and node helm values to enable certain configurations needed to run in
/// the multiregion forge cluster.
fn multiregion_benchmark_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .add_network_test(PerformanceBenchmark)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // Have single epoch change in land blocking
            helm_values["chain"]["epoch_duration_secs"] = 300.into();

            helm_values["genesis"]["multicluster"]["enabled"] = true.into();
        }))
        .with_multi_region_config()
        .with_success_criteria(
            SuccessCriteria::new(4500)
                .add_no_restarts()
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup, give 10% of the run for longer durations.
                    180,
                )
                .add_system_metrics_threshold(SYSTEM_12_CORES_10GB_THRESHOLD.clone())
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

/// This test runs a constant-TPS benchmark where the network includes
/// PFNs, and the transactions are submitted to the PFNs. This is useful
/// for measuring latencies when the system is not saturated.
///
/// Note: If `add_cpu_chaos` is true, CPU chaos is enabled on the entire swarm.
/// Likewise, if `add_network_emulation` is true, network chaos is enabled.
fn pfn_const_tps(
    duration: Duration,
    add_cpu_chaos: bool,
    add_network_emulation: bool,
    epoch_changes: bool,
) -> ForgeConfig {
    let epoch_duration_secs = if epoch_changes {
        300 // 5 minutes
    } else {
        60 * 60 * 2 // 2 hours; avoid epoch changes which can introduce noise
    };

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .add_network_test(PFNPerformance::new(
            7,
            add_cpu_chaos,
            add_network_emulation,
            None,
        ))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = epoch_duration_secs.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(4500)
                .add_no_restarts()
                .add_max_expired_tps(0.0)
                .add_max_failed_submission_tps(0.0)
                // Percentile thresholds are estimated and should be revisited.
                .add_latency_threshold(3.5, LatencyType::P50)
                .add_latency_threshold(4.5, LatencyType::P90)
                .add_latency_threshold(5.5, LatencyType::P99)
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup and at most 10% of the run
                    (duration.as_secs() / 10).max(60),
                )
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

/// This test runs a performance benchmark where the network includes
/// PFNs, and the transactions are submitted to the PFNs. This is useful
/// for measuring maximum throughput and latencies.
///
/// Note: If `add_cpu_chaos` is true, CPU chaos is enabled on the entire swarm.
/// Likewise, if `add_network_emulation` is true, network chaos is enabled.
fn pfn_performance(
    duration: Duration,
    add_cpu_chaos: bool,
    add_network_emulation: bool,
    epoch_changes: bool,
    num_validators: usize,
    num_pfns: usize,
    broadcast_to_all_vfns: bool,
) -> ForgeConfig {
    // Determine the minimum expected TPS
    let min_expected_tps = if broadcast_to_all_vfns { 2500 } else { 4500 };
    let epoch_duration_secs = if epoch_changes {
        300 // 5 minutes
    } else {
        60 * 60 * 2 // 2 hours; avoid epoch changes which can introduce noise
    };

    let config_override_fn = if broadcast_to_all_vfns {
        let f: OverrideNodeConfigFn = Arc::new(move |pfn_config: &mut NodeConfig, _| {
            pfn_config.mempool.default_failovers = num_validators;
            for network in &mut pfn_config.full_node_networks {
                network.max_outbound_connections = num_validators;
            }
        });
        Some(f)
    } else {
        None
    };

    // Create the forge config
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_validators)
        .add_network_test(PFNPerformance::new(
            num_pfns as u64,
            add_cpu_chaos,
            add_network_emulation,
            config_override_fn,
        ))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = epoch_duration_secs.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(min_expected_tps)
                .add_no_restarts()
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup and at most 10% of the run
                    (duration.as_secs() / 10).max(60),
                )
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

/// A simple test that runs the swarm forever. This is useful for
/// local testing (e.g., deploying a local swarm and interacting
/// with it).
#[derive(Debug)]
struct RunForever;

impl Test for RunForever {
    fn name(&self) -> &'static str {
        "run_forever"
    }
}

#[async_trait::async_trait]
impl AptosTest for RunForever {
    async fn run<'t>(&self, _ctx: &mut AptosContext<'t>) -> Result<()> {
        println!("The network has been deployed. Hit Ctrl+C to kill this, otherwise it will run forever.");
        let keep_running = Arc::new(AtomicBool::new(true));
        while keep_running.load(Ordering::Acquire) {
            thread::park();
        }
        Ok(())
    }
}

//TODO Make public test later
#[derive(Debug)]
struct GetMetadata;

impl Test for GetMetadata {
    fn name(&self) -> &'static str {
        "get_metadata"
    }
}

impl AdminTest for GetMetadata {
    fn run(&self, ctx: &mut AdminContext<'_>) -> Result<()> {
        let client = ctx.rest_client();
        let runtime = Runtime::new().unwrap();
        runtime.block_on(client.get_aptos_version()).unwrap();
        runtime.block_on(client.get_ledger_information()).unwrap();

        Ok(())
    }
}

pub async fn check_account_balance(
    client: &RestClient,
    account_address: AccountAddress,
    expected: u64,
) -> Result<()> {
    let balance = client
        .get_account_balance(account_address)
        .await?
        .into_inner();
    assert_eq!(balance.get(), expected);

    Ok(())
}

#[derive(Debug)]
struct FundAccount;

impl Test for FundAccount {
    fn name(&self) -> &'static str {
        "fund_account"
    }
}

#[async_trait::async_trait]
impl AptosTest for FundAccount {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let client = ctx.client();

        let account = ctx.random_account();
        let amount = 1000;
        ctx.create_user_account(account.public_key()).await?;
        ctx.mint(account.address(), amount).await?;
        check_account_balance(&client, account.address(), amount).await?;

        Ok(())
    }
}

#[derive(Debug)]
struct TransferCoins;

impl Test for TransferCoins {
    fn name(&self) -> &'static str {
        "transfer_coins"
    }
}

#[async_trait::async_trait]
impl AptosTest for TransferCoins {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let client = ctx.client();
        let payer = ctx.random_account();
        let payee = ctx.random_account();
        ctx.create_user_account(payer.public_key()).await?;
        ctx.create_user_account(payee.public_key()).await?;
        ctx.mint(payer.address(), 10000).await?;
        check_account_balance(&client, payer.address(), 10000).await?;

        let transfer_txn = payer.sign_with_transaction_builder(
            ctx.aptos_transaction_factory()
                .payload(aptos_stdlib::aptos_coin_transfer(payee.address(), 10)),
        );
        client.submit_and_wait(&transfer_txn).await?;
        check_account_balance(&client, payee.address(), 10).await?;

        Ok(())
    }
}

#[derive(Debug)]
struct RestartValidator;

impl Test for RestartValidator {
    fn name(&self) -> &'static str {
        "restart_validator"
    }
}

#[async_trait]
impl NetworkTest for RestartValidator {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();
        let swarm = ctx.swarm.read().await;
        let node = swarm.validators().next().unwrap();
        node.health_check().await.expect("node health check failed");
        node.stop().await.unwrap();
        println!("Restarting node {}", node.peer_id());
        node.start().await.unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
        node.health_check().await.expect("node health check failed");
        Ok(())
    }
}

#[derive(Debug)]
struct EmitTransaction;

impl Test for EmitTransaction {
    fn name(&self) -> &'static str {
        "emit_transaction"
    }
}

#[async_trait]
impl NetworkTest for EmitTransaction {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();
        let duration = Duration::from_secs(10);
        let all_validators = ctx
            .swarm
            .read()
            .await
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let stats = generate_traffic(ctx, &all_validators, duration)
            .await
            .unwrap();
        ctx.report.report_txn_stats(self.name().to_string(), &stats);
        Ok(())
    }
}

#[derive(Debug)]
struct Delay {
    seconds: u64,
}

impl Delay {
    fn new(seconds: u64) -> Self {
        Self { seconds }
    }
}

impl Test for Delay {
    fn name(&self) -> &'static str {
        "delay"
    }
}

#[async_trait]
impl NetworkTest for Delay {
    async fn run<'a>(&self, _ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        info!("forge sleep {}", self.seconds);
        tokio::time::sleep(Duration::from_secs(self.seconds)).await;
        Ok(())
    }
}

#[derive(Debug)]
struct GatherMetrics;

impl Test for GatherMetrics {
    fn name(&self) -> &'static str {
        "gather_metrics"
    }
}

#[async_trait]
impl NetworkTest for GatherMetrics {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();
        gather_metrics_one(ctx).await;
        Ok(())
    }
}

async fn gather_metrics_one(ctx: &NetworkContext<'_>) {
    let handle = ctx.runtime.handle();
    let outdir = Path::new("/tmp");
    let mut gets = FuturesUnordered::new();
    let now = chrono::prelude::Utc::now()
        .format("%Y%m%d_%H%M%S")
        .to_string();
    {
        let swarm = ctx.swarm.read().await;
        for val in swarm.validators() {
            let mut url = val.inspection_service_endpoint();
            let valname = val.peer_id().to_string();
            url.set_path("metrics");
            let fname = format!("{}.{}.metrics", now, valname);
            let outpath: PathBuf = outdir.join(fname);
            let th = handle.spawn(gather_metrics_to_file(url, outpath));
            gets.push(th);
        }
    }
    // join all the join handles
    while !gets.is_empty() {
        select! {
            _ = gets.next() => {}
        }
    }
}

async fn gather_metrics_to_file(url: Url, outpath: PathBuf) {
    let client = reqwest::Client::new();
    match client.get(url).send().await {
        Ok(response) => {
            let url = response.url().clone();
            let status = response.status();
            if status.is_success() {
                match response.text().await {
                    Ok(text) => match std::fs::write(outpath, text) {
                        Ok(_) => {},
                        Err(err) => {
                            info!("could not write metrics: {}", err);
                        },
                    },
                    Err(err) => {
                        info!("bad metrics GET: {} -> {}", url, err);
                    },
                }
            } else {
                info!("bad metrics GET: {} -> {}", url, status);
            }
        },
        Err(err) => {
            info!("bad metrics GET: {}", err);
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_random_namespace() {
        let mut rng = rand::rngs::mock::StepRng::new(100, 1);
        let words = ["apple", "banana", "carrot", "durian", "eggplant", "fig"]
            .to_vec()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let namespace = random_namespace(words, &mut rng).unwrap();
        assert_eq!(namespace, "forge-durian-eggplant-fig-apple");
    }

    #[test]
    fn verify_tool() {
        use clap::CommandFactory;
        Args::command().debug_assert()
    }
}
