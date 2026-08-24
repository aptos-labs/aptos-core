// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `deploy-testnet`: execute a bundle's governance proposal on a test network.
//!
//! Drives the full test-network governance ceremony using the bundle's scripts
//! exactly as reviewed -- nothing is regenerated:
//!
//! 1. `verify-bundle`, with sign-off required by default -- this includes
//!    checking each compiled script in `bytecode/` against the execution hash
//!    stamped on its source, so what was audited is what gets deployed;
//! 2. refuse to run against mainnet (chain id check);
//! 3. load the bundle's compiled scripts: the exact bytes the multi-step
//!    hash chain approves, no recompilation involved;
//! 4. simulate those same bytes against the target network (skippable);
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
use move_model::metadata::{CompilerVersion, LanguageVersion};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use url::Url;

/// Voting period while the proposal goes through, in seconds.
const FAST_RESOLUTION_SECS: u64 = 30;

/// Governance transactions can exceed the simulation-based gas estimate; use a
/// fixed generous cap like the previous tooling did.
const MAX_GAS: u64 = 2_000_000;
const GAS_UNIT_PRICE: u64 = 100;

/// Octas minted to the validator with --mint-to-validator (1000 APT).
const MINT_AMOUNT: u64 = 100_000_000_000;

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

/// A compiled script loaded from the bundle's `bytecode/` directory.
struct BundleScript {
    name: String,
    blob: Vec<u8>,
    /// The script's on-chain execution hash: the hash of `blob`.
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
    mint_to_validator: bool,
    skip_signoff: bool,
    skip_simulation: bool,
    dry_run: bool,
) -> Result<()> {
    // 1. Bundle integrity, and by default the full sign-off.
    verify::run(bundle_path, !skip_signoff)?;

    // The metadata location recorded on-chain is informational only; nothing
    // fetches it. The on-chain limit is 256 bytes; check here rather than
    // fail mid-ceremony.
    if metadata_url.len() > 256 {
        bail!(
            "metadata URL exceeds the on-chain 256-byte limit ({} bytes): {}",
            metadata_url.len(),
            metadata_url
        );
    }

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

    // 3. Load the bundle's compiled scripts (verify-bundle above checked them
    //    against the stamped execution hashes and the manifest checksums).
    let scripts = load_bundle_scripts(bundle_path)?;
    println!(
        "Loaded {} compiled script(s) from the bundle.",
        scripts.len()
    );

    // 4. Simulate the exact bytes to be submitted against the target network.
    if skip_simulation {
        println!("WARNING: skipping simulation (--skip-simulation)");
    } else {
        let named_blobs: Vec<(String, Vec<u8>)> = scripts
            .iter()
            .map(|s| (s.name.clone(), s.blob.clone()))
            .collect();
        aptos_release_builder::simulate::simulate_compiled_scripts(
            endpoint.clone(),
            &bundle_path.join("gas-profiling"), // unused: gas profiling is off
            &named_blobs,
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

    // Fund the validator's gas on throwaway networks (local swarms, forge).
    // Refused on testnet: its validator is expected to already be funded.
    if mint_to_validator {
        if chain_id == ChainId::testnet().id() {
            bail!("--mint-to-validator is not allowed on testnet");
        }
        submit(
            &client,
            &factory,
            &root,
            aptos_stdlib::aptos_coin_mint(validator.address(), MINT_AMOUNT),
        )
        .await
        .context("failed to mint gas funds to the validator")?;
        println!("Minted {} octas to the validator.", MINT_AMOUNT);
    }

    // Capture the network's governance config so the ceremony changes only
    // the voting period and the restore puts back exactly what was
    // configured, not assumed defaults.
    let original_config = get_governance_config(&client)
        .await
        .context("failed to read the current governance config")?;
    set_governance_config(&client, &factory, &root, core_path, GovernanceConfig {
        voting_duration_secs: FAST_RESOLUTION_SECS,
        ..original_config
    })
    .await?;
    let outcome = run_governance(
        &client,
        &factory,
        &validator,
        &scripts,
        bundle_path,
        metadata_url,
    )
    .await;
    let restore = set_governance_config(&client, &factory, &root, core_path, original_config)
        .await
        .with_context(|| {
            format!(
                "failed to restore the governance config -- restore it manually: \
                 min voting threshold {}, required proposer stake {}, voting period {}s",
                original_config.min_voting_threshold,
                original_config.required_proposer_stake,
                original_config.voting_duration_secs
            )
        });

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
    scripts: &[BundleScript],
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

    // Seed the approved-hash entry once, so an oversized first script clears
    // the mempool size limit. Each resolve_multi_step_proposal call rolls the
    // entry forward to the next script's hash on-chain, and the final step
    // removes it -- no per-step approval is needed.
    submit(
        client,
        factory,
        validator,
        aptos_stdlib::aptos_governance_add_approved_script_hash_script(proposal_id),
    )
    .await
    .with_context(|| {
        format!(
            "failed to approve the script hash for proposal {}",
            proposal_id
        )
    })?;

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
            TransactionPayload::Script(Script::new(script.blob.clone(), vec![], vec![
                TransactionArgument::U64(proposal_id),
            ])),
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

/// The on-chain governance config: everything the ceremony touches and must
/// put back exactly.
#[derive(Clone, Copy)]
struct GovernanceConfig {
    min_voting_threshold: u128,
    required_proposer_stake: u64,
    voting_duration_secs: u64,
}

/// The network's currently configured governance config.
async fn get_governance_config(client: &Client) -> Result<GovernanceConfig> {
    let resource = client
        .get_account_resource(
            AccountAddress::ONE,
            "0x1::aptos_governance::GovernanceConfig",
        )
        .await
        .context("failed to read the governance config")?
        .into_inner()
        .ok_or_else(|| anyhow!("no governance config on chain"))?;
    let field = |name: &str| {
        resource.data[name]
            .as_str()
            .with_context(|| format!("unexpected governance config field: {}", name))
    };
    Ok(GovernanceConfig {
        min_voting_threshold: field("min_voting_threshold")?.parse()?,
        required_proposer_stake: field("required_proposer_stake")?.parse()?,
        voting_duration_secs: field("voting_duration_secs")?.parse()?,
    })
}

/// Set the governance config, signed by root. Test networks only: the
/// script leans on `aptos_governance::get_signer_testnet_only`.
async fn set_governance_config(
    client: &Client,
    factory: &TransactionFactory,
    root: &LocalAccount,
    core_path: &Path,
    config: GovernanceConfig,
) -> Result<()> {
    println!(
        "Setting the governance config: min voting threshold {}, required proposer stake {}, \
         voting period {}s...",
        config.min_voting_threshold, config.required_proposer_stake, config.voting_duration_secs
    );
    let source = format!(
        r#"
script {{
    use aptos_framework::aptos_governance;

    fun main(core_resources: &signer) {{
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        aptos_governance::update_governance_config(&core_signer, {}, {}, {});
    }}
}}
"#,
        config.min_voting_threshold, config.required_proposer_stake, config.voting_duration_secs
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
    .context("failed to set the governance config")?;
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
            break;
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

    // Voting can close in the very second the deciding vote lands (early
    // resolution), and resolving asserts on-chain time is strictly past the
    // last vote; wait for the on-chain clock to tick over.
    let closed_at_secs = ledger_timestamp_secs(client).await?;
    while ledger_timestamp_secs(client).await? <= closed_at_secs {
        if start.elapsed() > VOTING_CLOSE_TIMEOUT {
            bail!(
                "the on-chain clock did not advance past the vote within {:?}",
                VOTING_CLOSE_TIMEOUT
            );
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
    Ok(())
}

/// The latest on-chain timestamp, in seconds.
async fn ledger_timestamp_secs(client: &Client) -> Result<u64> {
    Ok(client
        .get_ledger_information()
        .await
        .context("failed to read the ledger timestamp")?
        .into_inner()
        .timestamp_usecs
        / 1_000_000)
}

/// Compile every script in the bundle (in execution order) and require each to
/// match the execution hash stamped at generation time. The compiled blobs are
/// what later gets submitted, so what we validate is what we deploy.
fn load_bundle_scripts(bundle_path: &Path) -> Result<Vec<BundleScript>> {
    Ok(crate::bundle::load_compiled_scripts(bundle_path)?
        .into_iter()
        .map(|(name, blob)| {
            let hash = HashValue::sha3_256_of(&blob);
            BundleScript { name, blob, hash }
        })
        .collect())
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
        // The three compile options below must mirror CompileScriptFunction's
        // fallbacks (the path generation stamps hashes through): the stamped
        // hashes and the embedded next-execution-hash chain are computed from
        // bytecode compiled with these exact settings, and on-chain execution
        // compares the submitted blob's hash against that chain.
        None,
        Some(LanguageVersion::latest_stable()),
        Some(CompilerVersion::latest_stable()),
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
