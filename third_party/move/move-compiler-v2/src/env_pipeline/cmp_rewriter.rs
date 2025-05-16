// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module checks whether closure expressions are valid, which is done after type inference
//! and lambda lifting. Current checks:
//!
//! - The closure satisfies the ability requirements of it's inferred type. For the
//!   definition of closure abilities, see
//!   [AIP-112](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-112.md).
//! - The closure does not capture references, as this is currently not allowed.
//! - In a script, the closure cannot have a lambda lifted function.
//! ```

use move_model::{
    model::{GlobalEnv, StructEnv, TypeParameter},
    ty::Type,
};

pub fn rewrite(env: &GlobalEnv) {
    print!("CMP rewriter skelton added\n");
}