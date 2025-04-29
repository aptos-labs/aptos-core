// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::field_reassign_with_default)]

use anyhow::{bail, format_err, Context, Result};
use aptos_forge::{config::ForgeConfig, Options, *};
use aptos_logger::Level;
use clap::{Parser, Subcommand};
use futures::{future, FutureExt};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use serde_json::{json, Value};
use std::{self, env, num::NonZeroUsize, process, time::Duration};
use sugars::{boxed, hmap};
use suites::{
    dag::get_dag_test,
    indexer::get_indexer_test,
    land_blocking::get_land_blocking_test,
    multi_region::get_multi_region_test,
    netbench::get_netbench_test,
    pfn::get_pfn_test,
    realistic_environment::get_realistic_env_test,
    state_sync::get_state_sync_test,
    ungrouped::{
        chaos_test_suite, get_ungrouped_test, k8s_test_suite, local_test_suite, pre_release_suite,
        run_forever,
    },
};
use tokio::runtime::Runtime;
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
    /// Create a new cluster for testing purposes
    Create(Create),
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
    #[clap(long, help = "If set, spins up an indexer stack alongside the testnet")]
    enable_indexer: bool,
    #[clap(
        long,
        help = "The deployer profile used to spin up and configure forge infrastructure",
        default_value = &DEFAULT_FORGE_DEPLOYER_PROFILE,
    )]
    deployer_profile: String,
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
    #[clap(long, help = "If set, skips the actual cleanup")]
    dry_run: bool,
}

#[derive(Parser, Debug)]
struct Create {
    #[clap(long, help = "The kubernetes namespace to create in")]
    namespace: String,
    #[clap(long, default_value_t = 30)]
    num_validators: usize,
    #[clap(long, default_value_t = 1)]
    num_fullnodes: usize,
    #[clap(
        long,
        help = "Override the image tag used for validators",
        default_value = "main"
    )]
    validator_image_tag: String,
    #[clap(
        long,
        help = "Override the image tag used for testnet-specific components",
        default_value = "main"
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
    #[clap(long, help = "If set, spins up an indexer stack alongside the testnet")]
    enable_indexer: bool,
    #[clap(
        long,
        help = "Override the image tag used for indexer",
        requires = "enable_indexer"
    )]
    indexer_image_tag: Option<String>,
    #[clap(
        long,
        help = "The deployer profile used to spin up and configure forge infrastructure",
        default_value = &DEFAULT_FORGE_DEPLOYER_PROFILE,

    )]
    deployer_profile: String,
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

    env_logger::init();

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
                    let forge = Forge::new(
                        &args.options,
                        test_suite,
                        duration,
                        LocalFactory::from_workspace(swarm_dir)?,
                    );
                    run_forge_with_changelog(forge, &args.options, args.changelog.clone())
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
                    let forge = Forge::new(
                        &args.options,
                        test_suite,
                        duration,
                        K8sFactory::new(
                            namespace,
                            k8s.image_tag.clone(),
                            k8s.upgrade_image_tag.clone(),
                            // We want to port forward if we're running locally because local means we're not in cluster
                            k8s.port_forward || forge_runner_mode == ForgeRunnerMode::Local,
                            k8s.reuse,
                            k8s.keep,
                            k8s.enable_haproxy,
                            k8s.enable_indexer,
                            k8s.deployer_profile.clone(),
                        )?,
                    );
                    run_forge_with_changelog(forge, &args.options, args.changelog)
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
                    runtime.block_on(cleanup_cluster_with_management(cleanup.dry_run))?;
                }
                Ok(())
            },
            OperatorCommand::Create(create) => {
                let kube_client = runtime.block_on(create_k8s_client())?;
                let era = generate_new_era();
                let indexer_image_tag = create
                    .indexer_image_tag
                    .or(Some(create.validator_image_tag.clone()))
                    .expect("Expected indexer or validator image tag to use");
                let config: Value = serde_json::from_value(json!({
                    "profile": create.deployer_profile,
                    "era": era.clone(),
                    "namespace": create.namespace.clone(),
                    "indexer-grpc-values": {
                        "indexerGrpcImage": format!("{}:{}", INDEXER_GRPC_DOCKER_IMAGE_REPO, &indexer_image_tag),
                        "fullnodeConfig": {
                            "image": format!("{}:{}", VALIDATOR_DOCKER_IMAGE_REPO, &indexer_image_tag),
                        }
                    },
                }))?;

                let deploy_testnet_fut = async {
                    install_testnet_resources(
                        era.clone(),
                        create.namespace.clone(),
                        create.num_validators,
                        create.num_fullnodes,
                        create.validator_image_tag,
                        create.testnet_image_tag,
                        create.move_modules_dir,
                        false, // since we skip_collecting_running_nodes, we don't connect directly to the nodes to validatet their health
                        create.enable_haproxy,
                        create.enable_indexer,
                        create.deployer_profile,
                        None,
                        None,
                        true,
                    )
                    .await
                }
                .boxed();

                let deploy_indexer_fut = async {
                    if create.enable_indexer {
                        let indexer_deployer = ForgeDeployerManager::new(
                            kube_client.clone(),
                            create.namespace.clone(),
                            FORGE_INDEXER_DEPLOYER_DOCKER_IMAGE_REPO.to_string(),
                            None,
                        );
                        indexer_deployer.start(config).await?;
                        indexer_deployer.wait_completed().await
                    } else {
                        Ok(())
                    }
                }
                .boxed();

                runtime.block_on(future::try_join(deploy_testnet_fut, deploy_indexer_fut))?;
                Ok(())
            },
        },
    }
}

