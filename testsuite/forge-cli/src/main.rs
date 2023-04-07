// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Context, Result};
use aptos_config::config::ConsensusConfig;
use aptos_forge::{
    success_criteria::{LatencyType, StateProgressThreshold, SuccessCriteria},
    system_metrics::{MetricsThreshold, SystemMetricsThreshold},
    ForgeConfig, Options, *,
};
use aptos_logger::Level;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{move_types::account_address::AccountAddress, transaction_builder::aptos_stdlib};
use aptos_testcases::{
    compatibility_test::SimpleValidatorUpgrade,
    consensus_reliability_tests::ChangingWorkingQuorumTest,
    forge_setup_test::ForgeSetupTest,
    framework_upgrade::FrameworkUpgrade,
    fullnode_reboot_stress_test::FullNodeRebootStressTest,
    generate_traffic,
    load_vs_perf_benchmark::{LoadVsPerfBenchmark, TransactinWorkload, Workloads},
    modifiers::{ExecutionDelayConfig, ExecutionDelayTest},
    multi_region_simulation_test::MultiRegionMultiCloudSimulationTest,
    network_bandwidth_test::NetworkBandwidthTest,
    network_loss_test::NetworkLossTest,
    network_partition_test::NetworkPartitionTest,
    performance_test::PerformanceBenchmark,
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
use std::{
    env,
    num::NonZeroUsize,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;
use url::Url;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(long, default_value = "300")]
    duration_secs: usize,
    #[structopt(flatten)]
    options: Options,
    #[structopt(long)]
    num_validators: Option<usize>,
    #[structopt(long)]
    num_validator_fullnodes: Option<usize>,
    #[structopt(
        long,
        help = "Specify a test suite to run",
        default_value = "land_blocking"
    )]
    suite: String,
    #[structopt(long, multiple = true)]
    changelog: Option<Vec<String>>,

    // subcommand groups
    #[structopt(flatten)]
    cli_cmd: CliCommand,
}

#[derive(StructOpt, Debug)]
enum CliCommand {
    Test(TestCommand),
    Operator(OperatorCommand),
}

#[derive(StructOpt, Debug)]
enum TestCommand {
    LocalSwarm(LocalSwarm),
    K8sSwarm(K8sSwarm),
}

#[derive(StructOpt, Debug)]
enum OperatorCommand {
    SetNodeImageTag(SetNodeImageTag),
    CleanUp(CleanUp),
    Resize(Resize),
}

#[derive(StructOpt, Debug)]
struct LocalSwarm {}

#[derive(StructOpt, Debug)]
struct K8sSwarm {
    #[structopt(long, help = "The kubernetes namespace to use for test")]
    namespace: String,
    #[structopt(
        long,
        help = "The image tag currently is used for validators",
        default_value = "devnet"
    )]
    image_tag: String,
    #[structopt(
        long,
        help = "For supported tests, the image tag for validators to upgrade to",
        default_value = "devnet"
    )]
    upgrade_image_tag: String,
    #[structopt(
        long,
        help = "Path to flattened directory containing compiled Move modules"
    )]
    move_modules_dir: Option<String>,
    #[structopt(
        long,
        help = "If set, uses kubectl port-forward instead of assuming k8s DNS access"
    )]
    port_forward: bool,
    #[structopt(
        long,
        help = "If set, reuse the forge testnet active in the specified namespace"
    )]
    reuse: bool,
    #[structopt(
        long,
        help = "If set, keeps the forge testnet active in the specified namespace"
    )]
    keep: bool,
    #[structopt(long, help = "If set, enables HAProxy for each of the validators")]
    enable_haproxy: bool,
}

#[derive(StructOpt, Debug)]
struct SetNodeImageTag {
    #[structopt(long, help = "The name of the node StatefulSet to update")]
    stateful_set_name: String,
    #[structopt(long, help = "The name of the container to update")]
    container_name: String,
    #[structopt(long, help = "The docker image tag to use for the node")]
    image_tag: String,
    #[structopt(long, help = "The kubernetes namespace to clean up")]
    namespace: String,
}

