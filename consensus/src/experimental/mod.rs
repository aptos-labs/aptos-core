// Copyright (c) Aptos
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
 *       ┌─────────►  Buffer Manager  │                 │ Network │
 *       │         │                  ◄─────────────────┤         │
 *  ┌────┴─────┐   └─────────▲────────┘    Commit Vote  └─────────┘
 *  │ Ordering │             │
 *  │ State    │   Sync Req  │
 *  │ Computer ├─────────────┘
 *  └──────────┘
 */

pub mod buffer;
pub mod buffer_item;
pub mod buffer_manager;
pub mod decoupled_execution_utils;
pub mod errors;
pub mod execution_phase;
pub mod hashable;
pub mod ordering_state_computer;
pub mod persisting_phase;
pub mod pipeline_phase;
pub mod signing_phase;

#[cfg(test)]
mod tests;
