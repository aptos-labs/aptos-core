// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `aptos governance propose-bundle` and `execute-bundle` against a local swarm,
//! with a two-step governance bundle proposed twice: once executed from the
//! start, and once resumed after the first step was run by other means.

use crate::smoke_test_environment::SwarmBuilder;
use aptos::{governance::bundle::ExecutedStep, test::CliTestFramework};
use aptos_forge::NodeExt;
use aptos_governance_bundle::{
    BundleManifest, BundleSection, SourceSection, BYTECODE_DIR, METADATA_JSON, SUMMARY_DIR,
};
use aptos_rest_client::{aptos_api_types::ViewRequest, Client};
use aptos_temppath::TempPath;
use aptos_types::account_address::AccountAddress;
use serde_json::json;
use std::{
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

/// Voting period of the swarm's governance, in seconds.
const VOTING_DURATION_SECS: u64 = 10;
const VOTING_CLOSE_TIMEOUT: Duration = Duration::from_secs(120);
/// Gas funds for the validator's account (1000 APT).
const GAS_FUNDS_OCTAS: u64 = 100_000_000_000;

#[tokio::test]
async fn test_propose_and_execute_bundle() {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.voting_duration_secs = VOTING_DURATION_SECS;
        }))
        .build_with_cli(0)
        .await;
    let validator = swarm.validators().next().unwrap();
    let validator_idx = cli.add_account_to_cli(
        validator
            .account_private_key()
            .as_ref()
            .unwrap()
            .private_key(),
    );
    let pool_address = cli.account_id(validator_idx);
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
        propose_and_pass(&cli, &client, validator_idx, bundle_path, pool_address).await;
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
        propose_and_pass(&cli, &client, validator_idx, bundle_path, pool_address).await;
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
}

/// Propose the bundle, check it cannot be executed while the vote is open, vote
/// for it, and wait until it can be resolved. Returns the proposal id.
async fn propose_and_pass(
    cli: &CliTestFramework,
    client: &Client,
    validator_idx: usize,
    bundle_path: &Path,
    pool_address: AccountAddress,
) -> u64 {
    let proposal_id = cli
        .propose_bundle(
            validator_idx,
            bundle_path,
            pool_address,
            "https://dummy.invalid/metadata.json",
        )
        .await
        .unwrap()
        .proposal_id
        .unwrap();

    let err = cli
        .execute_bundle(validator_idx, bundle_path, proposal_id)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("still open for voting"), "{}", err);

    cli.vote(validator_idx, proposal_id, true, false, vec![pool_address])
        .await;
    wait_for_voting_closed(client, proposal_id).await;
    proposal_id
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
    let (first_blob, _) = cli.compile_script(&first_source).unwrap();

    fs::create_dir_all(dir.join(BYTECODE_DIR)).unwrap();
    fs::create_dir_all(dir.join(SUMMARY_DIR)).unwrap();
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
    let request = ViewRequest {
        function: format!("0x1::voting::{}", function).parse().unwrap(),
        type_arguments: vec!["0x1::governance_proposal::GovernanceProposal"
            .parse()
            .unwrap()],
        arguments: vec![json!("0x1"), json!(proposal_id.to_string())],
    };
    client
        .view(&request, None)
        .await
        .unwrap()
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
