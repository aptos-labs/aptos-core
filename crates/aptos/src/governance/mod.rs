// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "no-upload-proposal")]
use crate::common::utils::read_from_file;
use crate::{
    common::{
        types::{
            CliError, CliTypedResult, MovePackageDir, PoolAddressArgs, ProfileOptions,
            PromptOptions, RestOptions, TransactionOptions, TransactionSummary,
        },
        utils::prompt_yes_with_override,
    },
    move_tool::{set_bytecode_version, FrameworkPackageArgs, IncludedArtifacts},
    CliCommand, CliResult,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::HashValue;
use aptos_framework::{BuildOptions, BuiltPackage, ReleasePackage};
use aptos_logger::warn;
use aptos_rest_client::{
    aptos_api_types::{Address, HexEncodedBytes, U128, U64},
    Client, Transaction,
};
use aptos_sdk::move_types::language_storage::CORE_CODE_ADDRESS;
use aptos_types::{
    account_address::AccountAddress,
    event::EventHandle,
    governance::VotingRecords,
    stake_pool::StakePool,
    state_store::table::TableHandle,
    transaction::{Script, TransactionPayload},
};
use async_trait::async_trait;
use clap::Parser;
use move_core_types::transaction_argument::TransactionArgument;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::Formatter,
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

/// Tool for on-chain governance
///
/// This tool allows voters that have stake to vote the ability to
/// propose changes to the chain, as well as vote and execute these
/// proposals.
#[derive(Parser)]
pub enum GovernanceTool {
    Propose(SubmitProposal),
    Vote(SubmitVote),
    ShowProposal(ViewProposal),
    ListProposals(ListProposals),
    VerifyProposal(VerifyProposal),
    ExecuteProposal(ExecuteProposal),
    GenerateUpgradeProposal(GenerateUpgradeProposal),
}

impl GovernanceTool {
    pub async fn execute(self) -> CliResult {
        use GovernanceTool::*;
        match self {
            Propose(tool) => tool.execute_serialized().await,
            Vote(tool) => tool.execute_serialized().await,
            ExecuteProposal(tool) => tool.execute_serialized().await,
            GenerateUpgradeProposal(tool) => tool.execute_serialized_success().await,
            ShowProposal(tool) => tool.execute_serialized().await,
            ListProposals(tool) => tool.execute_serialized().await,
            VerifyProposal(tool) => tool.execute_serialized().await,
        }
    }
}

/// View a known on-chain governance proposal
///
/// This command will return the proposal requested as well as compute
/// the hash of the metadata to determine whether it was verified or not.
#[derive(Parser)]
pub struct ViewProposal {
    /// The identifier of the onchain governance proposal
    #[clap(long)]
    proposal_id: u64,

    #[clap(flatten)]
    rest_options: RestOptions,
    #[clap(flatten)]
    profile: ProfileOptions,
}

#[async_trait]
impl CliCommand<VerifiedProposal> for ViewProposal {
    fn command_name(&self) -> &'static str {
        "ViewProposal"
    }

    async fn execute(mut self) -> CliTypedResult<VerifiedProposal> {
        // Get proposal
        let client = self.rest_options.client(&self.profile)?;
        let forum = client
            .get_account_resource_bcs::<VotingForum>(
                AccountAddress::ONE,
                "0x1::voting::VotingForum<0x1::governance_proposal::GovernanceProposal>",
            )
            .await?
            .into_inner();
        let voting_table = forum.table_handle.0;

        let proposal: Proposal = get_proposal(&client, voting_table, self.proposal_id)
            .await?
            .into();

        let metadata_hash = proposal.metadata.get("metadata_hash").unwrap();
        let metadata_url = proposal.metadata.get("metadata_location").unwrap();

        // Compute the hash and verify accordingly
        let mut metadata_verified = false;
        let mut actual_metadata_hash = "Unable to fetch metadata url".to_string();
        let mut actual_metadata = None;
        if let Ok(url) = Url::parse(metadata_url) {
            if let Ok(bytes) = get_metadata_from_url(&url).await {
                let hash = HashValue::sha3_256_of(&bytes);
                metadata_verified = metadata_hash == &hash.to_hex();
                actual_metadata_hash = hash.to_hex();
                if let Ok(metadata) = String::from_utf8(bytes) {
                    actual_metadata = Some(metadata);
                }
            }
        }

        Ok(VerifiedProposal {
            metadata_verified,
            actual_metadata_hash,
            actual_metadata,
            proposal,
        })
    }
}

