// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Helpers shared by the MonoMove replay tools: `mono-move-replay` (the comparison against AptosVM)
//! and `mono-move-replay-benchmark` (the timing benchmark). Only code both use the same way lives
//! here; where the tools deliberately differ (how V1 is metered, how outputs are compared, their
//! data formats), each keeps its own.

pub mod capture;
pub mod cli;
pub mod gas;
pub mod label;
pub mod modules;

/// The message a caught panic carried.
pub fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}
