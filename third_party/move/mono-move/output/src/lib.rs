// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Converts a finished MonoMove transaction into an Aptos [`TransactionOutput`].
//!
//! [`to_transaction_output`] builds the DB-facing output (write set, events,
//! gas, status) from a transaction's [`mono_move_runtime::SessionEffects`];
//! [`to_contract_events`] and [`to_transaction_status`] are the pieces.
//!
//! Not yet modelled: storage metadata / refunds (write ops are metadata-less);
//! resource groups and delayed fields; and a faithful `ExecutionError` mapping
//! (see [`to_transaction_status`]).

pub mod error;
pub mod events;
pub mod output;

pub use error::OutputError;
pub use events::to_contract_events;
pub use output::{to_transaction_output, to_transaction_status};
