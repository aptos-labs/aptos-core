// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod proto_converter_tests;
mod test_context;

use crate::protos::extractor;
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

pub fn convert_protubuf_txn_to_serde_value(txn: &extractor::Transaction) -> serde_json::Value {
    serde_json::from_str(&protobuf_json_mapping::print_to_string(txn).unwrap()).unwrap()
}

pub fn convert_protubuf_txn_arr_to_serde_value(
    txns: &[extractor::Transaction],
) -> serde_json::Value {
    let txns_value = txns
        .iter()
        .map(convert_protubuf_txn_to_serde_value)
        .collect::<Vec<serde_json::Value>>();
    serde_json::to_value(txns_value).unwrap()
}

pub fn pretty(val: &serde_json::Value) -> String {
    serde_json::to_string_pretty(val).unwrap() + "\n"
}
