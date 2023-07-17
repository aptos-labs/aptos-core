// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Context, Result};
use aptos_config::config::{ChainHealthBackoffValues, ConsensusConfig, PipelineBackpressureValues};
use aptos_forge::{
    args::TransactionTypeArg,
    success_criteria::{LatencyType, StateProgressThreshold, SuccessCriteria},
    system_metrics::{MetricsThreshold, SystemMetricsThreshold},
    ForgeConfig, Options, *,
};
use aptos_logger::{info, Level};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{move_types::account_address::AccountAddress, transaction_builder::aptos_stdlib};
use aptos_testcases::{
    compatibility_test::SimpleValidatorUpgrade,
    consensus_reliability_tests::ChangingWorkingQuorumTest,
    forge_setup_test::ForgeSetupTest,
    framework_upgrade::FrameworkUpgrade,
    fullnode_reboot_stress_test::FullNodeRebootStressTest,
    generate_traffic,
    load_vs_perf_benchmark::{LoadVsPerfBenchmark, TransactionWorkload, Workloads},
    modifiers::{CpuChaosTest, ExecutionDelayConfig, ExecutionDelayTest},
    multi_region_network_test::MultiRegionNetworkEmulationTest,
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
use clap::{Parser, Subcommand};
use futures::stream::{FuturesUnordered, StreamExt};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use std::{
    env,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tokio::{runtime::Runtime, select};
use url::Url;

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
                    test_suite.get_success_criteria_mut().min_avg_tps = 400;
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

fn get_test_suite(
    suite_name: &str,
    duration: Duration,
    test_cmd: &TestCommand,
) -> Result<ForgeConfig> {
    match suite_name {
        "local_test_suite" => Ok(local_test_suite()),
        "pre_release" => Ok(pre_release_suite()),
        "run_forever" => Ok(run_forever()),
        // TODO(rustielin): verify each test suite
        "k8s_suite" => Ok(k8s_test_suite()),
        "chaos" => Ok(chaos_test_suite(duration)),
        single_test => single_test_suite(single_test, duration, test_cmd),
    }
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
        .add_network_test(SimpleValidatorUpgrade)
        .add_network_test(PerformanceBenchmark)
}

fn single_test_suite(
    test_name: &str,
    duration: Duration,
    test_cmd: &TestCommand,
) -> Result<ForgeConfig> {
    let single_test_suite = match test_name {
        // Land-blocking tests to be run on every PR:
        "land_blocking" => land_blocking_test_suite(duration), // to remove land_blocking, superseeded by the below
        "realistic_env_max_load" => realistic_env_max_load_test(duration, test_cmd, 7, 5),
        "compat" => compat(),
        "framework_upgrade" => framework_upgrade(),
        // Rest of the tests:
        "realistic_env_max_load_large" => realistic_env_max_load_test(duration, test_cmd, 20, 10),
        "realistic_env_load_sweep" => realistic_env_load_sweep_test(),
        "realistic_env_graceful_overload" => realistic_env_graceful_overload(),
        "realistic_network_tuned_for_throughput" => realistic_network_tuned_for_throughput_test(),
        "epoch_changer_performance" => epoch_changer_performance(),
        "state_sync_perf_fullnodes_apply_outputs" => state_sync_perf_fullnodes_apply_outputs(),
        "state_sync_perf_fullnodes_execute_transactions" => {
            state_sync_perf_fullnodes_execute_transactions()
        },
        "state_sync_perf_fullnodes_fast_sync" => state_sync_perf_fullnodes_fast_sync(),
        "state_sync_perf_validators" => state_sync_perf_validators(),
        "validators_join_and_leave" => validators_join_and_leave(),
        "config" => ForgeConfig::default().add_network_test(ReconfigurationTest),
        "network_partition" => network_partition(),
        "three_region_simulation" => three_region_simulation(),
        "three_region_simulation_with_different_node_speed" => {
            three_region_simulation_with_different_node_speed()
        },
        "network_bandwidth" => network_bandwidth(),
        "setup_test" => setup_test(),
        "single_vfn_perf" => single_vfn_perf(),
        "validator_reboot_stress_test" => validator_reboot_stress_test(),
        "fullnode_reboot_stress_test" => fullnode_reboot_stress_test(),
        "workload_mix" => workload_mix_test(),
        "account_creation" | "nft_mint" | "publishing" | "module_loading"
        | "write_new_resource" => individual_workload_tests(test_name.into()),
        "graceful_overload" => graceful_overload(),
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
        "consensus_only_perf_benchmark" => run_consensus_only_perf_test(),
        "consensus_only_three_region_simulation" => run_consensus_only_three_region_simulation(),
        "quorum_store_reconfig_enable_test" => quorum_store_reconfig_enable_test(),
        "mainnet_like_simulation_test" => mainnet_like_simulation_test(),
        "multiregion_benchmark_test" => multiregion_benchmark_test(),
        "pfn_const_tps" => pfn_const_tps(duration, false, false),
        "pfn_const_tps_with_network_chaos" => pfn_const_tps(duration, false, true),
        "pfn_const_tps_with_realistic_env" => pfn_const_tps(duration, true, true),
        "pfn_performance" => pfn_performance(duration, false, false),
        "pfn_performance_with_network_chaos" => pfn_performance(duration, false, true),
        "pfn_performance_with_realistic_env" => pfn_performance(duration, true, true),
        "gather_metrics" => gather_metrics(),
        _ => return Err(format_err!("Invalid --suite given: {:?}", test_name)),
    };
    Ok(single_test_suite)
}

fn wrap_with_realistic_env<T: NetworkTest + 'static>(test: T) -> CompositeNetworkTest {
    CompositeNetworkTest::new_with_two_wrappers(
        MultiRegionNetworkEmulationTest::default(),
        CpuChaosTest::default(),
        test,
    )
}

fn run_consensus_only_three_region_simulation() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 30000 })
                .txn_expiration_time_secs(5 * 60),
        )
        .add_network_test(ThreeRegionSameCloudSimulationTest)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            helm_values["validator"]["config"]["mempool"]["capacity"] = 3_000_000.into();
            helm_values["validator"]["config"]["mempool"]["capacity_bytes"] =
                (3_u64 * 1024 * 1024 * 1024).into();
            helm_values["validator"]["config"]["mempool"]["capacity_per_user"] = 100_000.into();
            helm_values["validator"]["config"]["mempool"]["system_transaction_timeout_secs"] =
                (5 * 60 * 60).into();
            helm_values["validator"]["config"]["mempool"]["system_transaction_gc_interval_ms"] =
                (5 * 60 * 60_000).into();
            helm_values["validator"]["config"]["consensus"]["max_sending_block_txns"] = 5000.into();
            helm_values["validator"]["config"]["consensus"]["max_receiving_block_txns"] =
                30000.into();
            helm_values["validator"]["config"]["consensus"]["max_sending_block_bytes"] =
                (3 * 1024 * 1024).into();
            helm_values["validator"]["config"]["state_sync"]["state_sync_driver"]
                ["bootstrapping_mode"] = "ExecuteTransactionsFromGenesis".into();
            helm_values["validator"]["config"]["state_sync"]["state_sync_driver"]
                ["continuous_syncing_mode"] = "ExecuteTransactions".into();
        }))
        // TODO(ibalajiarun): tune these success critiera after we have a better idea of the test behavior
        .with_success_criteria(
            SuccessCriteria::new(10000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 20.0,
                    max_round_gap: 6,
                }),
        )
}

