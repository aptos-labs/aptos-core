// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `propose-bundle` and `execute-bundle`: drive a governance bundle's proposal
//! on a live network.

use crate::{
    common::{
        types::{
            CliError, CliTypedResult, PoolAddressArgs, TransactionOptions, TransactionOptionsExt,
            TransactionSummary,
        },
        utils::{prompt_yes_with_override, read_from_file},
    },
    governance::{
        create_proposal, execution_payload, get_metadata_from_url, parse_proposal_metadata,
        ProposalSubmissionSummary,
    },
    CliCommand,
};
use aptos_api_types::{Address, HashValue as ApiHashValue, ViewFunction, U64};
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::HashValue;
use aptos_governance_bundle::{self as bundle, verify};
use aptos_rest_client::{Client, Transaction};
use aptos_types::{
    account_address::AccountAddress, on_chain_config::ApprovedExecutionHashes,
    transaction::TransactionPayload,
};
use async_trait::async_trait;
use clap::Parser;
use futures::try_join;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    parser::parse_type_tag,
};
use reqwest::Url;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::PathBuf;

/// `0x1::voting::PROPOSAL_STATE_*`.
const PROPOSAL_STATE_PENDING: u64 = 0;
const PROPOSAL_STATE_SUCCEEDED: u64 = 1;
const PROPOSAL_STATE_FAILED: u64 = 3;

/// Which bundle to use and how strictly to check it.
#[derive(Parser)]
pub struct BundleArgs {
    /// Path to the governance bundle directory
    #[clap(long)]
    pub(crate) bundle: PathBuf,

    /// Do not require the bundle's sign-off checkboxes to be ticked
    #[clap(long)]
    pub(crate) skip_signoff: bool,
}

/// A compiled script from the bundle's `bytecode/` directory.
struct BundleScript {
    name: String,
    blob: Vec<u8>,
    /// The script's on-chain execution hash: the hash of `blob`.
    hash: HashValue,
}

impl BundleArgs {
    /// Verify the bundle, requiring sign-off unless `--skip-signoff`, and load
    /// its compiled scripts in execution order.
    fn load(&self) -> CliTypedResult<Vec<BundleScript>> {
        verify::verify(&self.bundle, !self.skip_signoff).map_err(bundle_error)?;

        let scripts = bundle::load_compiled_scripts(&self.bundle)
            .map_err(bundle_error)?
            .into_iter()
            .map(|(name, blob)| {
                let hash = HashValue::sha3_256_of(&blob);
                BundleScript { name, blob, hash }
            })
            .collect();
        Ok(scripts)
    }

    /// The bundle's `metadata.json`, byte for byte.
    fn read_metadata(&self) -> CliTypedResult<Vec<u8>> {
        read_from_file(&self.bundle.join(bundle::METADATA_JSON))
    }
}

fn bundle_error(err: anyhow::Error) -> CliError {
    CliError::CommandArgumentError(format!("{:#}", err))
}

/// Submit a governance bundle's proposal
///
/// Verifies the bundle, checks that the metadata URL serves the bundle's
/// `metadata.json`, checks that the stake pool may propose, and creates a
/// multi-step proposal for the bundle's compiled scripts. Nothing is compiled.
#[derive(Parser)]
pub struct ProposeBundle {
    #[clap(flatten)]
    pub(crate) bundle_args: BundleArgs,
    #[clap(flatten)]
    pub(crate) pool_address_args: PoolAddressArgs,

    /// URL where the bundle's `metadata.json` is published
    ///
    /// Recorded on-chain so voters can read the proposal's title and description.
    /// It must serve exactly the bundle's `metadata.json`.
    #[clap(long)]
    pub(crate) metadata_url: Url,

    /// Do not check that the metadata URL serves the bundle's `metadata.json`
    #[clap(long)]
    pub(crate) skip_metadata_url_check: bool,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<ProposalSubmissionSummary> for ProposeBundle {
    fn command_name(&self) -> &'static str {
        "ProposeBundle"
    }

    async fn execute(mut self) -> CliTypedResult<ProposalSubmissionSummary> {
        let scripts = self.bundle_args.load()?;

        let metadata_bytes = self.bundle_args.read_metadata()?;
        let metadata = parse_proposal_metadata(&metadata_bytes)?;
        if !self.skip_metadata_url_check {
            let served = get_metadata_from_url(&self.metadata_url).await?;
            if served != metadata_bytes {
                return Err(CliError::CommandArgumentError(format!(
                    "{} does not serve the bundle's {} byte for byte; publish the bundle \
                     there first, or pass --skip-metadata-url-check",
                    self.metadata_url,
                    bundle::METADATA_JSON
                )));
            }
        }
        let metadata_hash = HashValue::sha3_256_of(&metadata_bytes);

        let pool_address = self.pool_address_args.pool_address;
        let client = self.txn_options.rest_client()?;
        let (_, sender) = self.txn_options.get_public_key_and_address()?;
        check_proposer_eligibility(&client, sender, pool_address).await?;

        println!(
            "{}\n\tMetadata Hash: {}\n\tScript Hash: {}\n\tSteps: {}",
            metadata,
            metadata_hash.to_hex(),
            scripts[0].hash.to_hex(),
            scripts.len()
        );
        prompt_yes_with_override(
            "Do you want to submit this proposal?",
            self.txn_options.prompt_options,
        )?;

        create_proposal(
            &self.txn_options,
            pool_address,
            scripts[0].hash,
            &self.metadata_url,
            metadata_hash,
            true,
        )
        .await
    }
}