/// List the last 100 visible onchain proposals
///
/// Note, if the full node you are talking to is pruning data, it may not have some of the
/// proposals show here
#[derive(Parser)]
pub struct ListProposals {
    #[clap(flatten)]
    rest_options: RestOptions,
    #[clap(flatten)]
    profile: ProfileOptions,
}

#[async_trait]
impl CliCommand<Vec<ProposalSummary>> for ListProposals {
    fn command_name(&self) -> &'static str {
        "ListProposals"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<ProposalSummary>> {
        // List out known proposals based on events
        let client = self.rest_options.client(&self.profile)?;

        let events = client
            .get_account_events_bcs(
                AccountAddress::ONE,
                "0x1::aptos_governance::GovernanceEvents",
                "create_proposal_events",
                None,
                Some(100),
            )
            .await?
            .into_inner();
        let mut proposals = vec![];

        for event in events {
            let event = bcs::from_bytes::<CreateProposalFullEvent>(event.event.event_data())?;
            proposals.push(event.into());
        }

        // TODO: Show more information about proposal?
        Ok(proposals)
    }
}

/// Verify a proposal given the source code of the script
///
/// The script's bytecode or source can be provided and it will
/// verify whether the hash matches the onchain hash
#[derive(Parser)]
pub struct VerifyProposal {
    /// The id of the onchain proposal
    #[clap(long)]
    pub(crate) proposal_id: u64,

    #[clap(flatten)]
    pub(crate) compile_proposal_args: CompileScriptFunction,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile: ProfileOptions,
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<VerifyProposalResponse> for VerifyProposal {
    fn command_name(&self) -> &'static str {
        "VerifyProposal"
    }

    async fn execute(mut self) -> CliTypedResult<VerifyProposalResponse> {
        // Compile local first to get the hash
        let (_, hash) = self
            .compile_proposal_args
            .compile("SubmitProposal", self.prompt_options)?;

        // Retrieve the onchain proposal
        let client = self.rest_options.client(&self.profile)?;
        let forum = client
            .get_account_resource_bcs::<VotingForum>(
                AccountAddress::ONE,
                "0x1::voting::VotingForum<0x1::governance_proposal::GovernanceProposal>",
            )
            .await?
            .into_inner();
        let voting_table = forum.table_handle.0;

        let proposal: Proposal = get_proposal(&client, voting_table, self.proposal_id)
            .await?
            .into();

        // Compare the hashes
        let computed_hash = hash.to_hex();
        let onchain_hash = proposal.execution_hash;

        Ok(VerifyProposalResponse {
            verified: computed_hash == onchain_hash,
            computed_hash,
            onchain_hash,
        })
    }
}

async fn get_proposal(
    client: &aptos_rest_client::Client,
    voting_table: AccountAddress,
    proposal_id: u64,
) -> CliTypedResult<JsonProposal> {
    let json = client
        .get_table_item(
            voting_table,
            "u64",
            "0x1::voting::Proposal<0x1::governance_proposal::GovernanceProposal>",
            format!("{}", proposal_id),
        )
        .await?
        .into_inner();
    serde_json::from_value(json)
        .map_err(|err| CliError::CommandArgumentError(format!("Failed to parse proposal {}", err)))
}

/// Submit a governance proposal
#[derive(Parser)]
pub struct SubmitProposal {
    /// Location of the JSON metadata of the proposal
    ///
    /// If this location does not keep the metadata in the exact format, it will be less likely
    /// that voters will approve this proposal, as they won't be able to verify it.
    #[clap(long)]
    pub(crate) metadata_url: Url,

    #[cfg(feature = "no-upload-proposal")]
    /// A JSON file to be uploaded later at the metadata URL
    ///
    /// If this does not match properly, voters may choose to vote no.  For real proposals,
    /// it is better to already have it uploaded at the URL.
    #[clap(long)]
    pub(crate) metadata_path: Option<PathBuf>,

    #[clap(long, default_value = "false")]
    pub(crate) is_multi_step: bool,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) pool_address_args: PoolAddressArgs,
    #[clap(flatten)]
    pub(crate) compile_proposal_args: CompileScriptFunction,
}