#[derive(StructOpt, Debug)]
struct CleanUp {
    #[structopt(
        long,
        help = "The kubernetes namespace to clean up. If unset, attemps to cleanup all by using forge-management configmaps"
    )]
    namespace: Option<String>,
}

#[derive(StructOpt, Debug)]
struct Resize {
    #[structopt(long, help = "The kubernetes namespace to resize")]
    namespace: String,
    #[structopt(long, default_value = "30")]
    num_validators: usize,
    #[structopt(long, default_value = "1")]
    num_fullnodes: usize,
    #[structopt(
        long,
        help = "Override the image tag used for validators",
        default_value = "devnet"
    )]
    validator_image_tag: String,
    #[structopt(
        long,
        help = "Override the image tag used for testnet-specific components",
        default_value = "devnet"
    )]
    testnet_image_tag: String,
    #[structopt(
        long,
        help = "Path to flattened directory containing compiled Move modules"
    )]
    move_modules_dir: Option<String>,
    #[structopt(
        long,
        help = "If set, dont use kubectl port forward to access the cluster"
    )]
    connect_directly: bool,
    #[structopt(long, help = "If set, enables HAProxy for each of the validators")]
    enable_haproxy: bool,
}

fn main() -> Result<()> {
    let mut logger = aptos_logger::Logger::new();
    logger.channel_size(1000).is_async(false).level(Level::Info);
    logger.build();

    let args = Args::from_args();
    let duration = Duration::from_secs(args.duration_secs as u64);
    let suite_name: &str = args.suite.as_ref();

    let runtime = Runtime::new()?;
    match args.cli_cmd {
        // cmd input for test
        CliCommand::Test(ref test_cmd) => {
            // Identify the test suite to run
            let mut test_suite = get_test_suite(suite_name, duration)?;

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
                TestCommand::LocalSwarm(..) => {
                    // Loosen all criteria for local runs
                    test_suite.get_success_criteria_mut().avg_tps = 400;
                    let previous_emit_job = test_suite.get_emit_job().clone();
                    let test_suite =
                        test_suite.with_emit_job(previous_emit_job.mode(EmitJobMode::MaxLoad {
                            mempool_backlog: 5000,
                        }));

                    run_forge(
                        duration,
                        test_suite,
                        LocalFactory::from_workspace()?,
                        &args.options,
                        args.changelog.clone(),
                    )
                },
                TestCommand::K8sSwarm(k8s) => {
                    if let Some(move_modules_dir) = &k8s.move_modules_dir {
                        test_suite = test_suite.with_genesis_modules_path(move_modules_dir.clone());
                    }
                    run_forge(
                        duration,
                        test_suite,
                        K8sFactory::new(
                            k8s.namespace.clone(),
                            k8s.image_tag.clone(),
                            k8s.upgrade_image_tag.clone(),
                            k8s.port_forward,
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
    tests: ForgeConfig<'_>,
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

fn get_test_suite(suite_name: &str, duration: Duration) -> Result<ForgeConfig<'static>> {
    match suite_name {
        "land_blocking" => Ok(land_blocking_test_suite(duration)),
        "local_test_suite" => Ok(local_test_suite()),
        "pre_release" => Ok(pre_release_suite()),
        "run_forever" => Ok(run_forever()),
        // TODO(rustielin): verify each test suite
        "k8s_suite" => Ok(k8s_test_suite()),
        "chaos" => Ok(chaos_test_suite(duration)),
        single_test => single_test_suite(single_test),
    }
}

/// Provides a forge config that runs the swarm forever (unless killed)
fn run_forever() -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_admin_tests(vec![&GetMetadata])
        .with_genesis_module_bundle(aptos_cached_packages::head_release_bundle().clone())
        .with_aptos_tests(vec![&RunForever])
}

fn local_test_suite() -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_aptos_tests(vec![&FundAccount, &TransferCoins])
        .with_admin_tests(vec![&GetMetadata])
        .with_network_tests(vec![&RestartValidator, &EmitTransaction])
        .with_genesis_module_bundle(aptos_cached_packages::head_release_bundle().clone())
}

fn k8s_test_suite() -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .with_aptos_tests(vec![&FundAccount, &TransferCoins])
        .with_admin_tests(vec![&GetMetadata])
        .with_network_tests(vec![
            &EmitTransaction,
            &SimpleValidatorUpgrade,
            &PerformanceBenchmark,
        ])
}

fn single_test_suite(test_name: &str) -> Result<ForgeConfig<'static>> {
    let config =
        ForgeConfig::default().with_initial_validator_count(NonZeroUsize::new(30).unwrap());
    let single_test_suite = match test_name {
        "epoch_changer_performance" => epoch_changer_performance(config),
        "state_sync_perf_fullnodes_apply_outputs" => {
            state_sync_perf_fullnodes_apply_outputs(config)
        },
        "state_sync_perf_fullnodes_execute_transactions" => {
            state_sync_perf_fullnodes_execute_transactions(config)
        },
        "state_sync_perf_fullnodes_fast_sync" => state_sync_perf_fullnodes_fast_sync(config),
        "state_sync_perf_validators" => state_sync_perf_validators(config),
        "validators_join_and_leave" => validators_join_and_leave(config),
        "compat" => compat(config),
        "framework_upgrade" => upgrade(config),
        "config" => config.with_network_tests(vec![&ReconfigurationTest]),
        "network_partition" => network_partition(config),
        "three_region_simulation" => three_region_simulation(config),
        "three_region_simulation_with_different_node_speed" => {
            three_region_simulation_with_different_node_speed(config)
        },
        "network_bandwidth" => network_bandwidth(config),
        "setup_test" => setup_test(config),
        "single_vfn_perf" => single_vfn_perf(config),
        "validator_reboot_stress_test" => validator_reboot_stress_test(config),
        "fullnode_reboot_stress_test" => fullnode_reboot_stress_test(config),
        "account_creation" | "nft_mint" | "publishing" | "module_loading"
        | "write_new_resource" => individual_workload_tests(test_name.into(), config),
        "graceful_overload" => graceful_overload(config),
        "three_region_simulation_graceful_overload" => three_region_sim_graceful_overload(config),
        // not scheduled on continuous
        "load_vs_perf_benchmark" => load_vs_perf_benchmark(config),
        "workload_vs_perf_benchmark" => workload_vs_perf_benchmark(config),
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
        "twin_validator_test" => twin_validator_test(config),
        "large_db_simple_test" => large_db_simple_test(),
        "consensus_only_perf_benchmark" => run_consensus_only_perf_test(config),
        "consensus_only_three_region_simulation" => {
            run_consensus_only_three_region_simulation(config)
        },
        "quorum_store_reconfig_enable_test" => quorum_store_reconfig_enable_test(config),
        "multi_region_multi_cloud_simulation_test" => {
            multi_region_multi_cloud_simulation_test(config)
        },
        _ => return Err(format_err!("Invalid --suite given: {:?}", test_name)),
    };
    Ok(single_test_suite)
}

fn run_consensus_only_three_region_simulation(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 30000 })
                .txn_expiration_time_secs(5 * 60),
        )
        .with_network_tests(vec![&ThreeRegionSameCloudSimulationTest])
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