/// Fail early, with a readable message, on the conditions `create_proposal_v2`
/// checks on-chain.
/// - The sender is the stake pool's delegated voter.
/// - The stake pool has the required proposer stake.
/// - The stake pool's lockup outlasts the voting period.
async fn check_proposer_eligibility(
    client: &Client,
    sender: AccountAddress,
    pool_address: AccountAddress,
) -> CliTypedResult<()> {
    let (voter, voting_power, required_stake, voting_duration, lockup_end, now) = try_join!(
        view_one::<Address>(
            client,
            ident_str!("stake"),
            ident_str!("get_delegated_voter"),
            vec![],
            vec![bcs_arg(&pool_address)],
        ),
        view_one::<U64>(
            client,
            ident_str!("aptos_governance"),
            ident_str!("get_voting_power"),
            vec![],
            vec![bcs_arg(&pool_address)],
        ),
        view_one::<U64>(
            client,
            ident_str!("aptos_governance"),
            ident_str!("get_required_proposer_stake"),
            vec![],
            vec![],
        ),
        view_one::<U64>(
            client,
            ident_str!("aptos_governance"),
            ident_str!("get_voting_duration_secs"),
            vec![],
            vec![],
        ),
        view_one::<U64>(
            client,
            ident_str!("stake"),
            ident_str!("get_lockup_secs"),
            vec![],
            vec![bcs_arg(&pool_address)],
        ),
        ledger_timestamp_secs(client),
    )?;

    let voter = AccountAddress::from(voter);
    if voter != sender {
        return Err(CliError::CommandArgumentError(format!(
            "the sender {} is not the delegated voter of stake pool {} (the voter is {})",
            sender, pool_address, voter
        )));
    }
    if voting_power.0 < required_stake.0 {
        return Err(CliError::CommandArgumentError(format!(
            "stake pool {} has voting power {} but proposing requires at least {}",
            pool_address, voting_power.0, required_stake.0
        )));
    }
    let voting_end = now + voting_duration.0;
    if voting_end >= lockup_end.0 {
        return Err(CliError::CommandArgumentError(format!(
            "the lockup of stake pool {} ends at unix second {}, before the voting period would \
             end at {}; the pool owner must increase the lockup before proposing",
            pool_address, lockup_end.0, voting_end
        )));
    }
    Ok(())
}

/// Execute an approved governance bundle's proposal
///
/// Submits the bundle's compiled scripts that have not been executed yet, in
/// order, starting from the step the chain expects next. Re-running after a
/// failure resumes from the failed step. Nothing is compiled.
#[derive(Parser)]
pub struct ExecuteBundle {
    #[clap(flatten)]
    pub(crate) bundle_args: BundleArgs,

    /// Id of the approved proposal to execute
    #[clap(long)]
    pub(crate) proposal_id: u64,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

/// One executed step of a bundle's proposal.
#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutedStep {
    pub script: String,
    #[serde(flatten)]
    pub transaction: TransactionSummary,
}

#[async_trait]
impl CliCommand<Vec<ExecutedStep>> for ExecuteBundle {
    fn command_name(&self) -> &'static str {
        "ExecuteBundle"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<ExecutedStep>> {
        let scripts = self.bundle_args.load()?;
        let proposal_id = self.proposal_id;
        let client = self.txn_options.rest_client()?;

        let (next_hash, approved) = try_join!(
            expected_next_hash(&client, proposal_id),
            approved_hashes(&client),
        )?;
        let Some(start) = scripts.iter().position(|s| s.hash == next_hash) else {
            return Err(CliError::CommandArgumentError(format!(
                "proposal {} expects a script with execution hash {} next, but no script in the \
                 bundle has that hash; check the proposal id and that this is the bundle that \
                 was proposed",
                proposal_id, next_hash
            )));
        };
        if start > 0 {
            println!(
                "{} of {} steps already executed; resuming at {}",
                start,
                scripts.len(),
                scripts[start].name
            );
        }

        prompt_yes_with_override(
            &format!(
                "Execute {} step(s) of proposal {}?",
                scripts.len() - start,
                proposal_id
            ),
            self.txn_options.prompt_options,
        )?;

        // The approved-hash entry lets an oversized script clear the mempool
        // size limit. Each executed step rolls it forward on-chain, so only the
        // first needs seeding.
        if !approved.entries.iter().any(|(id, _)| *id == proposal_id) {
            println!(
                "Approving the execution hash of proposal {}...",
                proposal_id
            );
            submit_step(
                &self.txn_options,
                aptos_stdlib::aptos_governance_add_approved_script_hash_script(proposal_id),
                "approving the execution hash",
            )
            .await?;
        }

        let mut executed = Vec::with_capacity(scripts.len() - start);
        for BundleScript { name, blob, .. } in scripts.into_iter().skip(start) {
            println!("Executing {}...", name);
            let txn = submit_step(
                &self.txn_options,
                execution_payload(blob, proposal_id),
                &name,
            )
            .await?;
            executed.push(ExecutedStep {
                script: name,
                transaction: TransactionSummary::from(&txn),
            });
        }
        Ok(executed)
    }
}

