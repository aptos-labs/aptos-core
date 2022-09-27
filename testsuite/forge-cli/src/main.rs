// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Context, Result};
use aptos_logger::Level;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{move_types::account_address::AccountAddress, transaction_builder::aptos_stdlib};
use forge::success_criteria::{StateProgressThreshold, SuccessCriteria};
use forge::system_metrics::{MetricsThreshold, SystemMetricsThreshold};
use forge::{ForgeConfig, Options, *};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{env, num::NonZeroUsize, process, thread, time::Duration};
use structopt::StructOpt;
use testcases::consensus_reliability_tests::ChangingWorkingQuorumTest;
use testcases::fullnode_reboot_stress_test::FullNodeRebootStressTest;
use testcases::load_vs_perf_benchmark::LoadVsPerfBenchmark;
use testcases::network_bandwidth_test::NetworkBandwidthTest;
use testcases::network_loss_test::NetworkLossTest;
use testcases::performance_with_fullnode_test::PerformanceBenchmarkWithFN;
use testcases::state_sync_performance::StateSyncValidatorPerformance;
use testcases::three_region_simulation_test::ThreeRegionSimulationTest;
use testcases::twin_validator_test::TwinValidatorTest;
use testcases::validator_reboot_stress_test::ValidatorRebootStressTest;
use testcases::{
    compatibility_test::SimpleValidatorUpgrade, forge_setup_test::ForgeSetupTest, generate_traffic,
    network_partition_test::NetworkPartitionTest, performance_test::PerformanceBenchmark,
    reconfiguration_test::ReconfigurationTest,
    state_sync_performance::StateSyncFullnodePerformance,
};
use tokio::runtime::Runtime;
use url::Url;

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
    logger
        .channel_size(1000)
        .is_async(false)
        .level(Level::Info)
        .read_env();
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
                    test_suite.get_success_criteria_mut().max_latency_ms = 60000;
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
                }
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
                }
            }
        }
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
            }
            OperatorCommand::CleanUp(cleanup) => {
                if let Some(namespace) = cleanup.namespace {
                    runtime.block_on(uninstall_testnet_resources(namespace))?;
                } else {
                    runtime.block_on(cleanup_cluster_with_management())?;
                }
                Ok(())
            }
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
            }
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
        }
        Err(e) => {
            eprintln!("Failed to run tests:\n{}", e);
            Err(e)
        }
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
        }
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
        }
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
        .with_genesis_module_bundle(cached_packages::head_release_bundle().clone())
        .with_aptos_tests(vec![&RunForever])
}