fn run_consensus_only_perf_test() -> ForgeConfig {
    let config = ForgeConfig::default();
    let emit_job = config.get_emit_job().clone();
    config
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .add_network_test(LoadVsPerfBenchmark {
            test: Box::new(PerformanceBenchmark),
            workloads: Workloads::TPS(&[30000]),
            criteria: vec![],
        })
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_emit_job(emit_job.txn_expiration_time_secs(5 * 60))
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            helm_values["validator"]["config"]["mempool"]["capacity"] = 3_000_000.into();
            helm_values["validator"]["config"]["mempool"]["capacity_bytes"] =
                (3_u64 * 1024 * 1024 * 1024).into();
            helm_values["validator"]["config"]["mempool"]["capacity_per_user"] = 100_000.into();
            helm_values["validator"]["config"]["mempool"]["system_transaction_timeout_secs"] =
                (5 * 60 * 60).into();
            helm_values["validator"]["config"]["mempool"]["system_transaction_gc_interval_ms"] =
                (5 * 60 * 60_000).into();
            helm_values["validator"]["config"]["consensus"]["max_sending_block_txns"] =
                10000.into();
            helm_values["validator"]["config"]["consensus"]["max_receiving_block_txns"] =
                50000.into();
            helm_values["validator"]["config"]["consensus"]["max_sending_block_bytes"] =
                (3 * 1024 * 1024).into();
            helm_values["validator"]["config"]["state_sync"]["state_sync_driver"]
                ["bootstrapping_mode"] = "ExecuteTransactionsFromGenesis".into();
            helm_values["validator"]["config"]["state_sync"]["state_sync_driver"]
                ["continuous_syncing_mode"] = "ExecuteTransactions".into();
        }))
        .with_success_criteria(
            // TODO(ibalajiarun): tune these success critiera after we have a better idea of the test behavior
            SuccessCriteria::new(10000)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 30.0,
                    max_round_gap: 10,
                }),
        )
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
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12, 30),
                    // Check that we don't use more than 5 GB of memory for 30% of the time.
                    MetricsThreshold::new(5 * 1024 * 1024 * 1024, 30),
                ))
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 10.0,
                    max_round_gap: 4,
                }),
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
    changing_working_quorum_test_helper(7, 300, 3000, 2500, true, true, ChangingWorkingQuorumTest {
        min_tps: 750,
        always_healthy_nodes: 2,
        max_down_nodes: 0,
        num_large_validators: 2,
        add_execution_delay: true,
        check_period_s: 57,
    })
}

