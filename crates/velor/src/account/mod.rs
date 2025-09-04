// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliResult};
use clap::Subcommand;

pub mod balance;
pub mod create;
pub mod create_resource_account;
pub mod derive_resource_account;
pub mod fund;
pub mod key_rotation;
pub mod list;
pub mod multisig_account;
pub mod transfer;

/// Tool for interacting with accounts
///
/// This tool is used to create accounts, get information about the
/// account's resources, and transfer resources between accounts.
#[derive(Debug, Subcommand)]
pub enum AccountTool {
    Create(create::CreateAccount),
    CreateResourceAccount(create_resource_account::CreateResourceAccount),
    DeriveResourceAccountAddress(derive_resource_account::DeriveResourceAccount),
    FundWithFaucet(fund::FundWithFaucet),
    Balance(balance::Balance),
    List(list::ListAccount),
    LookupAddress(key_rotation::LookupAddress),
    RotateKey(key_rotation::RotateKey),
    Transfer(transfer::TransferCoins),
}

impl AccountTool {
    pub async fn execute(self) -> CliResult {
        match self {
            AccountTool::Create(tool) => tool.execute_serialized().await,
            AccountTool::CreateResourceAccount(tool) => tool.execute_serialized().await,
            AccountTool::DeriveResourceAccountAddress(tool) => tool.execute_serialized().await,
            AccountTool::FundWithFaucet(tool) => tool.execute_serialized().await,
            AccountTool::Balance(tool) => tool.execute_serialized().await,
            AccountTool::List(tool) => tool.execute_serialized().await,
            AccountTool::LookupAddress(tool) => tool.execute_serialized().await,
            AccountTool::RotateKey(tool) => tool.execute_serialized().await,
            AccountTool::Transfer(tool) => tool.execute_serialized().await,
        }
    }
}

/// Tool for interacting with multisig accounts
#[derive(Debug, Subcommand)]
pub enum MultisigAccountTool {
    Approve(multisig_account::Approve),
    Create(multisig_account::Create),
    CreateTransaction(multisig_account::CreateTransaction),
    Execute(multisig_account::Execute),
    ExecuteReject(multisig_account::ExecuteReject),
    ExecuteWithPayload(multisig_account::ExecuteWithPayload),
    Reject(multisig_account::Reject),
    VerifyProposal(multisig_account::VerifyProposal),
}

impl MultisigAccountTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MultisigAccountTool::Approve(tool) => tool.execute_serialized().await,
            MultisigAccountTool::Create(tool) => tool.execute_serialized().await,
            MultisigAccountTool::CreateTransaction(tool) => tool.execute_serialized().await,
            MultisigAccountTool::Execute(tool) => tool.execute_serialized().await,
            MultisigAccountTool::ExecuteReject(tool) => tool.execute_serialized().await,
            MultisigAccountTool::ExecuteWithPayload(tool) => tool.execute_serialized().await,
            MultisigAccountTool::Reject(tool) => tool.execute_serialized().await,
            MultisigAccountTool::VerifyProposal(tool) => tool.execute_serialized().await,
        }
    }
}
