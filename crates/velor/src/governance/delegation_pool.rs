// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::governance::{utils::*, *};
use clap::Subcommand;

/// Tool for on-chain governance from delegation pools
///
/// This tool allows voters that have stake in a delegation pool to submit proposals or vote on
/// a proposal.
#[derive(Subcommand)]
pub enum DelegationPoolTool {
    Propose(SubmitProposal),
    Vote(SubmitVote),
}

impl DelegationPoolTool {
    pub async fn execute(self) -> CliResult {
        use DelegationPoolTool::*;
        match self {
            Propose(tool) => tool.execute_serialized().await,
            Vote(tool) => tool.execute_serialized().await,
        }
    }
}

/// Submit a governance proposal
///
/// You can only submit a proposal when the remaining lockup period of this delegation pool is
/// longer than a proposal duration and you have enough voting power to meet the minimum proposing
/// threshold. If you are voting with a delegation pool which hasn't enabled partial governance
/// voting yet, this command will enable it for you.
#[derive(Parser)]
pub struct SubmitProposal {
    /// The address of the delegation pool to propose.
    #[clap(long)]
    delegation_pool_address: AccountAddress,
    #[clap(flatten)]
    pub(crate) args: SubmitProposalArgs,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProposalSubmissionSummary {
    proposal_id: Option<u64>,
    txn_summaries: Vec<TransactionSummary>,
}

#[async_trait]
impl CliCommand<ProposalSubmissionSummary> for SubmitProposal {
    fn command_name(&self) -> &'static str {
        "SubmitDelegationPoolProposal"
    }

    async fn execute(mut self) -> CliTypedResult<ProposalSubmissionSummary> {
        let mut summaries = vec![];
        if let Some(txn_summary) = delegation_pool_governance_precheck(
            &self.args.txn_options,
            self.delegation_pool_address,
        )
        .await?
        {
            summaries.push(txn_summary);
        };
        // Validate the proposal metadata
        let (script_hash, metadata_hash) = self.args.compile_proposals().await?;
        prompt_yes_with_override(
            "Do you want to submit this proposal?",
            self.args.txn_options.prompt_options,
        )?;

        let txn: Transaction = self
            .args
            .txn_options
            .submit_transaction(velor_stdlib::delegation_pool_create_proposal(
                self.delegation_pool_address,
                script_hash.to_vec(),
                self.args.metadata_url.to_string().as_bytes().to_vec(),
                metadata_hash.to_hex().as_bytes().to_vec(),
                self.args.is_multi_step,
            ))
            .await?;
        let proposal_id = extract_proposal_id(&txn)?;
        summaries.push(TransactionSummary::from(&txn));
        Ok(ProposalSubmissionSummary {
            proposal_id,
            txn_summaries: summaries,
        })
    }
}

/// Submit a vote on a proposal
///
/// Votes can only be given on proposals that are currently open for voting. You can vote
/// with `--yes` for a yes vote, and `--no` for a no vote. If you are voting with a delegation pool
/// which hasn't enabled partial governance voting yet, this command will enable it for you.
#[derive(Parser)]
pub struct SubmitVote {
    /// The address of the delegation pool to vote.
    #[clap(long)]
    delegation_pool_address: AccountAddress,

    #[clap(flatten)]
    pub(crate) args: SubmitVoteArgs,
}

#[async_trait]
impl CliCommand<Vec<TransactionSummary>> for SubmitVote {
    fn command_name(&self) -> &'static str {
        "SubmitDelegationPoolVote"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<TransactionSummary>> {
        // The vote option is a group, so only one of yes and no must be true.
        let vote = self.args.yes;
        let mut summaries: Vec<TransactionSummary> = vec![];
        if let Some(txn_summary) = delegation_pool_governance_precheck(
            &self.args.txn_options,
            self.delegation_pool_address,
        )
        .await?
        {
            summaries.push(txn_summary);
        };

        let client = &self
            .args
            .txn_options
            .rest_options
            .client(&self.args.txn_options.profile_options)?;
        let voter_address = self.args.txn_options.profile_options.account_address()?;
        let remaining_voting_power = get_remaining_voting_power(
            client,
            self.delegation_pool_address,
            voter_address,
            self.args.proposal_id,
        )
        .await?;
        if remaining_voting_power == 0 {
            return Err(CliError::CommandArgumentError(
                "Voter has no voting power left on this proposal".to_string(),
            ));
        };
        let voting_power =
            check_remaining_voting_power(remaining_voting_power, self.args.voting_power);
        prompt_yes_with_override(
            &format!(
                "Vote {} with voting power = {} from stake pool {} on proposal {}?",
                vote_to_string(vote),
                voting_power,
                self.delegation_pool_address,
                self.args.proposal_id,
            ),
            self.args.txn_options.prompt_options,
        )?;
        summaries.push(
            self.args
                .txn_options
                .submit_transaction(velor_stdlib::delegation_pool_vote(
                    self.delegation_pool_address,
                    self.args.proposal_id,
                    voting_power,
                    vote,
                ))
                .await
                .map(TransactionSummary::from)?,
        );

        Ok(summaries)
    }
}

