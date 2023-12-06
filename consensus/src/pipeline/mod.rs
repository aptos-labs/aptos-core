// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

/*
 *         ┌──────────────────┐
 *         │ 2. Signing Phase │
 *         └──────────────▲─┬─┘
 *                        │ │
 * ┌────────────────────┐ │ │ ┌─────────────────────┐
 * │ 1. Execution Phase │ │ │ │ 4. Persisting Phase │
 * └─────────────────▲─┬┘ │ │ └┬─▲──────────────────┘
 *                   │ │  │ │  │ │
 *     0. Ordered  ┌─┴─▼──┴─▼──▼─┴────┐ 3. Commit Vote  ┌─────────┐
 *        Blocks   │                  ├─────────────────►         │
 *       ┌─────────►  pipeline manager  │                 │ Network │
 *       │         │                  ◄─────────────────┤         │
 *  ┌────┴─────┐   └─────────▲────────┘    Commit Vote  └─────────┘
 *  │ Ordering │             │
 *  │ State    │   Sync Req  │
 *  │ Computer ├─────────────┘
 *  └──────────┘
 */

pub mod commit_reliable_broadcast;
pub mod decoupled_execution_utils;
pub mod errors;
pub mod execution_schedule_phase;
pub mod execution_wait_phase;
pub mod hashable;
pub mod ordering_state_computer;
pub mod persisting_phase;
pub mod pipeline_item;
pub mod pipeline_manager;
pub mod pipeline_phase;
pub mod pipeline_queue;
pub mod signing_phase;

#[cfg(test)]
mod tests;