fn different_node_speed_and_reliability_test() -> ForgeConfig {
    changing_working_quorum_test_helper(20, 120, 70, 50, true, false, ChangingWorkingQuorumTest {
        min_tps: 30,
        always_healthy_nodes: 6,
        max_down_nodes: 5,
        num_large_validators: 3,
        add_execution_delay: true,
        check_period_s: 27,
    })
}

fn large_test_only_few_nodes_down() -> ForgeConfig {
    changing_working_quorum_test_helper(60, 120, 100, 70, false, false, ChangingWorkingQuorumTest {
        min_tps: 50,
        always_healthy_nodes: 40,
        max_down_nodes: 10,
        num_large_validators: 0,
        add_execution_delay: false,
        check_period_s: 27,
    })
}

fn changing_working_quorum_test_high_load() -> ForgeConfig {
    changing_working_quorum_test_helper(16, 120, 500, 300, true, true, ChangingWorkingQuorumTest {
        min_tps: 50,
        always_healthy_nodes: 0,
        max_down_nodes: 16,
        num_large_validators: 0,
        add_execution_delay: false,
        // Use longer check duration, as we are bringing enough nodes
        // to require state-sync to catch up to have consensus.
        check_period_s: 53,
    })
}

fn changing_working_quorum_test() -> ForgeConfig {
    changing_working_quorum_test_helper(16, 120, 100, 70, true, true, ChangingWorkingQuorumTest {
        min_tps: 15,
        always_healthy_nodes: 0,
        max_down_nodes: 16,
        num_large_validators: 0,
        add_execution_delay: false,
        // Use longer check duration, as we are bringing enough nodes
        // to require state-sync to catch up to have consensus.
        check_period_s: 53,
    })
}

fn consensus_stress_test() -> ForgeConfig {
    changing_working_quorum_test_helper(10, 60, 100, 80, true, false, ChangingWorkingQuorumTest {
        min_tps: 50,
        always_healthy_nodes: 10,
        max_down_nodes: 0,
        num_large_validators: 0,
        add_execution_delay: false,
        check_period_s: 27,
    })
}

fn realistic_env_load_sweep_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(10)
        .add_network_test(wrap_with_realistic_env(LoadVsPerfBenchmark {
            test: Box::new(PerformanceBenchmark),
            workloads: Workloads::TPS(&[10, 100, 1000, 3000, 5000]),
            criteria: [
                (9, 1.5, 3., 4.),
                (95, 1.5, 3., 4.),
                (950, 2., 3., 4.),
                (2750, 2.5, 3.5, 4.5),
                (4600, 3., 4., 5.),
            ]
            .into_iter()
            .map(|(min_tps, max_lat_p50, max_lat_p90, max_lat_p99)| {
                SuccessCriteria::new(min_tps)
                    .add_max_expired_tps(0)
                    .add_max_failed_submission_tps(0)
                    .add_latency_threshold(max_lat_p50, LatencyType::P50)
                    .add_latency_threshold(max_lat_p90, LatencyType::P90)
                    .add_latency_threshold(max_lat_p99, LatencyType::P99)
            })
            .collect(),
        }))
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
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 30.0,
                    max_round_gap: 10,
                }),
        )
}

