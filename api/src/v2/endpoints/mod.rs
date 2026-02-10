// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! v2 endpoint handlers.

pub mod account_transactions;
pub mod accounts;
pub mod balance;
pub mod blocks;
pub mod events;
pub mod gas_estimation;
pub mod health;
pub mod modules;
pub mod resources;
pub mod simulate;
#[cfg(feature = "api-v2-sse")]
pub mod sse;
pub mod tables;
pub mod transactions;
pub mod view;
