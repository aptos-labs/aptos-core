// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Converts a finished MonoMove transaction's events into Aptos
//! [`ContractEvent`]s ([`to_contract_events`]).
//!
//! [`v1_error`] describes MonoMove's typed errors the way V1 would have, for
//! callers that must produce V1 status codes. [`v1_return_bcs`] encodes a
//! return value as V1 does.
//!
//! [`ContractEvent`]: aptos_types::contract_event::ContractEvent

pub mod error;
pub mod events;
pub mod gap;
pub mod return_values;
pub mod v1_error;

pub use error::OutputError;
pub use events::{has_new_epoch_event, to_contract_events, to_contract_events_from_store};
pub use return_values::v1_return_bcs;