fn load_vs_perf_benchmark() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(10)
        .add_network_test(LoadVsPerfBenchmark {
            test: Box::new(PerformanceBenchmark),
            workloads: Workloads::TPS(&[
                200, 1000, 3000, 5000, 7000, 7500, 8000, 9000, 10000, 12000, 15000,
            ]),
            criteria: Vec::new(),
        })
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(0)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 30.0,
                    max_round_gap: 10,
                }),
        )
}

fn workload_vs_perf_benchmark() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["config"]["execution"]
                ["processed_transactions_detailed_counters"] = true.into();
        }))
        // .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
        //     mempool_backlog: 10000,
        // }))
        .add_network_test(LoadVsPerfBenchmark {
            test: Box::new(PerformanceBenchmark),
            workloads: Workloads::TRANSACTIONS(&[
                TransactionWorkload {
                    transaction_type: TransactionTypeArg::NoOp,
                    num_modules: 1,
                    unique_senders: false,
                },
                TransactionWorkload {
                    transaction_type: TransactionTypeArg::NoOp,
                    num_modules: 1,
                    unique_senders: true,
                },
                TransactionWorkload {
                    transaction_type: TransactionTypeArg::NoOp,
                    num_modules: 1000,
                    unique_senders: false,
                },
                TransactionWorkload {
                    transaction_type: TransactionTypeArg::CoinTransfer,
                    num_modules: 1,
                    unique_senders: true,
                },
                TransactionWorkload {
                    transaction_type: TransactionTypeArg::CoinTransfer,
                    num_modules: 1,
                    unique_senders: true,
                },
                TransactionWorkload {
                    transaction_type: TransactionTypeArg::AccountResource32B,
                    num_modules: 1,
                    unique_senders: true,
                },
                TransactionWorkload {
                    transaction_type: TransactionTypeArg::AccountResource1KB,
                    num_modules: 1,
                    unique_senders: true,
                },
                TransactionWorkload {
                    transaction_type: TransactionTypeArg::PublishPackage,
                    num_modules: 1,
                    unique_senders: true,
                },
            ]),
            criteria: Vec::new(),
        })
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(0)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 30.0,
                    max_round_gap: 10,
                }),
        )
}

fn graceful_overload() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
        // if we have full nodes for subset of validators, TPS drops.
        // Validators without VFN are not creating batches,
        // as no useful transaction reach their mempool.
        // something to potentially improve upon.
        // So having VFNs for all validators
        .with_initial_fullnode_count(10)
        .add_network_test(TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 10000 })
                .init_gas_price_multiplier(20),

            // Additionally - we are not really gracefully handling overlaods,
            // setting limits based on current reality, to make sure they
            // don't regress, but something to investigate
            inner_success_criteria: SuccessCriteria::new(3400),
        })
        // First start non-overload (higher gas-fee) traffic,
        // to not cause issues with TxnEmitter setup - account creation
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 1000 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE),
        )
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(900)
                .add_no_restarts()
                .add_wait_for_catchup_s(120)
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12, 40),
                    // Check that we don't use more than 5 GB of memory for 30% of the time.
                    MetricsThreshold::new(5 * 1024 * 1024 * 1024, 30),
                ))
                .add_latency_threshold(10.0, LatencyType::P50)
                .add_latency_threshold(30.0, LatencyType::P90)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 30.0,
                    max_round_gap: 10,
                }),
        )
}

fn realistic_env_graceful_overload() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        // if we have full nodes for subset of validators, TPS drops.
        // Validators without VFN are not creating batches,
        // as no useful transaction reach their mempool.
        // something to potentially improve upon.
        // So having VFNs for all validators
        .with_initial_fullnode_count(20)
        .add_network_test(wrap_with_realistic_env(TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 15000 })
                .init_gas_price_multiplier(20),
            // Additionally - we are not really gracefully handling overlaods,
            // setting limits based on current reality, to make sure they
            // don't regress, but something to investigate
            inner_success_criteria: SuccessCriteria::new(3400),
        }))
        // First start higher gas-fee traffic, to not cause issues with TxnEmitter setup - account creation
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 1000 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE),
        )
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["config"]["execution"]
                ["processed_transactions_detailed_counters"] = true.into();
        }))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(900)
                .add_no_restarts()
                .add_wait_for_catchup_s(120)
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // overload test uses more CPUs than others, so increase the limit
                    // Check that we don't use more than 18 CPU cores for 30% of the time.
                    MetricsThreshold::new(18, 40),
                    // Check that we don't use more than 5 GB of memory for 30% of the time.
                    MetricsThreshold::new(5 * 1024 * 1024 * 1024, 30),
                ))
                .add_latency_threshold(10.0, LatencyType::P50)
                .add_latency_threshold(30.0, LatencyType::P90)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 30.0,
                    max_round_gap: 10,
                }),
        )
}

