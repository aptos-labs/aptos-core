// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
