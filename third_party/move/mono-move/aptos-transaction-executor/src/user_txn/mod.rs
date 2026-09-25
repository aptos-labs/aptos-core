// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Execution of user transactions.

mod args;
mod entry_func;
mod execute;
mod metadata;
mod multisig;
mod pre_execution_checks;
mod script;
mod validation;
