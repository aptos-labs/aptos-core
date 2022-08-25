// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! For now, this module stores some constants related to our implementation of Bulletproofs as a
//! Move API.

/// The maximum range we'll use Bulletproofs with is [0, 2^64). Might need to revisit this later.
pub const MAX_RANGE_BITS: usize = 64;

/// The domain separation tag used for the Merlin proof transcript inside the Bulletproof verifier implementation.
pub const APTOS_MOVE_DOMAIN_SEPARATION_TAG: &[u8] = b"AptosBulletproofs";
