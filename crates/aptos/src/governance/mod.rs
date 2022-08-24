// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliError, CliTypedResult, PoolAddressArgs, PromptOptions, TransactionOptions,
    TransactionSummary,
};
use crate::common::utils::prompt_yes_with_override;
use crate::move_tool::{compile_move, init_move_dir, ArgWithType, FunctionArgType};
use crate::{CliCommand, CliResult};
use aptos_crypto::HashValue;
use aptos_logger::warn;
use aptos_rest_client::aptos_api_types::U64;
use aptos_rest_client::{aptos_api_types::MoveType, Transaction};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{Script, TransactionPayload},
};
use async_trait::async_trait;
use cached_packages::aptos_stdlib;
use clap::Parser;
use move_deps::{
    move_compiler::compiled_unit::CompiledUnitEnum,
    move_core_types::{language_storage::TypeTag, transaction_argument::TransactionArgument},
    move_package::BuildConfig,
};
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::{
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
    fmt::Formatter,
    fs,
    path::PathBuf,
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
    ExecuteProposal(ExecuteProposal),
}

impl GovernanceTool {
    pub async fn execute(self) -> CliResult {
        use GovernanceTool::*;
        match self {
            Propose(tool) => tool.execute_serialized().await,
            Vote(tool) => tool.execute_serialized().await,
            ExecuteProposal(tool) => tool.execute_serialized().await,
        }
    }
}

