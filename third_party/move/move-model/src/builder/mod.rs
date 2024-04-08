// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

mod builtins;
mod exp_builder;
mod macros;
pub(crate) mod model_builder;
pub(crate) mod module_builder;

pub(crate) fn pluralize(s: &str, n: usize) -> String {
    // Should add special cases here as we come along them
    if n != 1 {
        s.to_owned() + "s"
    } else {
        s.to_owned()
    }
}