fn run_consensus_only_perf_test(config: ForgeConfig) -> ForgeConfig {
    let emit_job = config.get_emit_job().clone();
    config
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_network_tests(vec![&LoadVsPerfBenchmark {
            test: &PerformanceBenchmark,
            workloads: Workloads::TPS(&[30000]),
        }])
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

fn large_db_simple_test() -> ForgeConfig<'static> {
    large_db_test(10, 500, 300, "10-validators".to_string())
}

fn twin_validator_test(config: ForgeConfig) -> ForgeConfig {
    config
        .with_network_tests(vec![&TwinValidatorTest])
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(5)
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

fn state_sync_failures_catching_up() -> ForgeConfig<'static> {
    changing_working_quorum_test_helper(
        10,
        300,
        3000,
        2500,
        true,
        false,
        &ChangingWorkingQuorumTest {
            min_tps: 1500,
            always_healthy_nodes: 2,
            max_down_nodes: 1,
            num_large_validators: 2,
            add_execution_delay: false,
            check_period_s: 27,
        },
    )
}

fn state_sync_slow_processing_catching_up() -> ForgeConfig<'static> {
    changing_working_quorum_test_helper(
        10,
        300,
        3000,
        2500,
        true,
        true,
        &ChangingWorkingQuorumTest {
            min_tps: 750,
            always_healthy_nodes: 2,
            max_down_nodes: 0,
            num_large_validators: 2,
            add_execution_delay: true,
            check_period_s: 57,
        },
    )
}

