// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `deploy-testnet`: execute a bundle's governance proposal on a test network.
//!
//! Drives the full test-network governance ceremony using the bundle's scripts
//! exactly as reviewed -- nothing is regenerated:
//!
//! 1. `verify-bundle`, with sign-off required by default;
//! 2. refuse to run against mainnet (chain id check);
//! 3. compile the bundle's scripts and check each against its stamped
//!    execution hash: a mismatch means the local toolchain no longer
//!    reproduces the bytecode the multi-step hash chain approves, which would
//!    otherwise strand the proposal midway through execution;
//! 4. simulate the proposal against the target network (skippable);
//! 5. shrink the voting period (root, `get_signer_testnet_only`), propose and
//!    vote with the validator, wait for voting to close, then execute each
//!    step -- restoring the voting period even when execution fails.
//!
//! The root account signs only the voting-period changes. Proposing, voting,
//! hash approval, and execution are all signed by the validator; execution is
//! permissionless on-chain, the validator is just a convenient funded sender.

use crate::commands::verify;
use anyhow::{anyhow, bail, Context, Result};
use aptos_cached_packages::aptos_stdlib;
use aptos_cli_common::PromptOptions;
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, ValidCryptoMaterialStringExt};
use aptos_move_cli::{compile_in_temp_dir, FrameworkPackageArgs};
use aptos_rest_client::{aptos_api_types::ViewRequest, AptosBaseUrl, Client, Transaction};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::{
    account_address::AccountAddress,
    account_config::aptos_test_root_address,
    chain_id::ChainId,
    transaction::{Script, TransactionArgument, TransactionPayload},
};
use clap::Parser;
use move_core_types::diag_writer::DiagWriter;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use url::Url;

/// Voting period while the proposal goes through, in seconds.
const FAST_RESOLUTION_SECS: u64 = 30;
/// The regular test-network voting period, restored afterwards.
const DEFAULT_RESOLUTION_SECS: u64 = 43200;

/// Governance transactions can exceed the simulation-based gas estimate; use a
/// fixed generous cap like the previous tooling did.
const MAX_GAS: u64 = 2_000_000;
const GAS_UNIT_PRICE: u64 = 100;

/// How long to wait for voting to close after the fast-resolve window.
const VOTING_CLOSE_TIMEOUT: Duration = Duration::from_secs(180);
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Everything needed to sign the ceremony's transactions.
pub struct Signers {
    /// Hex-encoded root (core resources) private key.
    pub root_key: String,
    /// The validator's stake pool address, used to propose and vote.
    pub validator_address: AccountAddress,
    /// Hex-encoded private key of the validator's voter account.
    pub validator_key: String,
}