fn local_test_suite() -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_aptos_tests(vec![&FundAccount, &TransferCoins])
        .with_admin_tests(vec![&GetMetadata])
        .with_network_tests(vec![&RestartValidator, &EmitTransaction])
        .with_genesis_module_bundle(cached_packages::head_release_bundle().clone())
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
        "epoch_changer_performance" => config
            .with_network_tests(vec![&PerformanceBenchmark])
            .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
            .with_initial_fullnode_count(2)
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 60.into();
            })),
        "state_sync_perf_fullnodes_apply_outputs" => {
            state_sync_perf_fullnodes_apply_outputs(config)
        }
        "state_sync_perf_fullnodes_execute_transactions" => {
            state_sync_perf_fullnodes_execute_transactions(config)
        }
        "state_sync_perf_validators" => state_sync_perf_validators(config),
        "compat" => config
            .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
            .with_network_tests(vec![&SimpleValidatorUpgrade])
            .with_success_criteria(SuccessCriteria::new(
                5000,
                10000,
                false,
                Some(Duration::from_secs(240)),
                None,
                None,
            ))
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 30.into();
            })),
        "config" => config.with_network_tests(vec![&ReconfigurationTest]),
        "network_partition" => config
            .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
            .with_network_tests(vec![&NetworkPartitionTest])
            .with_success_criteria(SuccessCriteria::new(
                3000,
                10000,
                true,
                Some(Duration::from_secs(240)),
                None,
                None,
            )),
        "three_region_simulation" => config
            .with_initial_validator_count(NonZeroUsize::new(12).unwrap())
            .with_initial_fullnode_count(12)
            .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
            .with_network_tests(vec![&ThreeRegionSimulationTest])
            // TODO(rustielin): tune these success critiera after we have a better idea of the test behavior
            .with_success_criteria(SuccessCriteria::new(3000, 100000, true, None, None, None)),
        "network_bandwidth" => config
            .with_initial_validator_count(NonZeroUsize::new(8).unwrap())
            .with_network_tests(vec![&NetworkBandwidthTest]),
        "setup_test" => config
            .with_initial_fullnode_count(1)
            .with_network_tests(vec![&ForgeSetupTest]),
        "single_vfn_perf" => config
            .with_initial_validator_count(NonZeroUsize::new(1).unwrap())
            .with_initial_fullnode_count(1)
            .with_network_tests(vec![&PerformanceBenchmarkWithFN])
            .with_success_criteria(SuccessCriteria::new(
                5000,
                10000,
                true,
                Some(Duration::from_secs(240)),
                None,
                None,
            )),
        "validator_reboot_stress_test" => config
            .with_initial_validator_count(NonZeroUsize::new(15).unwrap())
            .with_initial_fullnode_count(1)
            .with_network_tests(vec![&ValidatorRebootStressTest])
            .with_success_criteria(SuccessCriteria::new(
                2000,
                50000,
                false,
                Some(Duration::from_secs(600)),
                None,
                None,
            ))
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 120.into();
            })),
        "fullnode_reboot_stress_test" => config
            .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
            .with_initial_fullnode_count(10)
            .with_network_tests(vec![&FullNodeRebootStressTest])
            .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
            .with_success_criteria(SuccessCriteria::new(
                2000,
                50000,
                false,
                Some(Duration::from_secs(600)),
                None,
                None,
            )),
        "account_creation" | "nft_mint" => config
            .with_network_tests(vec![&PerformanceBenchmarkWithFN])
            .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
            .with_initial_fullnode_count(3)
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 600.into();
            }))
            .with_emit_job(
                EmitJobRequest::default()
                    .mode(EmitJobMode::MaxLoad {
                        mempool_backlog: 30000,
                    })
                    .transaction_type(if test_name == "account_creation" {
                        TransactionType::AccountGeneration
                    } else {
                        TransactionType::NftMint
                    }),
            )
            .with_success_criteria(SuccessCriteria::new(
                4000,
                10000,
                true,
                Some(Duration::from_secs(240)),
                None,
                None,
            )),
        // TODO: Add tracing latency of high-gas-fee transactions
        "graceful_overload" => config
            .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
            // if we have smaller number of full nodes, TPS drops.
            // Validators without VFN are proposing almost empty blocks,
            // as no useful transaction reach their mempool.
            // something to potentially improve upon.
            .with_initial_fullnode_count(8)
            .with_network_tests(vec![&PerformanceBenchmarkWithFN])
            .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 15000 }))
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 300.into();
            }))
            .with_success_criteria(SuccessCriteria::new(
                6500,
                50000,
                true,
                Some(Duration::from_secs(120)),
                Some(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12, 30),
                    // Check that we don't use more than 5 GB of memory for 30% of the time.
                    MetricsThreshold::new(5 * 1024 * 1024 * 1024, 30),
                )),
                Some(StateProgressThreshold {
                    max_no_progress_secs: 30.0,
                    max_round_gap: 10,
                }),
            )),
        // not scheduled on continuous
        "load_vs_perf_benchmark" => config
            .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
            .with_initial_fullnode_count(10)
            .with_network_tests(vec![&LoadVsPerfBenchmark {
                test: &PerformanceBenchmarkWithFN,
                tps: &[
                    200, 1000, 3000, 5000, 7000, 7500, 8000, 9000, 10000, 12000, 15000,
                ],
            }])
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                // no epoch change.
                helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
            }))
            .with_success_criteria(SuccessCriteria::new(
                0,
                10000,
                true,
                Some(Duration::from_secs(60)),
                None,
                None,
            )),
        // maximizing number of rounds and epochs within a given time, to stress test consensus
        // so using small constant traffic, small blocks and fast rounds, and short epochs.
        // reusing changing_working_quorum_test just for invariants/asserts, but with max_down_nodes = 0.
        "consensus_stress_test" => changing_working_quorum_test(
            10,
            60,
            100,
            80,
            &ChangingWorkingQuorumTest {
                min_tps: 50,
                always_healthy_nodes: 10,
                max_down_nodes: 0,
                num_large_validators: 0,
                add_execution_delay: false,
                // Check that every 27s all nodes make progress,
                // without any failures.
                // (make epoch length (120s) and this duration (27s) not be multiples of one another,
                // to test different timings)
                check_period_s: 27,
            },
            false,
        ),
        "changing_working_quorum_test" => changing_working_quorum_test(
            20,
            120,
            100,
            70,
            &ChangingWorkingQuorumTest {
                min_tps: 15,
                always_healthy_nodes: 0,
                max_down_nodes: 20,
                num_large_validators: 0,
                add_execution_delay: false,
                // Check that every 27s all nodes make progress,
                // without any failures.
                // (make epoch length (120s) and this duration (27s) not be multiples of one another,
                // to test different timings)
                check_period_s: 27,
            },
            false,
        ),
        "changing_working_quorum_test_high_load" => changing_working_quorum_test(
            20,
            120,
            500,
            300,
            &ChangingWorkingQuorumTest {
                min_tps: 50,
                always_healthy_nodes: 0,
                max_down_nodes: 20,
                num_large_validators: 0,
                add_execution_delay: false,
                // Check that every 27s all nodes make progress,
                // without any failures.
                // (make epoch length (120s) and this duration (27s) not be multiples of one another,
                // to test different timings)
                check_period_s: 27,
            },
            // Max load cannot be sustained without gaps in progress.
            // Using high load instead.
            false,
        ),
        // not scheduled on continuous
        "large_test_only_few_nodes_down" => changing_working_quorum_test(
            60,
            120,
            100,
            70,
            &ChangingWorkingQuorumTest {
                min_tps: 50,
                always_healthy_nodes: 40,
                max_down_nodes: 10,
                num_large_validators: 0,
                add_execution_delay: false,
                check_period_s: 27,
            },
            false,
        ),
        "different_node_speed_and_reliability_test" => changing_working_quorum_test(
            20,
            120,
            100,
            70,
            &ChangingWorkingQuorumTest {
                min_tps: 50,
                always_healthy_nodes: 6,
                max_down_nodes: 5,
                num_large_validators: 3,
                add_execution_delay: true,
                check_period_s: 27,
            },
            false,
        ),
        "twin_validator_test" => config
            .with_network_tests(vec![&TwinValidatorTest])
            .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
            .with_initial_fullnode_count(5)
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 300.into();
            }))
            .with_success_criteria(SuccessCriteria::new(
                6000,
                10000,
                true,
                Some(Duration::from_secs(60)),
                Some(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12, 30),
                    // Check that we don't use more than 5 GB of memory for 30% of the time.
                    MetricsThreshold::new(5 * 1024 * 1024 * 1024, 30),
                )),
                Some(StateProgressThreshold {
                    max_no_progress_secs: 10.0,
                    max_round_gap: 4,
                }),
            )),
        _ => return Err(format_err!("Invalid --suite given: {:?}", test_name)),
    };
    Ok(single_test_suite)
}

