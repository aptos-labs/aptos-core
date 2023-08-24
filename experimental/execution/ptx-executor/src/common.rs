// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use aptos_types::state_store::state_key::StateKey;

pub(crate) type TxnIdx = usize;

pub(crate) type VersionedKey = (StateKey, TxnIdx);

pub(crate) const BASE_VERSION: TxnIdx = TxnIdx::MAX;
pub(crate) const EXPECTANT_BLOCK_SIZE: usize = 10_000;
pub(crate) const EXPECTANT_BLOCK_KEYS: usize = 100_000;