#[async_trait]
impl CliCommand<ProposalSubmissionSummary> for SubmitProposal {
    fn command_name(&self) -> &'static str {
        "SubmitProposal"
    }

    async fn execute(mut self) -> CliTypedResult<ProposalSubmissionSummary> {
        let (_bytecode, script_hash) = self
            .compile_proposal_args
            .compile("SubmitProposal", self.txn_options.prompt_options)?;

        // Validate the proposal metadata
        let (metadata, metadata_hash) = self.get_metadata().await?;

        println!(
            "{}\n\tMetadata Hash: {}\n\tScript Hash: {}",
            metadata, metadata_hash, script_hash
        );
        prompt_yes_with_override(
            "Do you want to submit this proposal?",
            self.txn_options.prompt_options,
        )?;

        let txn: Transaction = if self.is_multi_step {
            self.txn_options
                .submit_transaction(aptos_stdlib::aptos_governance_create_proposal_v2(
                    self.pool_address_args.pool_address,
                    script_hash.to_vec(),
                    self.metadata_url.to_string().as_bytes().to_vec(),
                    metadata_hash.to_hex().as_bytes().to_vec(),
                    true,
                ))
                .await?
        } else {
            self.txn_options
                .submit_transaction(aptos_stdlib::aptos_governance_create_proposal(
                    self.pool_address_args.pool_address,
                    script_hash.to_vec(),
                    self.metadata_url.to_string().as_bytes().to_vec(),
                    metadata_hash.to_hex().as_bytes().to_vec(),
                ))
                .await?
        };
        let txn_summary = TransactionSummary::from(&txn);
        if let Transaction::UserTransaction(inner) = txn {
            // Find event with proposal id
            let proposal_id = if let Some(event) = inner.events.into_iter().find(|event| {
                event.typ.to_string().as_str() == "0x1::aptos_governance::CreateProposalEvent"
            }) {
                let data: CreateProposalEvent =
                    serde_json::from_value(event.data).map_err(|_| {
                        CliError::UnexpectedError(
                            "Failed to parse Proposal event to get ProposalId".to_string(),
                        )
                    })?;
                Some(data.proposal_id.0)
            } else {
                warn!("No proposal event found to find proposal id");
                None
            };

            return Ok(ProposalSubmissionSummary {
                proposal_id,
                transaction: txn_summary,
            });
        }
        Err(CliError::UnexpectedError(
            "Unable to find parse proposal transaction output".to_string(),
        ))
    }
}

impl SubmitProposal {
    /// Retrieve metadata and validate it
    async fn get_metadata(&self) -> CliTypedResult<(ProposalMetadata, HashValue)> {
        #[cfg(feature = "no-upload-proposal")]
        let bytes = if let Some(ref path) = self.metadata_path {
            read_from_file(path)?
        } else {
            get_metadata_from_url(&self.metadata_url).await?
        };
        #[cfg(not(feature = "no-upload-proposal"))]
        let bytes = get_metadata_from_url(&self.metadata_url).await?;

        let metadata: ProposalMetadata = serde_json::from_slice(&bytes).map_err(|err| {
            CliError::CommandArgumentError(format!(
                "Metadata is not in a proper JSON format: {}",
                err
            ))
        })?;
        Url::parse(&metadata.source_code_url).map_err(|err| {
            CliError::CommandArgumentError(format!(
                "Source code URL {} is invalid {}",
                metadata.source_code_url, err
            ))
        })?;
        Url::parse(&metadata.discussion_url).map_err(|err| {
            CliError::CommandArgumentError(format!(
                "Discussion URL {} is invalid {}",
                metadata.discussion_url, err
            ))
        })?;
        let metadata_hash = HashValue::sha3_256_of(&bytes);
        Ok((metadata, metadata_hash))
    }
}

/// Retrieve the Metadata from the given URL
async fn get_metadata_from_url(metadata_url: &Url) -> CliTypedResult<Vec<u8>> {
    let client = reqwest::ClientBuilder::default()
        .tls_built_in_root_certs(true)
        .build()
        .map_err(|err| CliError::UnexpectedError(format!("Failed to build HTTP client {}", err)))?;
    client
        .get(metadata_url.clone())
        .send()
        .await
        .map_err(|err| {
            CliError::CommandArgumentError(format!(
                "Failed to fetch metadata url {}: {}",
                metadata_url, err
            ))
        })?
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|err| {
            CliError::CommandArgumentError(format!(
                "Failed to fetch metadata url {}: {}",
                metadata_url, err
            ))
        })
}

