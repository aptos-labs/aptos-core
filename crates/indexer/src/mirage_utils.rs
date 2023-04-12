// Copyright © Aptos Foundation

use crate::util::{hash_str, truncate_str};

pub const MIRAGE_ADDRESS: &str = "0x701cdfb5e87de07beacc835c2bcf03428ae124b869e601f23e4e59ab645bf699";
pub const MIRAGE_TYPE_MAX_LENGTH: usize = 512;

pub fn trunc_type(move_type: &str) -> String {
    truncate_str(move_type, MIRAGE_TYPE_MAX_LENGTH)
}

pub fn hash_types(collateral_type: &str, borrow_type: &str) -> String {
    hash_str(&format!("<{},{}>", &trunc_type(collateral_type), &trunc_type(borrow_type)))
}
