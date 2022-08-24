// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliError, CliTypedResult, MovePackageDir, PoolAddressArgs, PromptOptions, TransactionOptions,
    TransactionSummary,
};
use crate::common::utils::prompt_yes_with_override;
use crate::move_tool::{init_move_dir, IncludedArtifacts};
use crate::{CliCommand, CliResult};
use aptos_crypto::HashValue;
use aptos_logger::warn;
use aptos_rest_client::aptos_api_types::{MoveStructTag, WriteResource, WriteSetChange, U64};
use aptos_rest_client::Transaction;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{Script, TransactionPayload},
};
use async_trait::async_trait;
use cached_packages::aptos_stdlib;
use clap::Parser;
use framework::{BuildOptions, BuiltPackage, ReleasePackage};
use move_deps::move_core_types::transaction_argument::TransactionArgument;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::{collections::BTreeMap, fmt::Formatter, fs, path::PathBuf};
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
            GenerateUpgradeProposal(tool) => tool.execute_serialized().await,
        }
    }
}

/// Submit proposal to other validators to be proposed on
#[derive(Parser)]
pub struct SubmitProposal {
    /// Code location of the script to be voted on
    #[clap(long)]
    pub(crate) metadata_url: Url,
    #[clap(long)]
    pub(crate) metadata_path: Option<PathBuf>,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) pool_address_args: PoolAddressArgs,
    #[clap(flatten)]
    pub(crate) compile_proposal_args: CompileProposalArgs,
}