fn different_node_speed_and_reliability_test() -> ForgeConfig<'static> {
    changing_working_quorum_test_helper(20, 120, 70, 50, true, false, &ChangingWorkingQuorumTest {
        min_tps: 30,
        always_healthy_nodes: 6,
        max_down_nodes: 5,
        num_large_validators: 3,
        add_execution_delay: true,
        check_period_s: 27,
    })
}

fn large_test_only_few_nodes_down() -> ForgeConfig<'static> {
    changing_working_quorum_test_helper(
        60,
        120,
        100,
        70,
        false,
        false,
        &ChangingWorkingQuorumTest {
            min_tps: 50,
            always_healthy_nodes: 40,
            max_down_nodes: 10,
            num_large_validators: 0,
            add_execution_delay: false,
            check_period_s: 27,
        },
    )
}

fn changing_working_quorum_test_high_load() -> ForgeConfig<'static> {
    changing_working_quorum_test_helper(20, 120, 500, 300, true, true, &ChangingWorkingQuorumTest {
        min_tps: 50,
        always_healthy_nodes: 0,
        max_down_nodes: 20,
        num_large_validators: 0,
        add_execution_delay: false,
        // Use longer check duration, as we are bringing enough nodes
        // to require state-sync to catch up to have consensus.
        check_period_s: 53,
    })
}

fn changing_working_quorum_test() -> ForgeConfig<'static> {
    changing_working_quorum_test_helper(20, 120, 100, 70, true, true, &ChangingWorkingQuorumTest {
        min_tps: 15,
        always_healthy_nodes: 0,
        max_down_nodes: 20,
        num_large_validators: 0,
        add_execution_delay: false,
        // Use longer check duration, as we are bringing enough nodes
        // to require state-sync to catch up to have consensus.
        check_period_s: 53,
    })
}

fn consensus_stress_test() -> ForgeConfig<'static> {
    changing_working_quorum_test_helper(10, 60, 100, 80, true, false, &ChangingWorkingQuorumTest {
        min_tps: 50,
        always_healthy_nodes: 10,
        max_down_nodes: 0,
        num_large_validators: 0,
        add_execution_delay: false,
        check_period_s: 27,
    })
}

fn load_vs_perf_benchmark(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(10)
        .with_network_tests(vec![&LoadVsPerfBenchmark {
            test: &PerformanceBenchmark,
            workloads: Workloads::TPS(&[
                200, 1000, 3000, 5000, 7000, 7500, 8000, 9000, 10000, 12000, 15000,
            ]),
        }])
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

fn workload_vs_perf_benchmark(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["config"]["execution"]
                ["processed_transactions_detailed_counters"] = true.into();
        }))
        // .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
        //     mempool_backlog: 10000,
        // }))
        .with_network_tests(vec![&LoadVsPerfBenchmark {
            test: &PerformanceBenchmark,
            workloads: Workloads::TRANSACTIONS(&[
                TransactinWorkload::NoOp,
                TransactinWorkload::NoOpUnique,
                TransactinWorkload::CoinTransfer,
                TransactinWorkload::CoinTransferUnique,
                TransactinWorkload::WriteResourceSmall,
                TransactinWorkload::WriteResourceBig,
                TransactinWorkload::LargeModuleWorkingSet,
                TransactinWorkload::PublishPackages,
                // TransactinWorkload::NftMint,
            ]),
        }])
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

