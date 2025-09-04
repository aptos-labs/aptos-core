// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod golden_output;
mod test_context;

pub use golden_output::*;
use serde_json::Value;
pub use test_context::*;

pub fn find_value(val: &Value, filter: for<'r> fn(&'r &Value) -> bool) -> Value {
    let resources = val
        .as_array()
        .unwrap_or_else(|| panic!("expect array, but got: {}", val));
    let mut balances = resources.iter().filter(filter);
    match balances.next() {
        Some(resource) => {
            let more = balances.next();
            if let Some(val) = more {
                panic!("found multiple items by the filter: {}", pretty(val));
            }
            resource.clone()
        },
        None => {
            panic!("\ncould not find item in {}", pretty(val))
        },
    }
}

pub fn assert_json(ret: Value, expected: Value) {
    assert!(
        ret == expected,
        "\nexpected: {}, \nbut got: {}",
        pretty(&expected),
        pretty(&ret)
    )
}

pub fn pretty(val: &Value) -> String {
    serde_json::to_string_pretty(val).unwrap() + "\n"
}

/// Returns the name of the current function. This macro is used to derive the
/// name for the golden file of each test case. We remove the API version
/// (e.g. v0) from the path.
#[macro_export]
macro_rules! current_function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);

        // Remove the per-API module stuff from the name.
        let re = regex::Regex::new(r"::v[0-9]+::").unwrap();
        let name = re.replace_all(&name, "::").to_string();

        let mut strip = 3;
        if name.contains("::{{closure}}") {
            strip += 13;
        }
        name[..name.len() - strip].to_string()
    }};
}
