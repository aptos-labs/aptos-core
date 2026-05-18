// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod binary_module_loader;
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

pub(crate) fn pluralize_be(n: usize) -> String {
    if n != 1 {
        "were".to_string()
    } else {
        "was".to_string()
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