fn graceful_overload(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
        // if we have full nodes for subset of validators, TPS drops.
        // Validators without VFN are proposing almost empty blocks,
        // as no useful transaction reach their mempool.
        // something to potentially improve upon.
        // So having VFNs for all validators
        .with_initial_fullnode_count(10)
        .with_network_tests(vec![&TwoTrafficsTest {
            inner_tps: 15000,
            inner_gas_price: aptos_global_constants::GAS_UNIT_PRICE,
            inner_init_gas_price_multiplier: 20,
            // because it is static, cannot use ::default_coin_transfer() method
            inner_transaction_type: TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool: false,
            },
            // Additionally - we are not really gracefully handling overlaods,
            // setting limits based on current reality, to make sure they
            // don't regress, but something to investigate
            avg_tps: 3400,
            latency_thresholds: &[],
        }])
        // First start higher gas-fee traffic, to not cause issues with TxnEmitter setup - account creation
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

fn three_region_sim_graceful_overload(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        // if we have full nodes for subset of validators, TPS drops.
        // Validators without VFN are proposing almost empty blocks,
        // as no useful transaction reach their mempool.
        // something to potentially improve upon.
        // So having VFNs for all validators
        .with_initial_fullnode_count(20)
        .with_network_tests(vec![&CompositeNetworkTest {
            wrapper: &ThreeRegionSameCloudSimulationTest,
            test: &TwoTrafficsTest {
                inner_tps: 15000,
                inner_gas_price: aptos_global_constants::GAS_UNIT_PRICE,
                inner_init_gas_price_multiplier: 20,
                // Cannot use default_coin_transfer(), as this needs to be static
                inner_transaction_type: TransactionType::CoinTransfer {
                    invalid_transaction_ratio: 0,
                    sender_use_account_pool: false,
                },
                // Additionally - we are not really gracefully handling overlaods,
                // setting limits based on current reality, to make sure they
                // don't regress, but something to investigate
                avg_tps: 3400,
                latency_thresholds: &[],
            },
        }])
        // First start higher gas-fee traffic, to not cause issues with TxnEmitter setup - account creation
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

fn individual_workload_tests(test_name: String, config: ForgeConfig) -> ForgeConfig {
    let job = EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
        mempool_backlog: 30000,
    });
    config
        .with_network_tests(vec![&PerformanceBenchmark])
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_initial_fullnode_count(3)
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
                    "account_creation" => TransactionType::default_account_generation(),
                    "nft_mint" => TransactionType::NftMintAndTransfer,
                    "publishing" => TransactionType::PublishPackage {
                        use_account_pool: false,
                    },
                    "module_loading" => TransactionType::CallCustomModules {
                        entry_point: EntryPoints::Nop,
                        num_modules: 1000,
                        use_account_pool: false,
                    },
                    _ => unreachable!("{}", test_name),
                })
            },
        )
        .with_success_criteria(
            SuccessCriteria::new(match test_name.as_str() {
                "account_creation" => 3700,
                "nft_mint" => 1000,
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

fn fullnode_reboot_stress_test(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
        .with_initial_fullnode_count(10)
        .with_network_tests(vec![&FullNodeRebootStressTest])
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .with_success_criteria(SuccessCriteria::new(2000).add_wait_for_catchup_s(600))
}

fn validator_reboot_stress_test(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(15).unwrap())
        .with_initial_fullnode_count(1)
        .with_network_tests(vec![&ValidatorRebootStressTest {
            num_simultaneously: 3,
            down_time_secs: 5.0,
            pause_secs: 5.0,
        }])
        .with_success_criteria(SuccessCriteria::new(2000).add_wait_for_catchup_s(600))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 120.into();
        }))
}