fn workload_mix_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_initial_fullnode_count(3)
        .add_network_test(PerformanceBenchmark)
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["config"]["execution"]
                ["processed_transactions_detailed_counters"] = true.into();
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 10000,
                })
                .transaction_mix(vec![
                    (
                        TransactionTypeArg::AccountGeneration.materialize_default(),
                        5,
                    ),
                    (TransactionTypeArg::NoOp5Signers.materialize_default(), 1),
                    (TransactionTypeArg::CoinTransfer.materialize_default(), 1),
                    (TransactionTypeArg::PublishPackage.materialize_default(), 1),
                    (
                        TransactionTypeArg::AccountResource32B.materialize(1, true),
                        1,
                    ),
                    // (
                    //     TransactionTypeArg::AccountResource10KB.materialize(1, true),
                    //     1,
                    // ),
                    (
                        TransactionTypeArg::ModifyGlobalResource.materialize(1, false),
                        1,
                    ),
                    // (
                    //     TransactionTypeArg::ModifyGlobalResource.materialize(10, false),
                    //     1,
                    // ),
                    (
                        TransactionTypeArg::Batch100Transfer.materialize_default(),
                        1,
                    ),
                    // (
                    //     TransactionTypeArg::TokenV1NFTMintAndTransferSequential
                    //         .materialize_default(),
                    //     1,
                    // ),
                    // (
                    //     TransactionTypeArg::TokenV1NFTMintAndTransferParallel.materialize_default(),
                    //     1,
                    // ),
                    // (
                    //     TransactionTypeArg::TokenV1FTMintAndTransfer.materialize_default(),
                    //     1,
                    // ),
                    (
                        TransactionTypeArg::TokenV2AmbassadorMint.materialize_default(),
                        1,
                    ),
                ]),
        )
        .with_success_criteria(
            SuccessCriteria::new(100)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 20.0,
                    max_round_gap: 6,
                }),
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
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["config"]["execution"]
                ["processed_transactions_detailed_counters"] = true.into();
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
                    "module_loading" => TransactionTypeArg::NoOp.materialize(1000, false),
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
            .add_chain_progress(StateProgressThreshold {
                max_no_progress_secs: 20.0,
                max_round_gap: 6,
            }),
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

fn apply_quorum_store_configs_for_single_node(helm_values: &mut serde_yaml::Value) {
    helm_values["validator"]["config"]["consensus"]["quorum_store"]["back_pressure"]
        ["dynamic_max_txn_per_s"] = 5500.into();
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
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            apply_quorum_store_configs_for_single_node(helm_values);
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
        .add_network_test(Delay::new(60))
        .add_network_test(GatherMetrics)
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
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["config"]["api"]["failpoints_enabled"] = true.into();
            // helm_values["validator"]["config"]["consensus"]["max_sending_block_txns"] =
            //     4000.into();
            // helm_values["validator"]["config"]["consensus"]["max_sending_block_bytes"] =
            //     1000000.into();
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["bootstrapping_mode"] = "ExecuteTransactionsFromGenesis".into();
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["continuous_syncing_mode"] = "ExecuteTransactions".into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(1000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 20.0,
                    max_round_gap: 6,
                }),
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
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 20.0,
                    max_round_gap: 6,
                }),
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
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            apply_quorum_store_configs_for_single_node(helm_values);
        }))
}

fn compat() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(SimpleValidatorUpgrade)
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 30.into();
        }))
}

fn framework_upgrade() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(FrameworkUpgrade)
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 30.into();
        }))
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
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["bootstrapping_mode"] = "ApplyTransactionOutputsFromGenesis".into();
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["continuous_syncing_mode"] = "ApplyTransactionOutputs".into();
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
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["bootstrapping_mode"] = "ExecuteTransactionsFromGenesis".into();
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["continuous_syncing_mode"] = "ExecuteTransactions".into();
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
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["bootstrapping_mode"] = "DownloadLatestStates".into();
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["continuous_syncing_mode"] = "ApplyTransactionOutputs".into();
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
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            helm_values["validator"]["config"]["state_sync"]["state_sync_driver"]
                ["bootstrapping_mode"] = "ApplyTransactionOutputsFromGenesis".into();
            helm_values["validator"]["config"]["state_sync"]["state_sync_driver"]
                ["continuous_syncing_mode"] = "ApplyTransactionOutputs".into();
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
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12, 30),
                    // Check that we don't use more than 10 GB of memory for 30% of the time.
                    MetricsThreshold::new(10 * 1024 * 1024 * 1024, 30),
                ))
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 10.0,
                    max_round_gap: 4,
                }),
        )
}

