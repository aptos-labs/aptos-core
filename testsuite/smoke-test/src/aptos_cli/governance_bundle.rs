// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `aptos governance propose-bundle` and `execute-bundle` against a local swarm,
//! with a two-step governance bundle proposed three times: executed from the
//! start, resumed after the first step was run by other means, and proposed
//! from a delegation pool.

use crate::smoke_test_environment::SwarmBuilder;
use aptos::{governance::bundle::ExecutedStep, test::CliTestFramework};
use aptos_crypto::{bls12381, x25519, PrivateKey, Uniform};
use aptos_forge::NodeExt;
use aptos_genesis::config::HostAndPort;
use aptos_governance_bundle::{
    BundleManifest, BundleSection, SourceSection, BYTECODE_DIR, METADATA_JSON, SCRIPTS_DIR,
    SUMMARY_DIR,
};
use aptos_move_cli::MemberId;
use aptos_rest_client::{aptos_api_types::ViewRequest, Client};
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress,
    network_address::{DnsName, NetworkAddress},
    on_chain_config::{OnChainChunkyDKGConfig, OnChainRandomnessConfig},
};
use serde_json::json;
use std::{
    fs,
    path::Path,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

/// Voting period of the swarm's governance, in seconds.
const VOTING_DURATION_SECS: u64 = 10;
/// Epoch length, in seconds; a pool joins the validator set at an epoch change.
const EPOCH_DURATION_SECS: u64 = 5;
const VOTING_CLOSE_TIMEOUT: Duration = Duration::from_secs(120);
const VALIDATOR_ACTIVE_TIMEOUT: Duration = Duration::from_secs(60);
/// Gas funds for an account (1000 APT).
const GAS_FUNDS_OCTAS: u64 = 100_000_000_000;
/// The validator's genesis stake (1,000,000 APT), so the delegation pool that
/// joins later holds too little voting power to matter for consensus.
const VALIDATOR_GENESIS_STAKE_OCTAS: u64 = 100_000_000_000_000;
/// Stake put into the delegation pool (100 APT).
const DELEGATION_STAKE_OCTAS: u64 = 10_000_000_000;
/// `0x1::stake::VALIDATOR_STATUS_ACTIVE`.
const VALIDATOR_STATUS_ACTIVE: u64 = 2;

/// A pool that proposes or votes, and the CLI account signing for it.
struct Pool {
    signer_idx: usize,
    address: AccountAddress,
}

#[tokio::test]
async fn test_propose_and_execute_bundle() {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_genesis_stake(Arc::new(|_, genesis_stake_amount| {
            *genesis_stake_amount = VALIDATOR_GENESIS_STAKE_OCTAS;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.voting_duration_secs = VOTING_DURATION_SECS;
            // The delegation pool round joins the validator set, which needs
            // quick epochs without DKG.
            genesis_config.allow_new_validators = true;
            genesis_config.epoch_duration_secs = EPOCH_DURATION_SECS;
            genesis_config.randomness_config_override =
                Some(OnChainRandomnessConfig::default_disabled());
            genesis_config.chunky_dkg_config_override =
                Some(OnChainChunkyDKGConfig::default_disabled());
        }))
        .build_with_cli(1)
        .await;
    let delegator_idx = 0;
    let validator = swarm.validators().next().unwrap();
    let validator_idx = cli.add_account_to_cli(
        validator
            .account_private_key()
            .as_ref()
            .unwrap()
            .private_key(),
    );
    let validator_pool = Pool {
        signer_idx: validator_idx,
        address: cli.account_id(validator_idx),
    };
    let client = validator.rest_client();
    // The validator's stake is locked; give its account APT to pay for gas.
    cli.fund_account(validator_idx, Some(GAS_FUNDS_OCTAS))
        .await
        .unwrap();
    // The proposer's lockup must outlast the voting period.
    cli.increase_lockup(validator_idx).await.unwrap();

    let bundle_dir = TempPath::new();
    bundle_dir.create_as_dir().unwrap();
    let bundle_path = bundle_dir.path();
    write_bundle(&cli, bundle_path);

    // Round one: execute-bundle runs every step from the start.
    let proposal_id =
        propose_and_pass(&cli, &client, bundle_path, &validator_pool, &validator_pool).await;
    let steps = cli
        .execute_bundle(validator_idx, bundle_path, proposal_id)
        .await
        .unwrap();
    assert_eq!(step_names(&steps), vec!["0-first", "1-last"]);
    assert!(is_resolved(&client, proposal_id).await);

    // Nothing left to execute.
    let err = cli
        .execute_bundle(validator_idx, bundle_path, proposal_id)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("already been fully executed"),
        "{}",
        err
    );

    // Round two: the same bundle again, with the first step run through
    // `execute-proposal` to stand in for an interrupted run. execute-bundle
    // must pick up at the second step.
    let proposal_id =
        propose_and_pass(&cli, &client, bundle_path, &validator_pool, &validator_pool).await;
    cli.execute_proposal_compiled(
        validator_idx,
        proposal_id,
        &bundle_path.join(BYTECODE_DIR).join("0-first.mv"),
    )
    .await
    .unwrap();
    let steps = cli
        .execute_bundle(validator_idx, bundle_path, proposal_id)
        .await
        .unwrap();
    assert_eq!(step_names(&steps), vec!["1-last"]);
    assert!(is_resolved(&client, proposal_id).await);

    // Round three: a delegation pool proposes. Its delegated voter is the pool
    // itself, so a delegator signs, and the proposal goes through
    // `delegation_pool::create_proposal`, which makes the pool the proposer.
    cli.fund_account(delegator_idx, Some(GAS_FUNDS_OCTAS))
        .await
        .unwrap();
    let delegation_pool = Pool {
        signer_idx: delegator_idx,
        address: join_as_delegation_pool(&cli, &client, delegator_idx).await,
    };
    let proposal_id = propose_and_pass(
        &cli,
        &client,
        bundle_path,
        &delegation_pool,
        &validator_pool,
    )
    .await;
    assert_eq!(
        proposer(&client, proposal_id).await,
        delegation_pool.address
    );
    let steps = cli
        .execute_bundle(delegator_idx, bundle_path, proposal_id)
        .await
        .unwrap();
    assert_eq!(step_names(&steps), vec!["0-first", "1-last"]);
    assert!(is_resolved(&client, proposal_id).await);
}