/// A bundle script compiled against the local framework.
struct CompiledScript {
    name: String,
    blob: Vec<u8>,
    hash: HashValue,
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    bundle_path: &Path,
    endpoint: Url,
    signers: Signers,
    node_api_key: Option<String>,
    metadata_url: &str,
    core_path: &Path,
    skip_signoff: bool,
    skip_simulation: bool,
    dry_run: bool,
) -> Result<()> {
    // 1. Bundle integrity, and by default the full sign-off.
    verify::run(bundle_path, !skip_signoff)?;

    // 2. Network guard: never mainnet.
    let client = build_client(endpoint.clone(), node_api_key.as_deref())?;
    let chain_id = client
        .get_ledger_information()
        .await
        .context("failed to reach the network endpoint")?
        .into_inner()
        .chain_id;
    if chain_id == ChainId::mainnet().id() {
        bail!("refusing to deploy to mainnet: this command drives the test-network-only governance flow");
    }

    // 3. Compile the bundle's scripts and check the stamped execution hashes.
    let scripts = compile_bundle_scripts(bundle_path, core_path)?;
    println!(
        "Compiled {} script(s); all match their stamped execution hashes.",
        scripts.len()
    );

    // 4. Simulate against the target network before touching it.
    if skip_simulation {
        println!("WARNING: skipping simulation (--skip-simulation)");
    } else {
        aptos_release_builder::simulate::simulate_all_proposals(
            endpoint.clone(),
            bundle_path,
            false,
            node_api_key.clone(),
        )
        .await
        .context("simulation failed; refusing to deploy")?;
    }

    if dry_run {
        println!("Dry run: all preflight checks passed; not submitting anything.");
        return Ok(());
    }

    // 5. The ceremony, with the voting period restored no matter what.
    let root_key = Ed25519PrivateKey::from_encoded_string(&signers.root_key)
        .map_err(|e| anyhow!("invalid root key: {}", e))?;
    let validator_key = Ed25519PrivateKey::from_encoded_string(&signers.validator_key)
        .map_err(|e| anyhow!("invalid validator key: {}", e))?;
    let root = local_account(&client, aptos_test_root_address(), root_key).await?;
    let validator = local_account(&client, signers.validator_address, validator_key).await?;
    let factory = TransactionFactory::new(ChainId::new(chain_id))
        .with_max_gas_amount(MAX_GAS)
        .with_gas_unit_price(GAS_UNIT_PRICE);

    set_resolution_time(&client, &factory, &root, core_path, FAST_RESOLUTION_SECS).await?;
    let outcome = run_governance(
        &client,
        &factory,
        &validator,
        &scripts,
        bundle_path,
        metadata_url,
    )
    .await;
    let restore = set_resolution_time(&client, &factory, &root, core_path, DEFAULT_RESOLUTION_SECS)
        .await
        .context("failed to restore the voting period -- restore it manually");

    match (outcome, restore) {
        (Ok(proposal_id), Ok(())) => {
            println!(
                "Deployment complete: proposal {} fully executed ({} scripts).",
                proposal_id,
                scripts.len()
            );
            println!("Next: run verify-framework-deployment against this network.");
            Ok(())
        },
        (Ok(_), Err(e)) => Err(e),
        (Err(e), Ok(())) => Err(e),
        (Err(e), Err(restore_err)) => Err(e.context(format!("additionally: {:#}", restore_err))),
    }
}

/// Propose, vote, and execute every script. Returns the proposal id. Errors
/// carry enough state (proposal id, executed step count) for manual recovery.
async fn run_governance(
    client: &Client,
    factory: &TransactionFactory,
    validator: &LocalAccount,
    scripts: &[CompiledScript],
    bundle_path: &Path,
    metadata_url: &str,
) -> Result<u64> {
    submit(
        client,
        factory,
        validator,
        aptos_stdlib::stake_increase_lockup(),
    )
    .await
    .context("failed to increase the validator's lockup")?;

    let metadata = fs::read(bundle_path.join(crate::bundle::METADATA_JSON))
        .context("failed to read the bundle's metadata.json")?;
    let first = scripts
        .first()
        .ok_or_else(|| anyhow!("bundle has no scripts"))?;
    let txn = submit(
        client,
        factory,
        validator,
        aptos_stdlib::aptos_governance_create_proposal_v2(
            validator.address(),
            first.hash.to_vec(),
            metadata_url.as_bytes().to_vec(),
            HashValue::sha3_256_of(&metadata).to_hex().into_bytes(),
            true,
        ),
    )
    .await
    .context("failed to create the governance proposal")?;
    let proposal_id = extract_proposal_id(&txn)?;
    println!("Created proposal {}", proposal_id);

    submit(
        client,
        factory,
        validator,
        aptos_stdlib::aptos_governance_vote(validator.address(), proposal_id, true),
    )
    .await
    .with_context(|| format!("failed to vote on proposal {}", proposal_id))?;
    println!("Voted; waiting for voting to close...");

    wait_for_voting_closed(client, proposal_id).await?;

    for (executed, script) in scripts.iter().enumerate() {
        let state = || {
            format!(
                "proposal {}: executed {}/{} scripts, failed at {}",
                proposal_id,
                executed,
                scripts.len(),
                script.name
            )
        };
        submit(
            client,
            factory,
            validator,
            aptos_stdlib::aptos_governance_add_approved_script_hash_script(proposal_id),
        )
        .await
        .with_context(|| format!("failed to approve the script hash -- {}", state()))?;
        submit(
            client,
            factory,
            validator,
            TransactionPayload::Script(Script::new(
                script.blob.clone(),
                vec![],
                vec![TransactionArgument::U64(proposal_id)],
            )),
        )
        .await
        .with_context(|| format!("failed to execute the script -- {}", state()))?;
        println!(
            "Executed {} ({}/{})",
            script.name,
            executed + 1,
            scripts.len()
        );
    }

    Ok(proposal_id)
}