fn land_blocking_test_suite(duration: Duration) -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(10)
        .add_network_test(PerformanceBenchmark)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // Have single epoch change in land blocking
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(
                if duration.as_secs() > 1200 {
                    4500
                } else {
                    5000
                },
            )
            .add_no_restarts()
            .add_wait_for_catchup_s(
                // Give at least 60s for catchup, give 10% of the run for longer durations.
                (duration.as_secs() / 10).max(60),
            )
            .add_system_metrics_threshold(SystemMetricsThreshold::new(
                // Check that we don't use more than 12 CPU cores for 30% of the time.
                MetricsThreshold::new(12, 30),
                // Check that we don't use more than 10 GB of memory for 30% of the time.
                MetricsThreshold::new(10 * 1024 * 1024 * 1024, 30),
            ))
            .add_chain_progress(StateProgressThreshold {
                max_no_progress_secs: 10.0,
                max_round_gap: 4,
            }),
        )
}

// TODO: Replace land_blocking when performance reaches on par with current land_blocking
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

    // Calculate the max CPU threshold
    let max_cpu_threshold = if num_validators >= 10 { 30 } else { 70 };

    // Create the test
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .add_network_test(wrap_with_realistic_env(TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 40000,
                })
                .init_gas_price_multiplier(20),
            inner_success_criteria: SuccessCriteria::new(
                if ha_proxy {
                    4700
                } else if long_running {
                    5500
                } else {
                    5000
                },
            ),
        }))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            // Have single epoch change in land blocking, and a few on long-running
            helm_values["chain"]["epoch_duration_secs"] =
                (if long_running { 600 } else { 300 }).into();
        }))
        // First start higher gas-fee traffic, to not cause issues with TxnEmitter setup - account creation
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(
            SuccessCriteria::new(95)
                .add_no_restarts()
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup, give 10% of the run for longer durations.
                    (duration.as_secs() / 10).max(60),
                )
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12, max_cpu_threshold),
                    // Check that we don't use more than 10 GB of memory for 30% of the time.
                    MetricsThreshold::new(10 * 1024 * 1024 * 1024, 30),
                ))
                .add_latency_threshold(3.0, LatencyType::P50)
                .add_latency_threshold(5.0, LatencyType::P90)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 10.0,
                    max_round_gap: 4,
                }),
        )
}

fn realistic_network_tuned_for_throughput_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(12).unwrap())
        // if we have full nodes for subset of validators, TPS drops.
        // Validators without VFN are not creating batches,
        // as no useful transaction reach their mempool.
        // something to potentially improve upon.
        // So having VFNs for all validators
        .with_initial_fullnode_count(12)
        .add_network_test(MultiRegionNetworkEmulationTest::default())
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
            mempool_backlog: 150000,
        }))
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["config"]["consensus"]
                ["max_sending_block_txns_quorum_store_override"] = 10000.into();
            helm_values["validator"]["config"]["consensus"]["pipeline_backpressure"] =
                serde_yaml::to_value(Vec::<PipelineBackpressureValues>::new()).unwrap();
            helm_values["validator"]["config"]["consensus"]["chain_health_backoff"] =
                serde_yaml::to_value(Vec::<ChainHealthBackoffValues>::new()).unwrap();

            helm_values["validator"]["config"]["consensus"]
                ["wait_for_full_blocks_above_recent_fill_threshold"] = (0.8).into();
            helm_values["validator"]["config"]["consensus"]
                ["wait_for_full_blocks_above_pending_blocks"] = 8.into();

            helm_values["validator"]["config"]["consensus"]["quorum_store"]["back_pressure"]
                ["backlog_txn_limit_count"] = 100000.into();
            helm_values["validator"]["config"]["consensus"]["quorum_store"]["back_pressure"]
                ["backlog_per_validator_batch_limit_count"] = 10.into();

            helm_values["validator"]["config"]["consensus"]["quorum_store"]["back_pressure"]
                ["dynamic_max_txn_per_s"] = 6000.into();

            // Experimental storage optimizations
            helm_values["validator"]["config"]["storage"]["rocksdb_configs"]["split_ledger_db"] =
                true.into();
            helm_values["validator"]["config"]["storage"]["rocksdb_configs"]
                ["use_sharded_state_merkle_db"] = true.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(8000)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // Tuned for throughput uses more cores than regular tests,
                    // as it achieves higher throughput.
                    // Check that we don't use more than 14 CPU cores for 30% of the time.
                    MetricsThreshold::new(14, 30),
                    // Check that we don't use more than 10 GB of memory for 30% of the time.
                    MetricsThreshold::new(10 * 1024 * 1024 * 1024, 30),
                ))
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 10.0,
                    max_round_gap: 4,
                }),
        )
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
            .add_system_metrics_threshold(SystemMetricsThreshold::new(
                // Check that we don't use more than 12 CPU cores for 30% of the time.
                MetricsThreshold::new(12, 30),
                // Check that we don't use more than 5 GB of memory for 30% of the time.
                MetricsThreshold::new(5 * 1024 * 1024 * 1024, 30),
            )),
        )
}

