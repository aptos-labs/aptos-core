// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Differential benchmark harness for MonoMove.
//!
//! Replays a real downloaded transaction's entry function on both the legacy
//! MoveVM (V1) and MonoMove (V2), compares their global-storage writes by byte
//! equality, and times each VM. See the crate README / design plan for the
//! full pipeline (dump reader, cache flattening, resolver, runners, compare).

pub mod args;
pub mod cache;
pub mod compare;
pub mod dump;
pub mod events;
pub mod extensions;
pub mod resolver;
pub mod txn;
pub mod v1;
pub mod v2;
