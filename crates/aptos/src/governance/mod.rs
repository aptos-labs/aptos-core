// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliTypedResult, TransactionOptions};
use crate::{CliCommand, CliResult};
use aptos_rest_client::Transaction;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

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
    pub(crate) pool_address: AccountAddress,
    /// Execution hash of the script to be voted on
    #[clap(long)]
    pub(crate) execution_hash: String,
    /// Code location of the script to be voted on
    #[clap(long)]
    pub(crate) code_location: String,
    /// Title of proposal
    #[clap(long)]
    pub(crate) title: String,
    /// Description of proposal
    #[clap(long)]
    pub(crate) description: String,
}

#[async_trait]
impl CliCommand<Transaction> for SubmitProposal {
    fn command_name(&self) -> &'static str {
        "SubmitProposal"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        let execution_hash = bcs::to_bytes(&hex::decode(self.execution_hash)?)?;

        // TODO: Do I want to upload the proposal here?
        // TODO: Validate code location
        // TODO: Allow Pool Address to use profile aliases

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "aptos_governance",
                "create_proposal",
                vec![],
                vec![
                    bcs::to_bytes(&self.pool_address)?,
                    bcs::to_bytes(&execution_hash)?,
                    bcs::to_bytes(&self.code_location)?,
                    bcs::to_bytes(&self.title)?,
                    bcs::to_bytes(&self.description)?,
                ],
            )
            .await
    }
}

#[derive(Parser)]
pub struct SubmitVote {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Delegated pool address to vote on behalf of
    #[clap(long)]
    pub(crate) pool_address: AccountAddress,
    /// Id of proposal to vote on
    #[clap(long)]
    pub(crate) proposal_id: u64,
    /// TODO: Not sure what this field is for
    #[clap(long)]
    pub(crate) should_pass: bool,
}

#[async_trait]
impl CliCommand<Transaction> for SubmitVote {
    fn command_name(&self) -> &'static str {
        "SubmitVote"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        // TODO: Verify execution hash?
        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "aptos_governance",
                "vote",
                vec![],
                vec![
                    bcs::to_bytes(&self.pool_address)?,
                    bcs::to_bytes(&self.proposal_id)?,
                    bcs::to_bytes(&self.should_pass)?,
                ],
            )
            .await
    }
}
