// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prefix Consensus SlotManager and associated types.
//!
//! The SlotManager is the main orchestrator for prefix consensus. It replaces
//! RoundManager, running one slot at a time: broadcasting proposals, collecting
//! them via SlotState, spawning SPC, building blocks from v_high, and advancing.

pub mod slot_manager;
