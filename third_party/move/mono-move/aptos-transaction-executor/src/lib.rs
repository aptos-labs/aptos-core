// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Aptos transaction-execution layer (the legacy AptosVM's role) on the
//! MonoMove VM.
//!
//! Executes user transactions single-threaded — prologue, payload, epilogue —
//! producing an unmaterialized [`TxnOutcome`].

// Modules stay private; anything public at the crate level is re-exported
// here.
mod calls;
mod errors;
mod executor;
mod materialize;
mod natives;
mod outcome;
mod providers;
mod system_txns;
mod user_txn;

pub use errors::{
    DiscardReason, ExecutionStage, ExecutionStatus, MaterializationError, MoveExecutionFailure,
    NoEffectsReason, PreExecutionCheckFailure, SystemTxnFailure,
};
pub use executor::AptosTransactionExecutor;
pub use natives::production_natives;
pub use outcome::TxnOutcome;
pub use providers::{decode_group_members, AptosDataProvider, GroupMembers};