/// A default config for running various state sync performance tests
fn state_sync_perf_fullnodes_config(forge_config: ForgeConfig<'static>) -> ForgeConfig<'static> {
    forge_config
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .with_initial_fullnode_count(4)
        .with_network_tests(vec![&StateSyncFullnodePerformance])
}

/// The config for running a state sync performance test when applying
/// transaction outputs in fullnodes.
fn state_sync_perf_fullnodes_apply_outputs(
    forge_config: ForgeConfig<'static>,
) -> ForgeConfig<'static> {
    state_sync_perf_fullnodes_config(forge_config)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["bootstrapping_mode"] = "ApplyTransactionOutputsFromGenesis".into();
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["continuous_syncing_mode"] = "ApplyTransactionOutputs".into();
        }))
        .with_success_criteria(SuccessCriteria::new(10000, 10000, false, None, None, None))
}

/// The config for running a state sync performance test when executing
/// transactions in fullnodes.
fn state_sync_perf_fullnodes_execute_transactions(
    forge_config: ForgeConfig<'static>,
) -> ForgeConfig<'static> {
    state_sync_perf_fullnodes_config(forge_config)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_node_helm_config_fn(Arc::new(|helm_values| {
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["bootstrapping_mode"] = "ExecuteTransactionsFromGenesis".into();
            helm_values["fullnode"]["config"]["state_sync"]["state_sync_driver"]
                ["continuous_syncing_mode"] = "ExecuteTransactions".into();
        }))
        .with_success_criteria(SuccessCriteria::new(5000, 10000, false, None, None, None))
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
        .with_success_criteria(SuccessCriteria::new(5000, 10000, false, None, None, None))
}

