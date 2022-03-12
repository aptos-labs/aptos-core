// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};

/// Struct that represents a ForceShiftEvent.
#[derive(Debug, Serialize, Deserialize)]
pub struct ForceShiftEvent {
    current_min_nonce: u64,
    shift_amount: u64,
    bits_at_shift: BitVector,
}

#[derive(Debug, Serialize, Deserialize)]
struct BitVector {
    length: u64,
    bits: Vec<bool>,
}

impl ForceShiftEvent {
    /// Get the min nonce at the time of the force shift
    pub fn current_min_nonce(&self) -> u64 {
        self.current_min_nonce
    }

    /// Get the amount that the CRSN window was shifted
    pub fn shift_amount(&self) -> u64 {
        self.shift_amount
    }

    pub fn bits_at_shift(&self) -> &[bool] {
        &self.bits_at_shift.bits
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}

impl MoveStructType for ForceShiftEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("CRSN");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ForceShiftEvent");
}