/// Precheck before any delegation pool governance operations. Check if feature flags are enabled.
/// Also check if partial governance voting is enabled for delegation pool. If not, send a
/// transaction to enable it.
async fn delegation_pool_governance_precheck(
    txn_options: &TransactionOptions,
    pool_address: AccountAddress,
) -> CliTypedResult<Option<TransactionSummary>> {
    let client = &txn_options
        .rest_options
        .client(&txn_options.profile_options)?;
    if !is_partial_governance_voting_enabled(client).await? {
        return Err(CliError::CommandArgumentError(
            "Partial governance voting feature flag is not enabled".to_string(),
        ));
    };
    if !is_delegation_pool_partial_governance_voting_enabled(client).await? {
        return Err(CliError::CommandArgumentError(
            "Delegation pool partial governance voting feature flag is not enabled".to_string(),
        ));
    };
    if is_partial_governance_voting_enabled_for_delegation_pool(client, pool_address).await? {
        Ok(None)
    } else {
        println!("Partial governance voting for delegation pool {} hasn't been enabled yet. Enabling it now...",
                 pool_address);
        let txn_summary = txn_options
            .submit_transaction(
                velor_stdlib::delegation_pool_enable_partial_governance_voting(pool_address),
            )
            .await
            .map(TransactionSummary::from)?;
        Ok(Some(txn_summary))
    }
}

async fn is_partial_governance_voting_enabled_for_delegation_pool(
    client: &Client,
    pool_address: AccountAddress,
) -> CliTypedResult<bool> {
    let response = client
        .view_bcs_with_json_response(
            &ViewFunction {
                module: ModuleId::new(
                    AccountAddress::ONE,
                    ident_str!("delegation_pool").to_owned(),
                ),
                function: ident_str!("partial_governance_voting_enabled").to_owned(),
                ty_args: vec![],
                args: vec![bcs::to_bytes(&pool_address).unwrap()],
            },
            None,
        )
        .await?;
    response.inner()[0].as_bool().ok_or_else(|| {
        CliError::UnexpectedError(
            "Unexpected response from node when checking if partial governance_voting is \
        enabled for delegation pool"
                .to_string(),
        )
    })
}

async fn get_remaining_voting_power(
    client: &Client,
    pool_address: AccountAddress,
    voter_address: AccountAddress,
    proposal_id: u64,
) -> CliTypedResult<u64> {
    let response = client
        .view_bcs_with_json_response(
            &ViewFunction {
                module: ModuleId::new(
                    AccountAddress::ONE,
                    ident_str!("delegation_pool").to_owned(),
                ),
                function: ident_str!("calculate_and_update_remaining_voting_power").to_owned(),
                ty_args: vec![],
                args: vec![
                    bcs::to_bytes(&pool_address).unwrap(),
                    bcs::to_bytes(&voter_address).unwrap(),
                    bcs::to_bytes(&proposal_id).unwrap(),
                ],
            },
            None,
        )
        .await?;
    let remaining_voting_power_str = response.inner()[0].as_str().ok_or_else(|| {
        CliError::UnexpectedError(format!(
            "Unexpected response from node when getting remaining voting power of {}\
        in delegation pool {}",
            pool_address, voter_address
        ))
    })?;
    remaining_voting_power_str.parse().map_err(|err| {
        CliError::UnexpectedError(format!(
            "Unexpected response from node when getting remaining voting power of {}\
        in delegation pool {}: {}",
            pool_address, voter_address, err
        ))
    })
}