/// The execution hash of the step the chain expects next, for a proposal that
/// has passed and is not fully executed. The chain rolls a proposal's execution
/// hash forward after each executed step.
async fn expected_next_hash(client: &Client, proposal_id: u64) -> CliTypedResult<HashValue> {
    let (state, resolved, next_hash) = try_join!(
        voting_view::<U64>(client, ident_str!("get_proposal_state"), proposal_id),
        voting_view::<bool>(client, ident_str!("is_resolved"), proposal_id),
        voting_view::<ApiHashValue>(client, ident_str!("get_execution_hash"), proposal_id),
    )?;

    match state.0 {
        PROPOSAL_STATE_SUCCEEDED => {},
        PROPOSAL_STATE_PENDING => {
            let expiration: U64 = voting_view(
                client,
                ident_str!("get_proposal_expiration_secs"),
                proposal_id,
            )
            .await?;
            return Err(CliError::CommandArgumentError(format!(
                "proposal {} is still open for voting until unix second {}",
                proposal_id, expiration.0
            )));
        },
        PROPOSAL_STATE_FAILED => {
            return Err(CliError::CommandArgumentError(format!(
                "proposal {} did not pass and cannot be executed",
                proposal_id
            )));
        },
        other => {
            return Err(CliError::UnexpectedError(format!(
                "proposal {} is in unknown state {}",
                proposal_id, other
            )));
        },
    }
    if resolved {
        return Err(CliError::CommandArgumentError(format!(
            "proposal {} has already been fully executed",
            proposal_id
        )));
    }
    Ok(next_hash.into())
}

async fn approved_hashes(client: &Client) -> CliTypedResult<ApprovedExecutionHashes> {
    Ok(client
        .get_account_resource_bcs::<ApprovedExecutionHashes>(
            AccountAddress::ONE,
            "0x1::aptos_governance::ApprovedExecutionHashes",
        )
        .await?
        .into_inner())
}

/// Submit one transaction of the execution; on failure, say how to resume.
async fn submit_step(
    txn_options: &TransactionOptions,
    payload: TransactionPayload,
    step: &str,
) -> CliTypedResult<Transaction> {
    txn_options
        .submit_transaction(payload)
        .await
        .inspect_err(|_| {
            eprintln!(
                "Step {} failed. Once the cause is fixed, re-run this command to resume from it.",
                step
            )
        })
}

async fn ledger_timestamp_secs(client: &Client) -> CliTypedResult<u64> {
    Ok(client
        .get_ledger_information()
        .await?
        .into_inner()
        .timestamp_usecs
        / 1_000_000)
}

/// Call a view function of a `0x1` module and deserialize its single return value.
async fn view_one<T: DeserializeOwned>(
    client: &Client,
    module: &IdentStr,
    function: &IdentStr,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) -> CliTypedResult<T> {
    let request = ViewFunction {
        module: ModuleId::new(AccountAddress::ONE, module.to_owned()),
        function: function.to_owned(),
        ty_args,
        args,
    };
    let mut values = client
        .view_bcs_with_json_response(&request, None)
        .await?
        .into_inner();
    if values.len() != 1 {
        return Err(CliError::UnexpectedError(format!(
            "view function {}::{} returned {} values, expected 1",
            module,
            function,
            values.len()
        )));
    }
    serde_json::from_value(values.remove(0)).map_err(|err| {
        CliError::UnexpectedError(format!(
            "failed to parse the result of view function {}::{}: {}",
            module, function, err
        ))
    })
}

/// Call a `0x1::voting` view function on a governance proposal.
async fn voting_view<T: DeserializeOwned>(
    client: &Client,
    function: &IdentStr,
    proposal_id: u64,
) -> CliTypedResult<T> {
    view_one(
        client,
        ident_str!("voting"),
        function,
        vec![
            parse_type_tag("0x1::governance_proposal::GovernanceProposal")
                .expect("the governance proposal type tag is well-formed"),
        ],
        vec![bcs_arg(&AccountAddress::ONE), bcs_arg(&proposal_id)],
    )
    .await
}

fn bcs_arg<T: Serialize>(value: &T) -> Vec<u8> {
    bcs::to_bytes(value).expect("BCS-encoding a view function argument cannot fail")
}
