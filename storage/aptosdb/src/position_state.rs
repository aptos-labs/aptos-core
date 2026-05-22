// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Versioned in-memory position state. Thin aliases over the shared
//! primitives in `aptos_storage_interface::state_store::jmt_pipeline`:
//! `PositionSlot` aliases `LeafSlot<()>` (no value-in-slot yet —
//! values live in `NativeStateStore` per-user-aggregated, and in
//! `position_db` on disk), and `PositionState` aliases
//! `ShardedJmtState<PositionSlot>`. All `extend` / `make_delta` /
//! shard layout / family management lives on the generics.
//!
//! When block-STM integration for position lands and the slot needs
//! to carry the in-memory value too, change the type parameter from
//! `()` to `PositionValue` — no other touchpoints in this module
//! should change.

#![forbid(unsafe_code)]

use aptos_storage_interface::state_store::jmt_pipeline::{LeafSlot, ShardedJmtState};

/// Position's per-entry slot — alias of the shared generic
/// [`LeafSlot`] with no in-slot value payload (`V = ()`). Carries
/// `{ state_key, value_hash }`; the actual `StateValue` bytes live
/// in `NativeStateStore` (per-user aggregated, for scanner reads)
/// and `position_db` (durable storage).
pub type PositionSlot = LeafSlot<()>;

/// Position's per-version in-memory state — a [`ShardedJmtState`]
/// over [`PositionSlot`]. All construction / extend / delta
/// operations live on the generic.
pub type PositionState = ShardedJmtState<PositionSlot>;

/// MapLayer family name used when constructing fresh position states.
pub const POSITION_STATE_FAMILY: &str = "position";