#[async_trait]
impl CliCommand<ProposalSubmissionSummary> for SubmitProposal {
    fn command_name(&self) -> &'static str {
        "SubmitProposal"
    }

    async fn execute(mut self) -> CliTypedResult<ProposalSubmissionSummary> {
        let (_bytecode, script_hash) = self.compile_proposal_args.compile()?;

        // Validate the proposal metadata
        let (metadata, metadata_hash) = self.get_metadata().await?;

        println!(
            "{}\n\tMetadata Hash: {}\n\tScript Hash: {}",
            metadata, metadata_hash, script_hash
        );
        prompt_yes_with_override(
            "Do you want to submit this proposal?",
            self.compile_proposal_args.prompt_options,
        )?;

        let txn = self
            .txn_options
            .submit_transaction(aptos_stdlib::aptos_governance_create_proposal(
                self.pool_address_args.pool_address,
                script_hash.to_vec(),
                self.metadata_url.to_string().as_bytes().to_vec(),
                metadata_hash.to_hex().as_bytes().to_vec(),
            ))
            .await?;

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
            let request = inner.request;
            let info = inner.info;

            return Ok(ProposalSubmissionSummary {
                proposal_id,
                transaction_hash: info.hash.into(),
                transaction_version: info.version.into(),
                gas_used: info.gas_used.0,
                gas_price_per_unit: request.gas_unit_price.0,
                sequence_number: request.sequence_number.0,
                vm_status: info.vm_status,
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
        let bytes = if let Some(path) = &self.metadata_path {
            Self::get_metadata_from_file(path)?
        } else {
            Self::get_metadata_from_url(&self.metadata_url).await?
        };

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

    async fn get_metadata_from_url(metadata_url: &Url) -> CliTypedResult<Vec<u8>> {
        let client = reqwest::ClientBuilder::default()
            .tls_built_in_root_certs(true)
            .build()
            .map_err(|err| {
                CliError::UnexpectedError(format!("Failed to build HTTP client {}", err))
            })?;
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

    fn get_metadata_from_file(metadata_path: &PathBuf) -> CliTypedResult<Vec<u8>> {
        fs::read(metadata_path).map_err(|err| {
            CliError::CommandArgumentError(format!(
                "Failed to read metadata path {:?}: {}",
                metadata_path, err
            ))
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct CreateProposalEvent {
    proposal_id: U64,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProposalSubmissionSummary {
    proposal_id: Option<u64>,
    transaction_hash: HashValue,
    transaction_version: u64,
    gas_used: u64,
    gas_price_per_unit: u64,
    sequence_number: u64,
    vm_status: String,
}

/// Submit a vote on a current proposal
#[derive(Parser)]
pub struct SubmitVote {
    /// Id of proposal to vote on
    #[clap(long)]
    pub(crate) proposal_id: u64,

    /// Vote yes on the proposal
    #[clap(long, group = "vote")]
    pub(crate) yes: bool,

    /// Vote no on the proposal
    #[clap(long, group = "vote")]
    pub(crate) no: bool,

    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) pool_address_args: PoolAddressArgs,
}

#[async_trait]
impl CliCommand<TransactionSummary> for SubmitVote {
    fn command_name(&self) -> &'static str {
        "SubmitVote"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let (vote_str, vote) = match (self.yes, self.no) {
            (true, false) => ("Yes", true),
            (false, true) => ("No", false),
            (_, _) => {
                return Err(CliError::CommandArgumentError(
                    "Must choose either --yes or --no".to_string(),
                ))
            }
        };

        // TODO: Display details of proposal

        prompt_yes_with_override(
            &format!("Are you sure you want to vote {}", vote_str),
            self.prompt_options,
        )?;

        self.txn_options
            .submit_transaction(aptos_stdlib::aptos_governance_vote(
                self.pool_address_args.pool_address,
                self.proposal_id,
                vote,
            ))
            .await
            .map(TransactionSummary::from)
    }
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
    script_path: &Path,
    prompt_options: PromptOptions,
) -> CliTypedResult<(Vec<u8>, HashValue)> {
    // Make a temporary directory for compilation
    let temp_dir = TempDir::new().map_err(|err| {
        CliError::UnexpectedError(format!("Failed to create temporary directory {}", err))
    })?;

    // Initialize a move directory
    let package_dir = temp_dir.path();
    init_move_dir(package_dir, "Proposal", BTreeMap::new(), prompt_options)?;

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
    compile_script(package_dir)
}

fn compile_script(package_dir: &Path) -> CliTypedResult<(Vec<u8>, HashValue)> {
    let build_options = BuildOptions {
        with_srcs: false,
        with_abis: false,
        with_source_maps: false,
        with_error_map: false,
        install_dir: None,
        named_addresses: Default::default(),
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
    pub(crate) compile_proposal_args: CompileProposalArgs,
}

#[async_trait]
impl CliCommand<TransactionSummary> for ExecuteProposal {
    fn command_name(&self) -> &'static str {
        "ExecuteProposal"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let (bytecode, _script_hash) = self.compile_proposal_args.compile()?;
        // TODO: Check hash so we don't do a failed roundtrip?

        let args = vec![TransactionArgument::U64(self.proposal_id)];
        let txn = TransactionPayload::Script(Script::new(bytecode, vec![], args));

        self.txn_options
            .submit_transaction(txn)
            .await
            .map(TransactionSummary::from)
    }
}

/// Execute a proposal that has passed voting requirements
#[derive(Parser)]
pub struct CompileProposalArgs {
    /// Path to the Move script for the proposal
    #[clap(long, parse(from_os_str))]
    pub script_path: PathBuf,

    #[clap(flatten)]
    pub prompt_options: PromptOptions,
}

impl CompileProposalArgs {
    fn compile(&self) -> CliTypedResult<(Vec<u8>, HashValue)> {
        // Check script file
        let script_path = self.script_path.as_path();
        if !self.script_path.exists() {
            return Err(CliError::CommandArgumentError(format!(
                "{} does not exist",
                script_path.display()
            )));
        } else if self.script_path.is_dir() {
            return Err(CliError::CommandArgumentError(format!(
                "{} is a directory",
                script_path.display()
            )));
        }

        // Compile script
        compile_in_temp_dir(script_path, self.prompt_options)
    }
}

/// Generates a package upgrade proposal script.
#[derive(Parser)]
pub struct GenerateUpgradeProposal {
    /// Address of the account which the proposal addresses.
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
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
        } = self;
        let package_path = move_options.get_package_path()?;
        let options = included_artifacts.build_options(move_options.named_addresses());
        let package = BuiltPackage::build(package_path, options)?;
        let release = ReleasePackage::new(package)?;
        release.generate_script_proposal(account, output)?;
        Ok(())
    }
}