fn build_client(endpoint: Url, node_api_key: Option<&str>) -> Result<Client> {
    let mut builder = Client::builder(AptosBaseUrl::Custom(endpoint));
    if let Some(key) = node_api_key {
        builder = builder.api_key(key)?;
    }
    Ok(builder.build())
}

async fn local_account(
    client: &Client,
    address: AccountAddress,
    key: Ed25519PrivateKey,
) -> Result<LocalAccount> {
    let sequence_number = client
        .get_account(address)
        .await
        .with_context(|| format!("failed to look up account {}", address))?
        .into_inner()
        .sequence_number;
    Ok(LocalAccount::new(address, key, sequence_number))
}

/// Sign `payload` with `account`, submit it, and wait for on-chain success.
async fn submit(
    client: &Client,
    factory: &TransactionFactory,
    account: &LocalAccount,
    payload: TransactionPayload,
) -> Result<Transaction> {
    let signed = account.sign_with_transaction_builder(factory.payload(payload));
    let txn = client.submit_and_wait(&signed).await?.into_inner();
    if !txn.success() {
        bail!("transaction committed but failed: {}", txn.vm_status());
    }
    Ok(txn)
}

/// Set the governance voting period, signed by root. Test networks only: the
/// script leans on `aptos_governance::get_signer_testnet_only`.
async fn set_resolution_time(
    client: &Client,
    factory: &TransactionFactory,
    root: &LocalAccount,
    core_path: &Path,
    seconds: u64,
) -> Result<()> {
    println!("Setting the voting period to {}s...", seconds);
    let source = format!(
        r#"
script {{
    use aptos_framework::aptos_governance;

    fun main(core_resources: &signer) {{
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        aptos_governance::update_governance_config(&core_signer, 0, 0, {});
    }}
}}
"#,
        seconds
    );
    let temp = aptos_temppath::TempPath::new();
    temp.create_as_file()?;
    let mut path = temp.path().to_path_buf();
    path.set_extension("move");
    fs::write(&path, source)?;

    let (blob, _) = compile_script(&path, core_path)?;
    submit(
        client,
        factory,
        root,
        TransactionPayload::Script(Script::new(blob, vec![], vec![])),
    )
    .await
    .with_context(|| format!("failed to set the voting period to {}s", seconds))?;
    Ok(())
}