pub fn run_forge_with_changelog<F: Factory>(
    forge: Forge<F>,
    options: &Options,
    optional_changelog: Option<Vec<String>>,
) -> Result<()> {
    if options.list {
        forge.list()?;

        return Ok(());
    }

    let forge_result = forge.run();
    let report = forge_result.map_err(|e| {
        eprintln!("Failed to run tests:\n{}", e);
        anyhow::anyhow!(e)
    })?;

    if let Some(changelog) = optional_changelog {
        if changelog.len() != 2 {
            println!("Use: changelog <from> <to>");
            process::exit(1);
        }
        let to_commit = changelog[1].clone();
        let from_commit = Some(changelog[0].clone());
        send_changelog_message(&report.to_string(), &from_commit, &to_commit);
    }
    Ok(())
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
    // These are high level suite aliases that express an intent
    let suite_aliases = hmap! {
        "local_test_suite" => boxed!(local_test_suite) as Box<dyn Fn() -> ForgeConfig>,
        "pre_release" => boxed!(pre_release_suite),
        "run_forever" => boxed!(run_forever),
        "k8s_suite" => boxed!(k8s_test_suite),
        "chaos" => boxed!(|| chaos_test_suite(duration)),
    };

    if let Some(test_suite) = suite_aliases.get(test_name) {
        return Ok(test_suite());
    }

    // Otherwise, check the test name against the grouped test suites
    // This is done in order of priority
    // A match higher up in the list will take precedence
    let named_test_suites = [
        boxed!(|| get_land_blocking_test(test_name, duration, test_cmd))
            as Box<dyn Fn() -> Option<ForgeConfig>>,
        boxed!(|| get_multi_region_test(test_name)),
        boxed!(|| get_netbench_test(test_name)),
        boxed!(|| get_pfn_test(test_name, duration)),
        boxed!(|| get_realistic_env_test(test_name, duration, test_cmd)),
        boxed!(|| get_state_sync_test(test_name)),
        boxed!(|| get_dag_test(test_name, duration, test_cmd)),
        boxed!(|| get_indexer_test(test_name)),
        boxed!(|| get_ungrouped_test(test_name)),
    ];

    for named_suite in named_test_suites.iter() {
        if let Some(suite) = named_suite() {
            return Ok(suite);
        }
    }

    bail!(format_err!("Invalid --suite given: {:?}", test_name))
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
