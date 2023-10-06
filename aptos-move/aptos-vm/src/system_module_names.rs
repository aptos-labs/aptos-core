// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Names of modules, functions, and types used by Aptos System.

use aptos_types::account_config;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use once_cell::sync::Lazy;

// Data to resolve basic account and transaction flow functions and structs
/// The ModuleId for the aptos block module
pub static BLOCK_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        Identifier::new("block").unwrap(),
    )
});

// TZ: TODO: remove these except for the block-related names
// Names for special functions and structs
pub const SCRIPT_PROLOGUE_NAME: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("script_prologue").unwrap());
pub const MULTI_AGENT_SCRIPT_PROLOGUE_NAME: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("multi_agent_script_prologue").unwrap());
pub const MODULE_PROLOGUE_NAME: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("module_prologue").unwrap());
pub const USER_EPILOGUE_NAME: Lazy<Identifier> = Lazy::new(|| Identifier::new("epilogue").unwrap());
pub const BLOCK_PROLOGUE: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("block_prologue").unwrap());

pub static MULTISIG_ACCOUNT_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        Identifier::new("multisig_account").unwrap(),
    )
});
pub const VALIDATE_MULTISIG_TRANSACTION: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("validate_multisig_transaction").unwrap());
pub const GET_NEXT_TRANSACTION_PAYLOAD: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("get_next_transaction_payload").unwrap());
pub const SUCCESSFUL_TRANSACTION_EXECUTION_CLEANUP: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("successful_transaction_execution_cleanup").unwrap());
pub const FAILED_TRANSACTION_EXECUTION_CLEANUP: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("failed_transaction_execution_cleanup").unwrap());

pub static TRANSACTION_FEE_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        Identifier::new("transaction_fee").unwrap(),
    )
});

pub const EMIT_FEE_STATEMENT: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("emit_fee_statement").unwrap());