#[derive(Debug, Deserialize, Serialize)]
struct CreateProposalEvent {
    proposal_id: U64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProposalSubmissionSummary {
    proposal_id: Option<u64>,
    #[serde(flatten)]
    transaction: TransactionSummary,
}

/// Submit a vote on a proposal
///
/// Votes can only be given on proposals that are currently open for voting.  You can vote
/// with `--yes` for a yes vote, and `--no` for a no vote.
#[derive(Parser)]
pub struct SubmitVote {
    /// Id of the proposal to vote on
    #[clap(long)]
    pub(crate) proposal_id: u64,

    /// Vote to accept the proposal
    #[clap(long, group = "vote")]
    pub(crate) yes: bool,

    /// Vote to reject the proposal
    #[clap(long, group = "vote")]
    pub(crate) no: bool,

    /// Space separated list of pool addresses.
    #[clap(long, multiple_values = true, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) pool_addresses: Vec<AccountAddress>,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Vec<TransactionSummary>> for SubmitVote {
    fn command_name(&self) -> &'static str {
        "SubmitVote"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<TransactionSummary>> {
        let (vote_str, vote) = match (self.yes, self.no) {
            (true, false) => ("Yes", true),
            (false, true) => ("No", false),
            (_, _) => {
                return Err(CliError::CommandArgumentError(
                    "Must choose only either --yes or --no".to_string(),
                ));
            },
        };

        let client: &Client = &self
            .txn_options
            .rest_options
            .client(&self.txn_options.profile_options)?;
        let proposal_id = self.proposal_id;
        let voting_records = client
            .get_account_resource_bcs::<VotingRecords>(
                CORE_CODE_ADDRESS,
                "0x1::aptos_governance::VotingRecords",
            )
            .await
            .unwrap()
            .into_inner()
            .votes;

        let mut summaries: Vec<TransactionSummary> = vec![];
        for pool_address in self.pool_addresses {
            let voting_record = client
                .get_table_item(
                    voting_records,
                    "0x1::aptos_governance::RecordKey",
                    "bool",
                    VotingRecord {
                        proposal_id: proposal_id.to_string(),
                        stake_pool: pool_address,
                    },
                )
                .await;
            let voted = if let Ok(voting_record) = voting_record {
                voting_record.into_inner().as_bool().unwrap()
            } else {
                false
            };
            if voted {
                println!("Stake pool {} already voted", pool_address);
                continue;
            }

            let stake_pool = client
                .get_account_resource_bcs::<StakePool>(pool_address, "0x1::stake::StakePool")
                .await?
                .into_inner();
            let voting_power = stake_pool.get_governance_voting_power();

            prompt_yes_with_override(
                &format!(
                    "Vote {} with voting power = {} from stake pool {}?",
                    vote_str, voting_power, pool_address
                ),
                self.txn_options.prompt_options,
            )?;

            summaries.push(
                self.txn_options
                    .submit_transaction(aptos_stdlib::aptos_governance_vote(
                        pool_address,
                        proposal_id,
                        vote,
                    ))
                    .await
                    .map(TransactionSummary::from)?,
            );
        }
        Ok(summaries)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VotingRecord {
    proposal_id: String,
    stake_pool: AccountAddress,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProposalMetadata {
    title: String,
    description: String,
    source_code_url: String,
    discussion_url: String,
}

impl std::fmt::Display for ProposalMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Proposal:\n\tTitle:{}\n\tDescription:{}\n\tSource code URL:{}\n\tDiscussion URL:{}",
            self.title, self.description, self.source_code_url, self.discussion_url
        )
    }
}

fn compile_in_temp_dir(
    script_name: &str,
    script_path: &Path,
    framework_package_args: &FrameworkPackageArgs,
    prompt_options: PromptOptions,
    bytecode_version: u32,
) -> CliTypedResult<(Vec<u8>, HashValue)> {
    // Make a temporary directory for compilation
    let temp_dir = TempDir::new().map_err(|err| {
        CliError::UnexpectedError(format!("Failed to create temporary directory {}", err))
    })?;

    // Initialize a move directory
    let package_dir = temp_dir.path();
    framework_package_args.init_move_dir(
        package_dir,
        script_name,
        BTreeMap::new(),
        prompt_options,
    )?;

    // Insert the new script
    let sources_dir = package_dir.join("sources");
    let new_script_path = if let Some(file_name) = script_path.file_name() {
        sources_dir.join(file_name)
    } else {
        // If for some reason we can't get the move file
        sources_dir.join("script.move")
    };
    fs::copy(script_path, new_script_path.as_path()).map_err(|err| {
        CliError::IO(
            format!(
                "Failed to copy {} to {}",
                script_path.display(),
                new_script_path.display()
            ),
            err,
        )
    })?;

    // Compile the script
    compile_script(
        framework_package_args.skip_fetch_latest_git_deps,
        package_dir,
        bytecode_version,
    )
}

fn compile_script(
    skip_fetch_latest_git_deps: bool,
    package_dir: &Path,
    bytecode_version: u32,
) -> CliTypedResult<(Vec<u8>, HashValue)> {
    let build_options = BuildOptions {
        with_srcs: false,
        with_abis: false,
        with_source_maps: false,
        with_error_map: false,
        skip_fetch_latest_git_deps,
        bytecode_version: Some(bytecode_version),
        ..BuildOptions::default()
    };

    let pack = BuiltPackage::build(package_dir.to_path_buf(), build_options)?;

    let scripts_count = pack.script_count();

    if scripts_count != 1 {
        return Err(CliError::UnexpectedError(format!(
            "Only one script can be prepared a time. Make sure one and only one script file \
                is included in the Move package. Found {} scripts.",
            scripts_count
        )));
    }

    let bytes = pack.extract_script_code().pop().unwrap();
    let hash = HashValue::sha3_256_of(bytes.as_slice());
    Ok((bytes, hash))
}

/// Execute a proposal that has passed voting requirements
#[derive(Parser)]
pub struct ExecuteProposal {
    /// Proposal Id being executed
    #[clap(long)]
    pub(crate) proposal_id: u64,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) compile_proposal_args: CompileScriptFunction,
}

