use crate::{
    common::types::{TransactionOptions, TransactionSummary},
    CliCommand, CliResult, CliTypedResult,
};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::{Parser, Subcommand};

/// Tool for multisig account operations.
#[derive(Debug, Subcommand)]
pub enum MultisigTool {
    Create(CreateMultisig),
}

impl MultisigTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MultisigTool::Create(tool) => tool.execute_serialized().await,
        }
    }
}

/// Create a multisig account.
#[derive(Debug, Parser)]
pub struct CreateMultisig {
    /// Hex account address(es) to add as owners, each prefixed with "0x" and separated by spaces
    #[clap(short, long, multiple(true), parse(try_from_str=crate::common::types::load_account_arg))]
    pub additional_owners: Vec<AccountAddress>,
    /// Number of signatures required to approve a transaction
    #[clap(short, long)]
    pub num_signatures_required: u64,
    #[clap(flatten)]
    pub txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for CreateMultisig {
    fn command_name(&self) -> &'static str {
        "CreateMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        // Generate multisig account creation transaction payload, ignoring metadata map.
        let payload = aptos_cached_packages::aptos_stdlib::multisig_account_create_with_owners(
            self.additional_owners,
            self.num_signatures_required,
            vec![], // Metadata keys not supported here.
            vec![], // Metadata values not supported here.
        );
        // Profile gas if needed.
        if self.txn_options.profile_gas {
            self.txn_options.profile_gas(payload).await
        } else {
            // Otherwise submit the transaction.
            self.txn_options
                .submit_transaction(payload)
                .await
                .map(TransactionSummary::from)
        }
    }
}
