// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Functions whose locals have a recursive type get a "recursion through `List`"
// rejection instead of the generic "type of a local is not carried".
// Non-recursive functions in the same unit still verify.

pub enum List {
    Nil,
    Cons(u32, Box<List>),
}

// These functions have a List parameter and are rejected by name.
pub fn count(_list: List) -> u32 {
    0
}

pub fn count_false(_list: List) -> u32 {
    0
}
