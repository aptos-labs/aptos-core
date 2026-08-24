// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Converts a finished MonoMove transaction's events into Aptos
//! [`ContractEvent`]s ([`to_contract_events`]).
//!
//! [`ContractEvent`]: aptos_types::contract_event::ContractEvent

pub mod error;
pub mod events;

pub use error::OutputError;
pub use events::{to_contract_events, to_contract_events_from_store};