/// Submit proposal to other validators to be proposed on
#[derive(Parser)]
pub struct SubmitProposal {
    /// Code location of the script to be voted on
    #[clap(long)]
    pub(crate) metadata_url: Url,

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
        let (metadata, metadata_hash) = get_metadata(self.metadata_url.clone()).await?;

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
            let proposal_id = if let Some(event) = inner.events.iter().find(|event| {
                event.typ.to_string().as_str() == "0x1::aptos_governance::CreateProposalEvent"
            }) {
                let data: CreateProposalEvent = serde_json::from_value(event.data.clone())
                    .map_err(|_| {
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

/// Retrieve metadata and validate it
async fn get_metadata(metadata_url: Url) -> CliTypedResult<(ProposalMetadata, HashValue)> {
    let client = reqwest::ClientBuilder::default()
        .tls_built_in_root_certs(true)
        .build()
        .map_err(|err| CliError::UnexpectedError(format!("Failed to build HTTP client {}", err)))?;
    let bytes = client
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
        .map_err(|err| {
            CliError::CommandArgumentError(format!(
                "Failed to fetch metadata url {}: {}",
                metadata_url, err
            ))
        })?;
    let metadata: ProposalMetadata = serde_json::from_slice(&bytes).map_err(|err| {
        CliError::CommandArgumentError(format!("Metadata is not in a proper JSON format: {}", err))
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
impl CliCommand<Transaction> for SubmitVote {
    fn command_name(&self) -> &'static str {
        "SubmitVote"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
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
            .submit_entry_function(
                AccountAddress::ONE,
                "aptos_governance",
                "vote",
                vec![],
                vec![
                    bcs::to_bytes(&self.pool_address_args.pool_address)?,
                    bcs::to_bytes(&self.proposal_id)?,
                    bcs::to_bytes(&vote)?,
                ],
            )
            .await
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
    git_revision: &str,
    prompt_options: PromptOptions,
) -> CliTypedResult<(Vec<u8>, HashValue)> {
    // Make a temporary directory for compilation
    let temp_dir = TempDir::new().map_err(|err| {
        CliError::UnexpectedError(format!("Failed to create temporary directory {}", err))
    })?;

    // Initialize a move directory
    let package_dir = temp_dir.path();
    init_move_dir(
        package_dir,
        "Proposal",
        git_revision,
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
    let build_config = BuildConfig {
        additional_named_addresses: BTreeMap::new(),
        generate_abis: false,
        generate_docs: false,
        ..Default::default()
    };

    let compiled_package = compile_move(build_config, package_dir)?;
    let scripts_count = compiled_package.scripts().count();

    if scripts_count != 1 {
        return Err(CliError::UnexpectedError(format!(
            "Only one script can be prepared a time. Make sure one and only one script file \
                is included in the Move package. Found {} scripts.",
            scripts_count
        )));
    }

    let script = *compiled_package
        .scripts()
        .collect::<Vec<_>>()
        .get(0)
        .unwrap();

    match script.unit {
        CompiledUnitEnum::Script(ref s) => {
            let mut bytes = vec![];

            s.script
                .serialize(&mut bytes)
                .map_err(|err| CliError::UnexpectedError(format!("Unexpected error: {}", err)))?;
            let hash = HashValue::sha3_256_of(bytes.as_slice());
            Ok((bytes, hash))
        }
        CompiledUnitEnum::Module(_) => Err(CliError::UnexpectedError(
            "You can only execute a script, a module is not supported.".to_string(),
        )),
    }
}

/// Execute a proposal that has passed voting requirements
#[derive(Parser)]
pub struct ExecuteProposal {
    /// Arguments combined with their type separated by spaces.
    ///
    /// Supported types [u8, u64, u128, bool, hex, string, address]
    ///
    /// Example: `address:0x1 bool:true u8:0`
    #[clap(long, multiple_values = true)]
    pub(crate) args: Vec<ArgWithType>,

    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u64 u128 bool address vector true false signer`
    #[clap(long, multiple_values = true)]
    pub(crate) type_args: Vec<MoveType>,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) compile_proposal_args: CompileProposalArgs,
}

impl TryFrom<&ArgWithType> for TransactionArgument {
    type Error = CliError;

    fn try_from(arg: &ArgWithType) -> Result<Self, Self::Error> {
        let txn_arg = match arg._ty {
            FunctionArgType::Address => TransactionArgument::Address(
                bcs::from_bytes(&arg.arg)
                    .map_err(|err| CliError::UnableToParse("address", err.to_string()))?,
            ),
            FunctionArgType::Bool => TransactionArgument::Bool(
                bcs::from_bytes(&arg.arg)
                    .map_err(|err| CliError::UnableToParse("bool", err.to_string()))?,
            ),
            FunctionArgType::Hex => TransactionArgument::U8Vector(
                bcs::from_bytes(&arg.arg)
                    .map_err(|err| CliError::UnableToParse("hex", err.to_string()))?,
            ),
            FunctionArgType::String => TransactionArgument::U8Vector(
                bcs::from_bytes(&arg.arg)
                    .map_err(|err| CliError::UnableToParse("string", err.to_string()))?,
            ),
            FunctionArgType::U128 => TransactionArgument::U128(
                bcs::from_bytes(&arg.arg)
                    .map_err(|err| CliError::UnableToParse("u128", err.to_string()))?,
            ),
            FunctionArgType::U64 => TransactionArgument::U64(
                bcs::from_bytes(&arg.arg)
                    .map_err(|err| CliError::UnableToParse("u64", err.to_string()))?,
            ),
            FunctionArgType::U8 => TransactionArgument::U8(
                bcs::from_bytes(&arg.arg)
                    .map_err(|err| CliError::UnableToParse("u8", err.to_string()))?,
            ),
        };

        Ok(txn_arg)
    }
}

#[async_trait]
impl CliCommand<TransactionSummary> for ExecuteProposal {
    fn command_name(&self) -> &'static str {
        "ExecuteProposal"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let (bytecode, _script_hash) = self.compile_proposal_args.compile()?;
        // TODO: Check hash so we don't do a failed roundtrip?

        // TODO: Clean these up to be common with the run function in move
        let args = self
            .args
            .iter()
            .map(|arg_with_type| arg_with_type.try_into())
            .collect::<Result<Vec<TransactionArgument>, CliError>>()?;

        let mut type_args: Vec<TypeTag> = Vec::new();

        // These TypeArgs are used for generics
        for type_arg in self.type_args.iter().cloned() {
            let type_tag = TypeTag::try_from(type_arg)
                .map_err(|err| CliError::UnableToParse("--type-args", err.to_string()))?;
            type_args.push(type_tag)
        }

        let txn = TransactionPayload::Script(Script::new(bytecode, type_args, args));

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

    /// Git hash or branch of the framework in aptos core
    #[clap(long)]
    pub framework_git_rev: String,

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
        compile_in_temp_dir(script_path, &self.framework_git_rev, self.prompt_options)
    }
}