/// Poll `0x1::voting::is_voting_closed` until the proposal can be resolved.
async fn wait_for_voting_closed(client: &Client, proposal_id: u64) -> Result<()> {
    let request = ViewRequest {
        function: "0x1::voting::is_voting_closed"
            .parse()
            .map_err(|e| anyhow!("bad view function id: {:#}", e))?,
        type_arguments: vec!["0x1::governance_proposal::GovernanceProposal"
            .parse()
            .map_err(|e| anyhow!("bad view type argument: {:#}", e))?],
        arguments: vec![
            serde_json::json!("0x1"),
            serde_json::json!(proposal_id.to_string()),
        ],
    };

    let start = Instant::now();
    loop {
        let closed = client
            .view(&request, None)
            .await
            .context("failed to poll the proposal's voting state")?
            .into_inner()
            .first()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if closed {
            return Ok(());
        }
        if start.elapsed() > VOTING_CLOSE_TIMEOUT {
            bail!(
                "voting on proposal {} did not close within {:?}; \
                 the vote is cast -- resume manually once it closes",
                proposal_id,
                VOTING_CLOSE_TIMEOUT
            );
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Compile every script in the bundle (in execution order) and require each to
/// match the execution hash stamped at generation time. The compiled blobs are
/// what later gets submitted, so what we validate is what we deploy.
fn compile_bundle_scripts(bundle_path: &Path, core_path: &Path) -> Result<Vec<CompiledScript>> {
    let scripts_dir = bundle_path.join(crate::bundle::SCRIPTS_DIR);
    let mut paths: Vec<PathBuf> = fs::read_dir(&scripts_dir)
        .with_context(|| format!("failed to read {}", scripts_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "move").unwrap_or(false))
        .collect();
    paths.sort();
    if paths.is_empty() {
        bail!("no scripts found in {}", scripts_dir.display());
    }

    let mut compiled = vec![];
    let mut errors = vec![];
    for path in &paths {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let source =
            fs::read_to_string(path).with_context(|| format!("failed to read {}", name))?;
        let Some(stamped) = stamped_hash(&source) else {
            bail!(
                "{} has no stamped execution hash (expected a leading \
                 '// Script hash: ...' comment, added by generate-bundle)",
                name
            );
        };
        let (blob, hash) = compile_script(path, core_path)
            .with_context(|| format!("failed to compile {}", name))?;
        if hash.to_hex().to_lowercase() != stamped {
            errors.push(format!(
                "{}: compiled hash {} does not match stamped hash {}",
                name,
                hash.to_hex(),
                stamped
            ));
        }
        compiled.push(CompiledScript { name, blob, hash });
    }
    if !errors.is_empty() {
        bail!(
            "the local toolchain no longer reproduces the bundle's approved \
             bytecode; deploying would strand the proposal mid-execution.\n  - {}\n\
             Run from the bundle's recorded source commit (see bundle.toml).",
            errors.join("\n  - ")
        );
    }
    Ok(compiled)
}

/// The execution hash stamped as the script's first line by generate-bundle.
fn stamped_hash(source: &str) -> Option<String> {
    source
        .lines()
        .next()?
        .strip_prefix("// Script hash: ")
        .map(|s| s.trim().trim_start_matches("0x").to_lowercase())
}

fn compile_script(path: &Path, core_path: &Path) -> Result<(Vec<u8>, HashValue)> {
    let framework_dir = core_path.join("aptos-move/framework/aptos-framework");
    let framework_package_args = FrameworkPackageArgs::try_parse_from([
        "aptos-release-tool",
        "--framework-local-dir",
        &framework_dir.to_string_lossy(),
        "--skip-fetch-latest-git-deps",
    ])
    .context("failed to build framework package args; this should not happen")?;
    let (blob, hash) = compile_in_temp_dir(
        &DiagWriter::stderr(),
        "script",
        path,
        &framework_package_args,
        PromptOptions::yes(),
        None, // bytecode_version
        None, // language_version
        None, // compiler_version
    )
    .map_err(|e| anyhow!("{:#}", e))?;
    Ok((blob, hash))
}

/// Pull the proposal id out of the create-proposal transaction's events.
fn extract_proposal_id(txn: &Transaction) -> Result<u64> {
    let Transaction::UserTransaction(inner) = txn else {
        bail!("create-proposal did not commit as a user transaction");
    };
    inner
        .events
        .iter()
        .find(|event| {
            let typ = event.typ.to_string();
            typ == "0x1::aptos_governance::CreateProposal"
                || typ == "0x1::aptos_governance::CreateProposalEvent"
        })
        .and_then(|event| event.data.get("proposal_id"))
        .and_then(|id| id.as_str())
        .and_then(|id| id.parse::<u64>().ok())
        .ok_or_else(|| anyhow!("no CreateProposal event found in the proposal transaction"))
}

#[cfg(test)]
mod tests {
    use super::stamped_hash;

    #[test]
    fn stamped_hash_parses_the_leading_comment() {
        let source = "// Script hash: 0xAB12ef\nscript { fun main() {} }";
        assert_eq!(stamped_hash(source), Some("ab12ef".to_string()));

        let unstamped = "script { fun main() {} }";
        assert_eq!(stamped_hash(unstamped), None);
    }
}