fn single_vfn_perf(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(1).unwrap())
        .with_initial_fullnode_count(1)
        .with_network_tests(vec![&PerformanceBenchmark])
        .with_success_criteria(
            SuccessCriteria::new(5000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240),
        )
}

fn setup_test(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_fullnode_count(1)
        .with_network_tests(vec![&ForgeSetupTest])
}

fn network_bandwidth(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(8).unwrap())
        .with_network_tests(vec![&NetworkBandwidthTest])
}

fn three_region_simulation_with_different_node_speed(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .with_initial_fullnode_count(30)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .with_network_tests(vec![&CompositeNetworkTest {
            wrapper: &ExecutionDelayTest {
                add_execution_delay: ExecutionDelayConfig {
                    inject_delay_node_fraction: 0.5,
                    inject_delay_max_transaction_percentage: 40,
                    inject_delay_per_transaction_ms: 2,
                },
            },
            test: &ThreeRegionSameCloudSimulationTest,
        }])
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

fn three_region_simulation(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(12).unwrap())
        .with_initial_fullnode_count(12)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .with_network_tests(vec![&ThreeRegionSameCloudSimulationTest])
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

fn network_partition(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
        .with_network_tests(vec![&NetworkPartitionTest])
        .with_success_criteria(
            SuccessCriteria::new(2500)
                .add_no_restarts()
                .add_wait_for_catchup_s(240),
        )
}

fn compat(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_network_tests(vec![&SimpleValidatorUpgrade])
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 30.into();
        }))
}

fn upgrade(config: ForgeConfig) -> ForgeConfig {
    config
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_network_tests(vec![&FrameworkUpgrade])
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 30.into();
        }))
}

fn epoch_changer_performance(config: ForgeConfig) -> ForgeConfig {
    config
        .with_network_tests(vec![&PerformanceBenchmark])
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_initial_fullnode_count(2)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 60.into();
        }))
}

/// A default config for running various state sync performance tests
fn state_sync_perf_fullnodes_config(forge_config: ForgeConfig<'static>) -> ForgeConfig<'static> {
    forge_config
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .with_initial_fullnode_count(4)
}

/// The config for running a state sync performance test when applying
/// transaction outputs in fullnodes.
fn state_sync_perf_fullnodes_apply_outputs(
    forge_config: ForgeConfig<'static>,
) -> ForgeConfig<'static> {
    state_sync_perf_fullnodes_config(forge_config)
        .with_network_tests(vec![&StateSyncFullnodePerformance])
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
fn state_sync_perf_fullnodes_execute_transactions(
    forge_config: ForgeConfig<'static>,
) -> ForgeConfig<'static> {
    state_sync_perf_fullnodes_config(forge_config)
        .with_network_tests(vec![&StateSyncFullnodePerformance])
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
fn state_sync_perf_fullnodes_fast_sync(forge_config: ForgeConfig<'static>) -> ForgeConfig<'static> {
    state_sync_perf_fullnodes_config(forge_config)
        .with_network_tests(vec![&StateSyncFullnodeFastSyncPerformance])
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 180.into(); // Frequent epochs
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 30000,
                })
                .transaction_type(TransactionType::default_account_generation()), // Create many state values
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
fn state_sync_perf_validators(forge_config: ForgeConfig<'static>) -> ForgeConfig<'static> {
    forge_config
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
        .with_network_tests(vec![&StateSyncValidatorPerformance])
        .with_success_criteria(SuccessCriteria::new(5000))
}

/// The config for running a validator join and leave test.
fn validators_join_and_leave(forge_config: ForgeConfig<'static>) -> ForgeConfig<'static> {
    forge_config
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 60.into();
            helm_values["chain"]["allow_new_validators"] = true.into();
        }))
        .with_network_tests(vec![&ValidatorJoinLeaveTest])
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

fn land_blocking_test_suite(duration: Duration) -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(10)
        .with_network_tests(vec![&PerformanceBenchmark])
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

fn pre_release_suite() -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .with_network_tests(vec![&NetworkBandwidthTest])
}

