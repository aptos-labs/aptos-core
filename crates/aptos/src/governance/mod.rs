// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    AccountAddressWrapper, CliError, CliTypedResult, PromptOptions, TransactionOptions,
};
use crate::common::utils::prompt_yes_with_override;
use crate::{CliCommand, CliResult};
use aptos_crypto::HashValue;
use aptos_rest_client::Transaction;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Formatter;

/// Tool for on-chain governance
///
#[derive(Parser)]
pub enum GovernanceTool {
    Propose(SubmitProposal),
    Vote(SubmitVote),
}

impl GovernanceTool {
    pub async fn execute(self) -> CliResult {
        use GovernanceTool::*;
        match self {
            Propose(tool) => tool.execute_serialized().await,
            Vote(tool) => tool.execute_serialized().await,
        }
    }
}

/// Submit proposal to other validators to be proposed on
#[derive(Parser)]
pub struct SubmitProposal {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Delegated pool address to submit proposal on behalf of
    #[clap(long)]
    pub(crate) pool_address: AccountAddressWrapper,
    /// Execution hash of the script to be voted on
    #[clap(long, parse(try_from_str = read_hex_hash))]
    pub(crate) execution_hash: HashValue,
    /// Code location of the script to be voted on
    #[clap(long)]
    pub(crate) metadata_url: Url,

    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<Transaction> for SubmitProposal {
    fn command_name(&self) -> &'static str {
        "SubmitProposal"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        // Validate the proposal metadata
        let client = reqwest::ClientBuilder::default()
            .tls_built_in_root_certs(true)
            .build()
            .map_err(|err| {
                CliError::UnexpectedError(format!("Failed to build HTTP client {}", err))
            })?;
        let bytes = client
            .get(self.metadata_url.clone())
            .send()
            .await
            .map_err(|err| {
                CliError::CommandArgumentError(format!(
                    "Failed to fetch metadata url {}: {}",
                    self.metadata_url, err
                ))
            })?
            .bytes()
            .await
            .map_err(|err| {
                CliError::CommandArgumentError(format!(
                    "Failed to fetch metadata url {}: {}",
                    self.metadata_url, err
                ))
            })?;
        let metadata: ProposalMetadata = serde_json::from_slice(&bytes).map_err(|err| {
            CliError::CommandArgumentError(format!(
                "Metadata is not in a proper JSON format: {}",
                err
            ))
        })?;
        let metadata_hash = HashValue::sha3_256_of(&bytes);

        println!("{}", metadata);
        prompt_yes_with_override("Do you want to submit this proposal?", self.prompt_options)?;

        // TODO: Allow Pool Address to use profile aliases
        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "aptos_governance",
                "create_proposal",
                vec![],
                vec![
                    bcs::to_bytes(&self.pool_address.account_address)?,
                    bcs::to_bytes(&self.execution_hash)?,
                    bcs::to_bytes(&self.metadata_url.to_string())?,
                    bcs::to_bytes(&metadata_hash)?,
                ],
            )
            .await
    }
}

fn read_hex_hash(str: &str) -> CliTypedResult<HashValue> {
    let hex = str.strip_prefix("0x").unwrap_or(str);
    HashValue::from_hex(hex).map_err(|err| CliError::CommandArgumentError(err.to_string()))
}

#[derive(Parser)]
pub struct SubmitVote {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Delegated pool address to vote on behalf of
    #[clap(long)]
    pub(crate) pool_address: AccountAddressWrapper,
    /// Id of proposal to vote on
    #[clap(long)]
    pub(crate) proposal_id: u64,
    /// Vote choice. True for yes. False for no.
    #[clap(long)]
    pub(crate) should_pass: bool,
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<Transaction> for SubmitVote {
    fn command_name(&self) -> &'static str {
        "SubmitVote"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        // TODO: Display details of proposal
        let vote = if self.should_pass { "Yes" } else { "No" };
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
                    bcs::to_bytes(&self.pool_address.account_address)?,
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
    script_url: String,
    script_hash: String,
}

impl std::fmt::Display for ProposalMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Proposal:\n\tTitle:{}\n\tDescription:{}\n\tScript URL:{}\n\tScript hash:{}",
            self.title, self.description, self.script_url, self.script_hash
        )
    }
}