fn land_blocking_test_suite(duration: Duration) -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(10)
        .with_network_tests(vec![&PerformanceBenchmarkWithFN])
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // Have single epoch change in land blocking
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(SuccessCriteria::new(
            if duration.as_secs() > 1200 {
                5000
            } else {
                6000
            },
            10000,
            true,
            Some(Duration::from_secs(if duration.as_secs() > 1200 {
                240
            } else {
                60
            })),
            Some(SystemMetricsThreshold::new(
                // Check that we don't use more than 12 CPU cores for 30% of the time.
                MetricsThreshold::new(12, 30),
                // Check that we don't use more than 10 GB of memory for 30% of the time.
                MetricsThreshold::new(10 * 1024 * 1024 * 1024, 30),
            )),
            Some(StateProgressThreshold {
                max_no_progress_secs: 10.0,
                max_round_gap: 4,
            }),
        ))
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
            &ThreeRegionSimulationTest,
            &NetworkLossTest,
        ])
        .with_success_criteria(SuccessCriteria::new(
            if duration > Duration::from_secs(1200) {
                100
            } else {
                1000
            },
            10000,
            true,
            None,
            Some(SystemMetricsThreshold::new(
                // Check that we don't use more than 12 CPU cores for 30% of the time.
                MetricsThreshold::new(12, 30),
                // Check that we don't use more than 5 GB of memory for 30% of the time.
                MetricsThreshold::new(5 * 1024 * 1024 * 1024, 30),
            )),
            None,
        ))
}

fn changing_working_quorum_test(
    num_validators: usize,
    epoch_duration: usize,
    target_tps: usize,
    min_avg_tps: usize,
    test: &'static ChangingWorkingQuorumTest,
    max_load: bool,
) -> ForgeConfig<'static> {
    let config = ForgeConfig::default();
    let num_large_validators = test.num_large_validators;
    config
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(if test.max_down_nodes == 0 {
            0
        } else {
            std::cmp::max(2, target_tps / 1000)
        })
        .with_network_tests(vec![test])
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = epoch_duration.into();
            helm_values["genesis"]["validator"]["num_validators_with_larger_stake"] =
                num_large_validators.into();
        }))
        .with_node_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["validator"]["config"]["api"]["failpoints_enabled"] = true.into();
            helm_values["validator"]["config"]["consensus"]["max_block_txns"] =
                (target_tps / 4).into();
            helm_values["validator"]["config"]["consensus"]["round_initial_timeout_ms"] =
                500.into();
            helm_values["validator"]["config"]["consensus"]
                ["round_timeout_backoff_exponent_base"] = 1.0.into();
            helm_values["validator"]["config"]["consensus"]["quorum_store_poll_count"] = 1.into();
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(if max_load {
                    EmitJobMode::MaxLoad {
                        mempool_backlog: 20000,
                    }
                } else {
                    EmitJobMode::ConstTps { tps: target_tps }
                })
                .transaction_mix(vec![
                    (TransactionType::P2P, 80),
                    (TransactionType::AccountGeneration, 20),
                ]),
        )
        .with_success_criteria(SuccessCriteria::new(
            min_avg_tps,
            10000,
            true,
            Some(Duration::from_secs(30)),
            None,
            Some(StateProgressThreshold {
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
        ))
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
        let stats = generate_traffic(
            ctx,
            &all_validators,
            duration,
            aptos_global_constants::GAS_UNIT_PRICE,
        )
        .unwrap();
        ctx.report
            .report_txn_stats(self.name().to_string(), &stats, duration);

        Ok(())
    }
}
