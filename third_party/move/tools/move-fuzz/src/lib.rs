// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Coverage-guided fuzzer for Move packages on Aptos.
//!
//! Resolves and builds a project's packages, statically models their public API
//! to synthesize Move driver scripts, then fuzzes those scripts against an
//! in-process VM as single transactions or as dependency chains.

pub mod subexec;
pub mod utils;

pub mod common;
pub mod language;

pub mod account;
pub mod deps;
pub mod package;

pub mod simulator;
pub mod testnet;

pub mod prep;

pub mod executor;
pub mod mutate;

pub mod fuzzer;
pub mod state;

pub mod cli;