fn changing_working_quorum_test_helper(
    num_validators: usize,
    epoch_duration: usize,
    target_tps: usize,
    min_avg_tps: usize,
    apply_txn_outputs: bool,
    use_chain_backoff: bool,
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
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["config"]["api"]["failpoints_enabled"] = true.into();
            let block_size = (target_tps / 4) as u64;
            helm_values["validator"]["config"]["consensus"]["max_sending_block_txns"] =
                block_size.into();
            helm_values["validator"]["config"]["consensus"]
                ["max_sending_block_txns_quorum_store_override"] = block_size.into();
            helm_values["validator"]["config"]["consensus"]
                ["max_receiving_block_txns_quorum_store_override"] = block_size.into();
            helm_values["validator"]["config"]["consensus"]["round_initial_timeout_ms"] =
                500.into();
            helm_values["validator"]["config"]["consensus"]
                ["round_timeout_backoff_exponent_base"] = 1.0.into();
            helm_values["validator"]["config"]["consensus"]["quorum_store_poll_time_ms"] =
                100.into();

            let mut min_block_txns = block_size;
            let mut chain_health_backoff = ConsensusConfig::default().chain_health_backoff;
            if use_chain_backoff {
                // Generally if we are stress testing the consensus, we don't want to slow it down.
                chain_health_backoff = vec![];
            } else {
                for (i, item) in chain_health_backoff.iter_mut().enumerate() {
                    // as we have lower TPS, make limits smaller
                    item.max_sending_block_txns_override =
                        (block_size / 2_u64.pow(i as u32 + 1)).max(2);
                    min_block_txns = min_block_txns.min(item.max_sending_block_txns_override);
                    // as we have fewer nodes, make backoff triggered earlier:
                    item.backoff_if_below_participating_voting_power_percentage = 90 - i * 5;
                }
            }
            helm_values["validator"]["config"]["consensus"]["quorum_store"]
                ["sender_max_batch_txns"] = min_block_txns.into();
            helm_values["validator"]["config"]["consensus"]["quorum_store"]
                ["receiver_max_batch_txns"] = min_block_txns.into();

            helm_values["validator"]["config"]["consensus"]["chain_health_backoff"] =
                serde_yaml::to_value(chain_health_backoff).unwrap();

            // Override the syncing mode of all nodes to use transaction output syncing.
            // TODO(joshlind): remove me once we move back to output syncing by default.
            if apply_txn_outputs {
                helm_values["validator"]["config"]["state_sync"]["state_sync_driver"]
                    ["bootstrapping_mode"] = "ApplyTransactionOutputsFromGenesis".into();
                helm_values["validator"]["config"]["state_sync"]["state_sync_driver"]
                    ["continuous_syncing_mode"] = "ApplyTransactionOutputs".into();

                helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                    ["bootstrapping_mode"] = "ApplyTransactionOutputsFromGenesis".into();
                helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                    ["continuous_syncing_mode"] = "ApplyTransactionOutputs".into();
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
        .with_success_criteria(
            SuccessCriteria::new(min_avg_tps)
                .add_no_restarts()
                .add_wait_for_catchup_s(30)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: if max_down_nodes == 0 {
                        // very aggressive if no nodes are expected to be down
                        3.0
                    } else if max_down_nodes * 3 + 1 + 2 < num_validators {
                        // number of down nodes is at least 2 below the quorum limit, so
                        // we can still be reasonably aggressive
                        15.0
                    } else {
                        // number of down nodes is close to the quorum limit, so
                        // make a check a bit looser, as state sync might be required
                        // to get the quorum back.
                        30.0
                    },
                    max_round_gap: 6,
                }),
        )
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
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["storage"]["labels"]["tag"] = existing_db_tag.clone().into();
            helm_values["fullnode"]["storage"]["labels"]["tag"] = existing_db_tag.clone().into();
            helm_values["validator"]["config"]["base"]["working_dir"] =
                "/opt/aptos/data/checkpoint".into();
            helm_values["fullnode"]["config"]["base"]["working_dir"] =
                "/opt/aptos/data/checkpoint".into();
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
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 20.0,
                    max_round_gap: 6,
                }),
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
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12, 30),
                    // Check that we don't use more than 10 GB of memory for 30% of the time.
                    MetricsThreshold::new(10 * 1024 * 1024 * 1024, 30),
                ))
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 10.0,
                    max_round_gap: 4,
                }),
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
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 20.0,
                    max_round_gap: 6,
                }),
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
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            helm_values["multicluster"]["enabled"] = true.into();
            // Create headless services for validators and fullnodes.
            // Note: chaos-mesh will not work with clusterIP services.
            helm_values["service"]["validator"]["internal"]["type"] = "ClusterIP".into();
            helm_values["service"]["validator"]["internal"]["headless"] = true.into();
            helm_values["service"]["fullnode"]["internal"]["type"] = "ClusterIP".into();
            helm_values["service"]["fullnode"]["internal"]["headless"] = true.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(4500)
                .add_no_restarts()
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup, give 10% of the run for longer durations.
                    180,
                )
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12, 30),
                    // Check that we don't use more than 10 GB of memory for 30% of the time.
                    MetricsThreshold::new(10 * 1024 * 1024 * 1024, 30),
                ))
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 10.0,
                    max_round_gap: 4,
                }),
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
) -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 100 }))
        .add_network_test(PFNPerformance::new(add_cpu_chaos, add_network_emulation))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // Require frequent epoch changes
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(50)
                .add_no_restarts()
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup and at most 10% of the run
                    (duration.as_secs() / 10).max(60),
                )
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 10.0,
                    max_round_gap: 4,
                }),
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
) -> ForgeConfig {
    // Determine the minimum expected TPS
    let min_expected_tps = if add_cpu_chaos {
        3000
    } else if add_network_emulation {
        4000
    } else {
        4500
    };

    // Create the forge config
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .add_network_test(PFNPerformance::new(add_cpu_chaos, add_network_emulation))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // Require frequent epoch changes
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(min_expected_tps)
                .add_no_restarts()
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup and at most 10% of the run
                    (duration.as_secs() / 10).max(60),
                )
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 10.0,
                    max_round_gap: 4,
                }),
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
        let mut payer = ctx.random_account();
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

