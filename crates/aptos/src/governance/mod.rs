// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliError, CliTypedResult, PoolAddressArgs, PromptOptions, TransactionOptions,
    TransactionSummary,
};
use crate::common::utils::prompt_yes_with_override;
use crate::move_tool::{compile_move, ArgWithType, FunctionArgType};
use crate::{CliCommand, CliResult};
use aptos_crypto::HashValue;
use aptos_rest_client::{aptos_api_types::MoveType, Transaction};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{Script, TransactionPayload},
};
use async_trait::async_trait;
use clap::Parser;
use move_deps::{
    move_compiler::compiled_unit::CompiledUnitEnum,
    move_core_types::{language_storage::TypeTag, transaction_argument::TransactionArgument},
    move_package::BuildConfig,
};
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
    fmt::Formatter,
    fs,
    path::PathBuf,
};

/// Tool for on-chain governance
///
/// This tool allows voters that have stake to vote the ability to
/// propose changes to the chain, as well as vote and execute these
/// proposals.
#[derive(Parser)]
pub enum GovernanceTool {
    Propose(SubmitProposal),
    Vote(SubmitVote),
    PrepareProposal(PrepareProposal),
    ExecuteProposal(ExecuteProposal),
}

impl GovernanceTool {
    pub async fn execute(self) -> CliResult {
        use GovernanceTool::*;
        match self {
            Propose(tool) => tool.execute_serialized().await,
            Vote(tool) => tool.execute_serialized().await,
            PrepareProposal(tool) => tool.execute_serialized().await,
            ExecuteProposal(tool) => tool.execute_serialized().await,
        }
    }
}

/// Submit proposal to other validators to be proposed on
#[derive(Parser)]
pub struct SubmitProposal {
    /// Execution hash of the script to be voted on
    #[clap(long, parse(try_from_str = read_hex_hash))]
    pub(crate) execution_hash: HashValue,

    /// Code location of the script to be voted on
    #[clap(long)]
    pub(crate) metadata_url: Url,

    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) pool_address_args: PoolAddressArgs,
}

#[async_trait]
impl CliCommand<Transaction> for SubmitProposal {
    fn command_name(&self) -> &'static str {
        "SubmitProposal"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        // Validate the proposal metadata
        let (metadata, metadata_hash) = get_metadata(&self.metadata_url)?;

        println!("{}, Hash: {}", metadata, metadata_hash);
        prompt_yes_with_override("Do you want to submit this proposal?", self.prompt_options)?;

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "aptos_governance",
                "create_proposal",
                vec![],
                vec![
                    bcs::to_bytes(&self.pool_address_args.pool_address)?,
                    bcs::to_bytes(&self.execution_hash.to_hex())?,
                    bcs::to_bytes(&self.metadata_url.to_string())?,
                    bcs::to_bytes(&metadata_hash.to_hex())?,
                ],
            )
            .await
    }
}

/// Retrieve metadata and validate it
fn get_metadata(metadata_url: &Url) -> CliTypedResult<(ProposalMetadata, HashValue)> {
    let client = reqwest::ClientBuilder::default()
        .tls_built_in_root_certs(true)
        .build()
        .map_err(|err| CliError::UnexpectedError(format!("Failed to build HTTP client {}", err)))?;
    let bytes = client
        .get(metadata_url)
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

fn read_hex_hash(str: &str) -> CliTypedResult<HashValue> {
    let hex = str.strip_prefix("0x").unwrap_or(str);
    HashValue::from_hex(hex).map_err(|err| CliError::CommandArgumentError(err.to_string()))
}

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
        let vote = match (vote_yes, vote_no) {
            (true, false) => "Yes",
            (false, true) => "No",
            (_, _) => {
                return Err(CliError::CommandArgumentError(
                    "Must choose either --yes or --no".to_string(),
                ))
            }
        };

        // TODO: Display details of proposal

        prompt_yes_with_override(
            &format!("Are you sure you want to vote {}", vote),
            self.prompt_options,
        )?;

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "aptos_governance",
                "vote",
                vec![],
                vec![
                    bcs::to_bytes(&self.pool_address_args.pool_address)?,
                    bcs::to_bytes(&self.proposal_id)?,
                    bcs::to_bytes(&self.should_pass)?,
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

#[derive(Clone, Debug, Default, Serialize)]
pub struct ScriptHash {
    hash: String,
    bytecode: String,
}

#[derive(Parser)]
pub struct PrepareProposal {
    /// Path to the Move package that contains the execution script
    #[clap(long, parse(from_os_str))]
    pub path: PathBuf,
}

#[async_trait]
impl CliCommand<ScriptHash> for PrepareProposal {
    fn command_name(&self) -> &'static str {
        "PrepareProposal"
    }

    async fn execute(mut self) -> CliTypedResult<ScriptHash> {
        let build_config = BuildConfig {
            additional_named_addresses: BTreeMap::new(),
            generate_abis: false,
            generate_docs: false,
            ..Default::default()
        };

        let compiled_package = compile_move(build_config, self.path.as_path())?;
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

                s.script.serialize(&mut bytes).map_err(|err| {
                    CliError::UnexpectedError(format!("Unexpected error: {}", err))
                })?;

                Ok(ScriptHash {
                    hash: hex::encode(HashValue::sha3_256_of(bytes.as_slice()).to_vec()),
                    bytecode: hex::encode(bytes),
                })
            }
            CompiledUnitEnum::Module(_) => Err(CliError::UnexpectedError(
                "You can only execute a script, a module is not supported.".to_string(),
            )),
        }
    }
}

#[derive(Parser)]
pub struct ExecuteProposal {
    /// Path to the compiled script file
    #[clap(long, parse(from_os_str))]
    pub path: PathBuf,

    /// Hex encoded arguments separated by spaces.
    ///
    /// Example: `0x01 0x02 0x03`
    #[clap(long, multiple_values = true)]
    pub(crate) args: Vec<ArgWithType>,

    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u64 u128 bool address vector true false signer`
    #[clap(long, multiple_values = true)]
    pub(crate) type_args: Vec<MoveType>,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
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
        if !self.path.exists() {
            return Err(CliError::UnableToReadFile(
                self.path.display().to_string(),
                "Path doesn't exist".to_string(),
            ));
        }

        let code = fs::read(self.path.as_path()).map_err(|err| {
            CliError::UnableToReadFile(self.path.display().to_string(), err.to_string())
        })?;

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

        let txn = TransactionPayload::Script(Script::new(code, type_args, args));

        self.txn_options
            .submit_transaction(txn)
            .await
            .map(TransactionSummary::from)
    }
}