#[async_trait]
impl CliCommand<TransactionSummary> for ExecuteProposal {
    fn command_name(&self) -> &'static str {
        "ExecuteProposal"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let (bytecode, _script_hash) = self
            .compile_proposal_args
            .compile("ExecuteProposal", self.txn_options.prompt_options)?;
        // TODO: Check hash so we don't do a failed roundtrip?

        let args = vec![TransactionArgument::U64(self.proposal_id)];
        let txn = TransactionPayload::Script(Script::new(bytecode, vec![], args));

        self.txn_options
            .submit_transaction(txn)
            .await
            .map(TransactionSummary::from)
    }
}

/// Compile a specified script.
#[derive(Parser)]
pub struct CompileScriptFunction {
    /// Path to the Move script for the proposal
    #[clap(long, group = "script", parse(from_os_str))]
    pub script_path: Option<PathBuf>,

    /// Path to the Move script for the proposal
    #[clap(long, group = "script", parse(from_os_str))]
    pub compiled_script_path: Option<PathBuf>,

    #[clap(flatten)]
    pub(crate) framework_package_args: FrameworkPackageArgs,

    #[clap(long)]
    pub(crate) bytecode_version: Option<u32>,
}

impl CompileScriptFunction {
    pub(crate) fn compile(
        &self,
        script_name: &str,
        prompt_options: PromptOptions,
    ) -> CliTypedResult<(Vec<u8>, HashValue)> {
        set_bytecode_version(self.bytecode_version);
        if let Some(compiled_script_path) = &self.compiled_script_path {
            let bytes = std::fs::read(compiled_script_path).map_err(|e| {
                CliError::IO(format!("Unable to read {:?}", self.compiled_script_path), e)
            })?;
            let hash = HashValue::sha3_256_of(bytes.as_slice());
            return Ok((bytes, hash));
        }

        // Check script file
        let script_path = self
            .script_path
            .as_ref()
            .ok_or_else(|| {
                CliError::CommandArgumentError(
                    "Must choose either --compiled-script-path or --script-path".to_string(),
                )
            })?
            .as_path();
        if !script_path.exists() {
            return Err(CliError::CommandArgumentError(format!(
                "{} does not exist",
                script_path.display()
            )));
        } else if script_path.is_dir() {
            return Err(CliError::CommandArgumentError(format!(
                "{} is a directory",
                script_path.display()
            )));
        }

        // Compile script
        compile_in_temp_dir(
            script_name,
            script_path,
            &self.framework_package_args,
            prompt_options,
            self.bytecode_version.unwrap_or(5),
        )
    }
}

