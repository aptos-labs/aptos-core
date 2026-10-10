// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A source comment before the first declaration.
pub fn mask_low(value: u32) -> u32 {
    value ^ 1
}

/* A block comment between declarations. */
pub fn mask_high(value: u32) -> u32 {
    value ^ 2
}

// A trailing source comment.
