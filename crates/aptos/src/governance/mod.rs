// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliError, CliTypedResult, MovePackageDir, PoolAddressArgs, PromptOptions, TransactionOptions,
    TransactionSummary,
};
use crate::common::utils::prompt_yes_with_override;
#[cfg(feature = "no-upload-proposal")]
use crate::common::utils::read_from_file;
use crate::move_tool::{FrameworkPackageArgs, IncludedArtifacts};
use crate::{CliCommand, CliResult};
use aptos_crypto::HashValue;
use aptos_logger::warn;
use aptos_rest_client::aptos_api_types::U64;
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
            GenerateUpgradeProposal(tool) => tool.execute_serialized_success().await,
        }
    }
}

/// Submit proposal to other validators to be proposed on
#[derive(Parser)]
pub struct SubmitProposal {
    /// Location of the JSON metadata of the proposal
    #[clap(long)]
    pub(crate) metadata_url: Url,

    #[cfg(feature = "no-upload-proposal")]
    /// A JSON file to be uploaded later at the metadata URL
    #[clap(long)]
    pub(crate) metadata_path: Option<PathBuf>,

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

        let txn = self
            .txn_options
            .submit_transaction(aptos_stdlib::aptos_governance_create_proposal(
                self.pool_address_args.pool_address,
                script_hash.to_vec(),
                self.metadata_url.to_string().as_bytes().to_vec(),
                metadata_hash.to_hex().as_bytes().to_vec(),
            ))
            .await?;
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
struct ProposalSubmissionSummary {
    proposal_id: Option<u64>,
    #[serde(flatten)]
    transaction: TransactionSummary,
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
            self.txn_options.prompt_options,
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
    script_name: &str,
    script_path: &Path,
    framework_package_args: &FrameworkPackageArgs,
    prompt_options: PromptOptions,
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

/// Execute a proposal that has passed voting requirements
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
}

impl CompileScriptFunction {
    pub(crate) fn compile(
        &self,
        script_name: &str,
        prompt_options: PromptOptions,
    ) -> CliTypedResult<(Vec<u8>, HashValue)> {
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
        )
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