/// Generates a package upgrade proposal script.
#[derive(Parser)]
pub struct GenerateUpgradeProposal {
    /// Address of the account which the proposal addresses.
    #[clap(long, parse(try_from_str = crate::common::types::load_account_arg))]
    pub(crate) account: AccountAddress,

    /// Where to store the generated proposal
    #[clap(long, parse(from_os_str), default_value = "proposal.move")]
    pub(crate) output: PathBuf,

    /// What artifacts to include in the package. This can be one of `none`, `sparse`, and
    /// `all`. `none` is the most compact form and does not allow to reconstruct a source
    /// package from chain; `sparse` is the minimal set of artifacts needed to reconstruct
    /// a source package; `all` includes all available artifacts. The choice of included
    /// artifacts heavily influences the size and therefore gas cost of publishing: `none`
    /// is the size of bytecode alone; `sparse` is roughly 2 times as much; and `all` 3-4
    /// as much.
    #[clap(long, default_value_t = IncludedArtifacts::Sparse)]
    pub(crate) included_artifacts: IncludedArtifacts,

    /// Generate the script for mainnet governance proposal by default or generate the upgrade script for testnet.
    #[clap(long)]
    pub(crate) testnet: bool,

    #[clap(long, default_value = "")]
    pub(crate) next_execution_hash: String,

    #[clap(flatten)]
    pub(crate) move_options: MovePackageDir,
}

#[async_trait]
impl CliCommand<()> for GenerateUpgradeProposal {
    fn command_name(&self) -> &'static str {
        "GenerateUpgradeProposal"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let GenerateUpgradeProposal {
            move_options,
            account,
            included_artifacts,
            output,
            testnet,
            next_execution_hash,
        } = self;
        let package_path = move_options.get_package_path()?;
        let options = included_artifacts.build_options(
            move_options.skip_fetch_latest_git_deps,
            move_options.named_addresses(),
            move_options.bytecode_version_or_detault(),
        );
        let package = BuiltPackage::build(package_path, options)?;
        let release = ReleasePackage::new(package)?;

        // If we're generating a single-step proposal on testnet
        if testnet && next_execution_hash.is_empty() {
            release.generate_script_proposal_testnet(account, output)?;
            // If we're generating a single-step proposal on mainnet
        } else if next_execution_hash.is_empty() {
            release.generate_script_proposal(account, output)?;
            // If we're generating a multi-step proposal
        } else {
            let next_execution_hash_bytes = hex::decode(next_execution_hash)?;
            release.generate_script_proposal_multi_step(
                account,
                output,
                next_execution_hash_bytes,
            )?;
        };
        Ok(())
    }
}

/// Generate execution hash for a specified script.
#[derive(Parser)]
pub struct GenerateExecutionHash {
    #[clap(long)]
    pub script_path: Option<PathBuf>,
}

impl GenerateExecutionHash {
    pub fn generate_hash(&self) -> CliTypedResult<(Vec<u8>, HashValue)> {
        CompileScriptFunction {
            script_path: self.script_path.clone(),
            compiled_script_path: None,
            framework_package_args: FrameworkPackageArgs {
                framework_git_rev: None,
                framework_local_dir: Option::from(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("..")
                        .join("..")
                        .join("aptos-move")
                        .join("framework")
                        .join("aptos-framework"),
                ),
                skip_fetch_latest_git_deps: false,
            },
            bytecode_version: None,
        }
        .compile("execution_hash", PromptOptions::yes())
    }
}

/// Response for `verify proposal`
#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyProposalResponse {
    pub verified: bool,
    pub computed_hash: String,
    pub onchain_hash: String,
}