/// Propose the bundle from `proposer`, check it cannot be executed while the
/// vote is open, vote for it from `voter`, and wait until it can be resolved.
/// Returns the proposal id.
async fn propose_and_pass(
    cli: &CliTestFramework,
    client: &Client,
    bundle_path: &Path,
    proposer: &Pool,
    voter: &Pool,
) -> u64 {
    let proposal_id = cli
        .propose_bundle(
            proposer.signer_idx,
            bundle_path,
            proposer.address,
            "https://dummy.invalid/metadata.json",
        )
        .await
        .unwrap()
        .proposal_id
        .unwrap();

    let err = cli
        .execute_bundle(proposer.signer_idx, bundle_path, proposal_id)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("still open for voting"), "{}", err);

    cli.vote(voter.signer_idx, proposal_id, true, false, vec![
        voter.address,
    ])
    .await;
    wait_for_voting_closed(client, proposal_id).await;
    proposal_id
}

/// The account recorded as the proposal's proposer: the voter for a stake pool,
/// the pool itself for a delegation pool.
async fn proposer(client: &Client, proposal_id: u64) -> AccountAddress {
    proposal_view(client, "get_proposer", proposal_id)
        .await
        .as_str()
        .unwrap()
        .parse()
        .unwrap()
}

/// Create a delegation pool owned, operated, and staked by the CLI account, and
/// join the validator set with it. Returns the pool address once the pool is an
/// active validator, which is also when it gets a lockup.
async fn join_as_delegation_pool(
    cli: &CliTestFramework,
    client: &Client,
    owner_idx: usize,
) -> AccountAddress {
    let owner = cli.account_id(owner_idx);
    run_entry_function(
        cli,
        owner_idx,
        "0x1::delegation_pool::initialize_delegation_pool",
        vec!["u64:0", "hex:0x00"],
    )
    .await;
    let pool_address: AccountAddress = view(
        client,
        "0x1::delegation_pool::get_owned_pool_address",
        vec![],
        vec![json!(owner.to_hex_literal())],
    )
    .await
    .as_str()
    .unwrap()
    .parse()
    .unwrap();
    let pool_arg = format!("address:{}", pool_address.to_hex_literal());
    run_entry_function(cli, owner_idx, "0x1::delegation_pool::add_stake", vec![
        &pool_arg,
        &format!("u64:{}", DELEGATION_STAKE_OCTAS),
    ])
    .await;

    let mut rng = rand::thread_rng();
    let consensus_key = bls12381::PrivateKey::generate(&mut rng);
    let proof_of_possession = bls12381::ProofOfPossession::create(&consensus_key);
    run_entry_function(cli, owner_idx, "0x1::stake::rotate_consensus_key", vec![
        &pool_arg,
        &hex_arg(&consensus_key.public_key().to_bytes()),
        &hex_arg(&proof_of_possession.to_bytes()),
    ])
    .await;
    let network_key = x25519::PrivateKey::generate(&mut rng);
    let network_address = HostAndPort {
        host: DnsName::try_from("127.0.0.1".to_string()).unwrap(),
        port: 6180,
    }
    .as_network_address(network_key.public_key())
    .unwrap();
    run_entry_function(
        cli,
        owner_idx,
        "0x1::stake::update_network_and_fullnode_addresses",
        vec![
            &pool_arg,
            &hex_arg(&bcs::to_bytes(&vec![network_address]).unwrap()),
            &hex_arg(&bcs::to_bytes(&Vec::<NetworkAddress>::new()).unwrap()),
        ],
    )
    .await;
    run_entry_function(cli, owner_idx, "0x1::stake::join_validator_set", vec![
        &pool_arg,
    ])
    .await;

    let deadline = Instant::now() + VALIDATOR_ACTIVE_TIMEOUT;
    while view(client, "0x1::stake::get_validator_state", vec![], vec![
        json!(pool_address.to_hex_literal()),
    ])
    .await
        != json!(VALIDATOR_STATUS_ACTIVE.to_string())
    {
        assert!(
            Instant::now() < deadline,
            "delegation pool {} did not become an active validator in time",
            pool_address
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    pool_address
}

async fn run_entry_function(cli: &CliTestFramework, index: usize, function: &str, args: Vec<&str>) {
    cli.run_function(
        index,
        None,
        MemberId::from_str(function).unwrap(),
        args,
        vec![],
    )
    .await
    .unwrap_or_else(|err| panic!("{} failed: {}", function, err));
}

fn hex_arg(bytes: &[u8]) -> String {
    format!("hex:0x{}", hex::encode(bytes))
}

fn step_names(steps: &[ExecutedStep]) -> Vec<&str> {
    steps.iter().map(|step| step.script.as_str()).collect()
}

async fn is_resolved(client: &Client, proposal_id: u64) -> bool {
    proposal_view(client, "is_resolved", proposal_id)
        .await
        .as_bool()
        .unwrap()
}

/// Write a two-step bundle at `dir`: the first step only hands off to the
/// second, which completes the proposal.
fn write_bundle(cli: &CliTestFramework, dir: &Path) {
    let last_source = r#"script {
    use aptos_framework::aptos_governance;
    use std::vector;

    fun main(proposal_id: u64) {
        let _framework_signer = aptos_governance::resolve_multi_step_proposal(proposal_id, @0x1, vector::empty<u8>());
    }
}
"#;
    let (last_blob, last_hash) = cli.compile_script(last_source).unwrap();

    let first_source = format!(
        r#"script {{
    use aptos_framework::aptos_governance;

    fun main(proposal_id: u64) {{
        let _framework_signer = aptos_governance::resolve_multi_step_proposal(proposal_id, @0x1, x"{}");
    }}
}}
"#,
        last_hash.to_hex()
    );
    let (first_blob, first_hash) = cli.compile_script(&first_source).unwrap();

    fs::create_dir_all(dir.join(SCRIPTS_DIR)).unwrap();
    fs::create_dir_all(dir.join(BYTECODE_DIR)).unwrap();
    fs::create_dir_all(dir.join(SUMMARY_DIR)).unwrap();
    // Sources are stamped with their execution hashes, as generate-bundle does.
    fs::write(
        dir.join(SCRIPTS_DIR).join("0-first.move"),
        format!("// Script hash: {}\n{}", first_hash.to_hex(), first_source),
    )
    .unwrap();
    fs::write(
        dir.join(SCRIPTS_DIR).join("1-last.move"),
        format!("// Script hash: {}\n{}", last_hash.to_hex(), last_source),
    )
    .unwrap();
    fs::write(dir.join(BYTECODE_DIR).join("0-first.mv"), first_blob).unwrap();
    fs::write(dir.join(BYTECODE_DIR).join("1-last.mv"), last_blob).unwrap();
    fs::write(
        dir.join(METADATA_JSON),
        json!({
            "title": "Smoke test bundle",
            "description": "A two-step proposal that changes nothing.",
            "source_code_url": "https://github.com/aptos-labs/aptos-core",
            "discussion_url": "https://github.com/aptos-labs/aptos-core",
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        dir.join(SUMMARY_DIR).join("changes.md"),
        "- [x] Reviewed.\n",
    )
    .unwrap();

    BundleManifest::new(
        dir,
        BundleSection {
            name: "smoke-test".to_string(),
            created_at: "1970-01-01T00:00:00Z".to_string(),
        },
        SourceSection {
            branch: None,
            commit: "0".repeat(40),
        },
    )
    .unwrap()
    .write(dir)
    .unwrap();
}

/// Call a `0x1::voting` view function on a governance proposal.
async fn proposal_view(client: &Client, function: &str, proposal_id: u64) -> serde_json::Value {
    view(
        client,
        &format!("0x1::voting::{}", function),
        vec!["0x1::governance_proposal::GovernanceProposal"],
        vec![json!("0x1"), json!(proposal_id.to_string())],
    )
    .await
}

/// Call a view function and return its single result.
async fn view(
    client: &Client,
    function: &str,
    type_arguments: Vec<&str>,
    arguments: Vec<serde_json::Value>,
) -> serde_json::Value {
    let request = ViewRequest {
        function: function.parse().unwrap(),
        type_arguments: type_arguments
            .into_iter()
            .map(|t| t.parse().unwrap())
            .collect(),
        arguments,
    };
    client
        .view(&request, None)
        .await
        .unwrap_or_else(|err| panic!("{} failed: {}", function, err))
        .into_inner()
        .remove(0)
}

async fn ledger_timestamp_secs(client: &Client) -> u64 {
    client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .timestamp_usecs
        / 1_000_000
}

/// Wait until the proposal can be resolved: voting is closed, and on-chain time
/// has moved strictly past the last vote.
async fn wait_for_voting_closed(client: &Client, proposal_id: u64) {
    let deadline = Instant::now() + VOTING_CLOSE_TIMEOUT;
    while !proposal_view(client, "is_voting_closed", proposal_id)
        .await
        .as_bool()
        .unwrap()
    {
        assert!(
            Instant::now() < deadline,
            "voting on proposal {} did not close in time",
            proposal_id
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let closed_at = ledger_timestamp_secs(client).await;
    while ledger_timestamp_secs(client).await <= closed_at {
        assert!(
            Instant::now() < deadline,
            "the on-chain clock did not advance past the vote in time"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