impl NetworkTest for RestartValidator {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        let runtime = Runtime::new()?;
        runtime.block_on(async {
            let node = ctx.swarm().validators_mut().next().unwrap();
            node.health_check().await.expect("node health check failed");
            node.stop().await.unwrap();
            println!("Restarting node {}", node.peer_id());
            node.start().await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            node.health_check().await.expect("node health check failed");
        });
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

impl NetworkTest for EmitTransaction {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        let duration = Duration::from_secs(10);
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let stats = generate_traffic(ctx, &all_validators, duration).unwrap();
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

impl NetworkTest for Delay {
    fn run(&self, _ctx: &mut NetworkContext<'_>) -> Result<()> {
        info!("forge sleep {}", self.seconds);
        std::thread::sleep(Duration::from_secs(self.seconds));
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

impl NetworkTest for GatherMetrics {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        let runtime = ctx.runtime.handle();
        runtime.block_on(gather_metrics_one(ctx));
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
    for val in ctx.swarm.validators() {
        let mut url = val.inspection_service_endpoint();
        let valname = val.peer_id().to_string();
        url.set_path("metrics");
        let fname = format!("{}.{}.metrics", now, valname);
        let outpath: PathBuf = outdir.join(fname);
        let th = handle.spawn(gather_metrics_to_file(url, outpath));
        gets.push(th);
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