fn chaos_test_suite(duration: Duration) -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .with_network_tests(vec![
            &NetworkBandwidthTest,
            &ThreeRegionSameCloudSimulationTest,
            &NetworkLossTest,
        ])
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
    test: &'static ChangingWorkingQuorumTest,
) -> ForgeConfig<'static> {
    let config = ForgeConfig::default();
    let num_large_validators = test.num_large_validators;
    config
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(
            if test.max_down_nodes == 0 {
                0
            } else {
                std::cmp::max(2, target_tps / 1000)
            },
        )
        .with_network_tests(vec![test])
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
            helm_values["validator"]["config"]["consensus"]["max_receiving_block_txns"] =
                block_size.into();
            helm_values["validator"]["config"]["consensus"]["round_initial_timeout_ms"] =
                500.into();
            helm_values["validator"]["config"]["consensus"]
                ["round_timeout_backoff_exponent_base"] = 1.0.into();
            helm_values["validator"]["config"]["consensus"]["quorum_store_poll_count"] = 1.into();

            let mut chain_health_backoff = ConsensusConfig::default().chain_health_backoff;
            if use_chain_backoff {
                // Generally if we are stress testing the consensus, we don't want to slow it down.
                chain_health_backoff = vec![];
            } else {
                for (i, item) in chain_health_backoff.iter_mut().enumerate() {
                    // as we have lower TPS, make limits smaller
                    item.max_sending_block_txns_override =
                        (block_size / 2_u64.pow(i as u32 + 1)).max(2);
                    // as we have fewer nodes, make backoff triggered earlier:
                    item.backoff_if_below_participating_voting_power_percentage = 90 - i * 5;
                }
            }

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
                    (TransactionType::default_coin_transfer(), 80),
                    (TransactionType::default_account_generation(), 20),
                ]),
        )
        .with_success_criteria(
            SuccessCriteria::new(min_avg_tps)
                .add_no_restarts()
                .add_wait_for_catchup_s(30)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: if test.max_down_nodes == 0 {
                        // very aggressive if no nodes are expected to be down
                        3.0
                    } else if test.max_down_nodes * 3 + 1 + 2 < num_validators {
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
) -> ForgeConfig<'static> {
    let config = ForgeConfig::default();
    config
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(std::cmp::max(2, target_tps / 1000))
        .with_network_tests(vec![&PerformanceBenchmark])
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
                    (TransactionType::default_coin_transfer(), 75),
                    (TransactionType::default_account_generation(), 20),
                    (TransactionType::NftMintAndTransfer, 5),
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

fn quorum_store_reconfig_enable_test(forge_config: ForgeConfig<'static>) -> ForgeConfig<'static> {
    forge_config
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(20)
        .with_network_tests(vec![&QuorumStoreOnChainEnableTest {}])
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

fn multi_region_multi_cloud_simulation_test(config: ForgeConfig<'static>) -> ForgeConfig<'static> {
    config
        .with_initial_validator_count(NonZeroUsize::new(100).unwrap())
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 200_000,
                })
                .txn_expiration_time_secs(5 * 60),
        )
        .with_network_tests(vec![&MultiRegionMultiCloudSimulationTest {}])
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
    fn run<'t>(&self, ctx: &mut AdminContext<'t>) -> Result<()> {
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
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
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
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = Duration::from_secs(10);
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let stats = generate_traffic(ctx, &all_validators, duration).unwrap();
        ctx.report
            .report_txn_stats(self.name().to_string(), &stats, duration);

        Ok(())
    }
}