/// Voting forum onchain type
///
/// TODO: Move to a shared location
#[derive(Serialize, Deserialize, Debug)]
pub struct VotingForum {
    table_handle: TableHandle,
    events: VotingEvents,
    next_proposal_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VotingEvents {
    create_proposal_events: EventHandle,
    register_forum_events: EventHandle,
    resolve_proposal_events: EventHandle,
    vote_events: EventHandle,
}

/// Summary of proposal from the listing events for `ListProposals`
#[derive(Serialize, Deserialize, Debug)]
struct ProposalSummary {
    proposer: AccountAddress,
    stake_pool: AccountAddress,
    proposal_id: u64,
    execution_hash: String,
    proposal_metadata: BTreeMap<String, String>,
}

impl From<CreateProposalFullEvent> for ProposalSummary {
    fn from(event: CreateProposalFullEvent) -> Self {
        let proposal_metadata = event
            .proposal_metadata
            .into_iter()
            .map(|(key, value)| (key, String::from_utf8(value).unwrap()))
            .collect();
        ProposalSummary {
            proposer: event.proposer,
            stake_pool: event.stake_pool,
            proposal_id: event.proposal_id,
            execution_hash: hex::encode(event.execution_hash),
            proposal_metadata,
        }
    }
}

#[derive(Deserialize)]
struct CreateProposalFullEvent {
    proposer: AccountAddress,
    stake_pool: AccountAddress,
    proposal_id: u64,
    execution_hash: Vec<u8>,
    proposal_metadata: BTreeMap<String, Vec<u8>>,
}

/// A proposal and the verified information about it
#[derive(Serialize, Deserialize, Debug)]
pub struct VerifiedProposal {
    metadata_verified: bool,
    actual_metadata_hash: String,
    actual_metadata: Option<String>,
    proposal: Proposal,
}

/// A reformatted type that has human readable version of the proposal onchain
#[derive(Serialize, Deserialize, Debug)]
pub struct Proposal {
    proposer: AccountAddress,
    metadata: BTreeMap<String, String>,
    creation_time_secs: u64,
    execution_hash: String,
    min_vote_threshold: u128,
    expiration_secs: u64,
    early_resolution_vote_threshold: Option<u128>,
    yes_votes: u128,
    no_votes: u128,
    is_resolved: bool,
    resolution_time_secs: u64,
}

impl From<JsonProposal> for Proposal {
    fn from(proposal: JsonProposal) -> Self {
        let metadata = proposal
            .metadata
            .data
            .into_iter()
            .map(|pair| {
                let value = match pair.key.as_str() {
                    "metadata_hash" => String::from_utf8(pair.value.0)
                        .unwrap_or_else(|_| "Failed to parse utf8".to_string()),
                    "metadata_location" => String::from_utf8(pair.value.0)
                        .unwrap_or_else(|_| "Failed to parse utf8".to_string()),
                    "RESOLVABLE_TIME_METADATA_KEY" => bcs::from_bytes::<u64>(pair.value.inner())
                        .map(|inner| inner.to_string())
                        .unwrap_or_else(|_| "Failed to parse u64".to_string()),
                    _ => pair.value.to_string(),
                };
                (pair.key, value)
            })
            .collect();

        Proposal {
            proposer: proposal.proposer.into(),
            metadata,
            creation_time_secs: proposal.creation_time_secs.into(),
            execution_hash: format!("{:x}", proposal.execution_hash),
            min_vote_threshold: proposal.min_vote_threshold.into(),
            expiration_secs: proposal.expiration_secs.into(),
            early_resolution_vote_threshold: proposal
                .early_resolution_vote_threshold
                .vec
                .first()
                .map(|inner| inner.0),
            yes_votes: proposal.yes_votes.into(),
            no_votes: proposal.no_votes.into(),
            is_resolved: proposal.is_resolved,
            resolution_time_secs: proposal.resolution_time_secs.into(),
        }
    }
}

/// An ugly JSON parsing version for from the JSON API
#[derive(Serialize, Deserialize, Debug)]
struct JsonProposal {
    creation_time_secs: U64,
    early_resolution_vote_threshold: JsonEarlyResolutionThreshold,
    execution_hash: aptos_rest_client::aptos_api_types::HashValue,
    expiration_secs: U64,
    is_resolved: bool,
    min_vote_threshold: U128,
    no_votes: U128,
    resolution_time_secs: U64,
    yes_votes: U128,
    proposer: Address,
    metadata: JsonMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonEarlyResolutionThreshold {
    vec: Vec<U128>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonMetadata {
    data: Vec<JsonMetadataPair>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonMetadataPair {
    key: String,
    value: HexEncodedBytes,
}
