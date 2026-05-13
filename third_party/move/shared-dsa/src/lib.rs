// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Collection of data structures and algorithms, for shared use across
//! various crates.

mod unordered_map;
mod unordered_set;

pub use std::collections::hash_map::{Entry, OccupiedEntry, VacantEntry};
pub use unordered_map::UnorderedMap;
pub use unordered_set::UnorderedSet;
