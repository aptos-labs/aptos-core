// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod proto_converter_tests;
mod test_context;

use aptos_protos::extractor::v1 as extractor;
pub use test_context::{new_test_context, TestContext};

pub(crate) mod golden_output;

/// Returns the name of the current function
#[macro_export]
macro_rules! current_function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        let mut strip = 3;
        if name.contains("::{{closure}}") {
            strip += 13;
        }
        &name[..name.len() - strip]
    }};
}

// TODO: Remove after we add back golden
#[allow(dead_code)]
pub fn pretty(txns: &[extractor::Transaction]) -> String {
    serde_json::to_string_pretty(txns).unwrap()
}
