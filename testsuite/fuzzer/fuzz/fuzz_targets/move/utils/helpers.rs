// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use move_binary_format::file_format::CompiledModule;
use move_core_types::value::{MoveStructLayout, MoveTypeLayout};

#[macro_export]
macro_rules! tdbg {
    () => {
        ()
    };
    ($val:expr $(,)?) => {
        {
            if std::env::var("DEBUG").is_ok() {
                dbg!($val)
            } else {
                $val
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        {
            if std::env::var("DEBUG").is_ok() {
                dbg!($($val),+)
            } else {
                ($($val),+)
            }
        }
    };
}

pub(crate) fn is_valid_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout as L;

    match layout {
        L::Bool
        | L::U8
        | L::U16
        | L::U32
        | L::U64
        | L::U128
        | L::U256
        | L::Address
        | L::Signer
        | L::Function => true,

        L::Vector(layout) | L::Native(_, layout) => is_valid_layout(layout),
        L::Struct(MoveStructLayout::RuntimeVariants(variants)) => {
            variants.iter().all(|v| v.iter().all(is_valid_layout))
        },
        L::Struct(MoveStructLayout::Runtime(fields)) => {
            if fields.is_empty() {
                return false;
            }
            fields.iter().all(is_valid_layout)
        },
        L::Struct(_) => {
            // decorated layouts not supported
            false
        },
    }
}

pub(crate) fn compiled_module_serde(module: &CompiledModule) -> Result<(), ()> {
    let mut blob = vec![];
    module.serialize(&mut blob).map_err(|_| ())?;
    CompiledModule::deserialize(&blob).map_err(|_| ())?;
    Ok(())
}

pub(crate) fn base64url_encode_str(data: &str) -> String {
    base64::encode_config(data.as_bytes(), base64::URL_SAFE_NO_PAD)
}
