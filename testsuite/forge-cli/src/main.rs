// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Context, Result};
use aptos_logger::Level;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{move_types::account_address::AccountAddress, transaction_builder::aptos_stdlib};
use forge::success_criteria::SuccessCriteria;
use forge::{ForgeConfig, Options, *};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{env, num::NonZeroUsize, process, thread, time::Duration};
use structopt::StructOpt;
use testcases::continuous_progress_test::ContinuousProgressTest;
use testcases::network_bandwidth_test::NetworkBandwidthTest;
use testcases::network_latency_test::NetworkLatencyTest;
use testcases::network_loss_test::NetworkLossTest;
use testcases::performance_with_fullnode_test::PerformanceBenchmarkWithFN;
use testcases::{
    compatibility_test::SimpleValidatorUpgrade, forge_setup_test::ForgeSetupTest, generate_traffic,
    network_partition_test::NetworkPartitionTest, performance_test::PerformanceBenchmark,
    reconfiguration_test::ReconfigurationTest, state_sync_performance::StateSyncPerformance,
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

    let duration = Duration::from_secs(1200);
    let suite_name = "consensus_stress_test";

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
                    test_suite.get_mut_success_criteria().avg_tps = 400;
                    test_suite.get_mut_success_criteria().max_latency_ms = 60000;
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
        .with_aptos_tests(&[&FundAccount, &TransferCoins])
        .with_admin_tests(&[&GetMetadata])
        .with_genesis_module_bundle(cached_packages::head_release_bundle().clone())
        .with_aptos_tests(&[&RunForever])
}

fn local_test_suite() -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_aptos_tests(&[&FundAccount, &TransferCoins])
        .with_admin_tests(&[&GetMetadata])
        .with_network_tests(&[&RestartValidator, &EmitTransaction])
        .with_genesis_module_bundle(cached_packages::head_release_bundle().clone())
}

fn k8s_test_suite() -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .with_aptos_tests(&[&FundAccount, &TransferCoins])
        .with_admin_tests(&[&GetMetadata])
        .with_network_tests(&[
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
            .with_network_tests(&[&PerformanceBenchmark])
            .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
            .with_initial_fullnode_count(2)
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 60.into();
            })),
        "state_sync" => config
            .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
            .with_initial_fullnode_count(4)
            .with_network_tests(&[&StateSyncPerformance])
            .with_success_criteria(SuccessCriteria::new(5000, 10000, false, None)),
        "compat" => config
            .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
            .with_network_tests(&[&SimpleValidatorUpgrade])
            .with_success_criteria(SuccessCriteria::new(
                5000,
                10000,
                false,
                Some(Duration::from_secs(240)),
            )),
        "config" => config.with_network_tests(&[&ReconfigurationTest]),
        "network_partition" => config.with_network_tests(&[&NetworkPartitionTest]),
        "network_latency" => config
            .with_network_tests(&[&NetworkLatencyTest])
            .with_success_criteria(SuccessCriteria::new(4000, 10000, None)),
        "network_bandwidth" => config.with_network_tests(&[&NetworkBandwidthTest]),
        "setup_test" => config
            .with_initial_fullnode_count(1)
            .with_network_tests(&[&ForgeSetupTest]),
        "single_vfn_perf" => config
            .with_initial_validator_count(NonZeroUsize::new(1).unwrap())
            .with_initial_fullnode_count(1)
            .with_network_tests(&[&PerformanceBenchmarkWithFN])
            .with_success_criteria(SuccessCriteria::new(
                5000,
                10000,
                true,
                Some(Duration::from_secs(240)),
            )),
        "account_creation_state_sync" => config
            .with_network_tests(&[&PerformanceBenchmarkWithFN])
            .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
            .with_initial_fullnode_count(3)
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 1200.into();
            }))
            .with_emit_job(
                EmitJobRequest::default()
                    .mode(EmitJobMode::MaxLoad {
                        mempool_backlog: 30000,
                    })
                    .transaction_type(TransactionType::AccountGeneration),
            )
            .with_success_criteria(SuccessCriteria::new(
                4000,
                10000,
                true,
                Some(Duration::from_secs(240)),
            )),
        "consensus_stress_test" => config
            .with_network_tests(&[&ContinuousProgressTest { target_tps: 100 }])
            .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
            .with_genesis_helm_config_fn(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 60.into();
            }))
            .with_node_helm_config_fn(Arc::new(|helm_values| {
                helm_values["consensus"]["max_block_txns"] = 50.into();
                helm_values["consensus"]["round_initial_timeout_ms"] = 500.into();
                helm_values["consensus"]["round_timeout_backoff_exponent_base"] = 1.0.into();
                helm_values["consensus"]["quorum_store_poll_count"] = 1.into();
            }))
            .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 100 }))
            .with_success_criteria(SuccessCriteria::new(
                80,
                10000,
                true,
                Some(Duration::from_secs(30)),
            )),
        _ => return Err(format_err!("Invalid --suite given: {:?}", test_name)),
    };
    Ok(single_test_suite)
}

fn land_blocking_test_suite(duration: Duration) -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(10)
        .with_network_tests(&[&PerformanceBenchmarkWithFN])
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
        ))
}

fn pre_release_suite() -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .with_network_tests(&[&NetworkBandwidthTest])
}

fn chaos_test_suite(duration: Duration) -> ForgeConfig<'static> {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .with_network_tests(&[&NetworkBandwidthTest, &NetworkLatencyTest, &NetworkLossTest])
        .with_success_criteria(SuccessCriteria::new(
            if duration > Duration::from_secs(1200) {
                100
            } else {
                1000
            },
            10000,
            true,
            None,
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
        let stats = generate_traffic(ctx, &all_validators, duration, 1).unwrap();
        ctx.report
            .report_txn_stats(self.name().to_string(), &stats, duration);

        Ok(())
    }
}
