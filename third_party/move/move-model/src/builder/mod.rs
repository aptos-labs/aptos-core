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
        if s.ends_with('y') {
            s[0..s.len() - 1].to_string() + "ies"
        } else {
            s.to_owned() + "s"
        }
    } else {
        s.to_owned()
    }
}

pub(crate) fn ith_str(n: usize) -> String {
    match n {
        0 => panic!("cannot be 0"),
        1 => "1st".to_string(),
        2 => "2nd".to_string(),
        3 => "3rd".to_string(),
        _ => format!("{}th", n),
    }
}
