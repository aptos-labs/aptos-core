// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This crate extends Move's gas algebra by introducing Aptos-specific counting units.
//!
//! It provides an abstract algebra that goes beyond concrete quantities and allows
//! the representation of gas expressions.
//!
//! These expressions can be evaluated or interpreted symbolically, opening up possibilities
//! for building advanced analysis tools.

mod abstract_algebra;
mod algebra;

pub use abstract_algebra::*;
pub use algebra::*;
