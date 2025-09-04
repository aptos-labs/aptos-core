// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::serde_helper::bcs_utils::bcs_size_of_byte_array;
use move_binary_format::errors::PartialVMResult;
use move_vm_types::delayed_values::{
    delayed_field_id::{DelayedFieldID, ExtractWidth},
    error::code_invariant_error,
};
use once_cell::sync::Lazy;

static U64_MAX_DIGITS: Lazy<usize> = Lazy::new(|| u64::MAX.to_string().len());
static U128_MAX_DIGITS: Lazy<usize> = Lazy::new(|| u128::MAX.to_string().len());

pub fn calculate_width_for_constant_string(byte_len: usize) -> usize {
    // we need to be able to store it both raw, as well as when it is exchanged with u64 DelayedFieldID.
    // so the width needs to be larger of the two options
    (bcs_size_of_byte_array(byte_len) + 1) // 1 is for empty padding serialized length
        .max(*U64_MAX_DIGITS + 2) // largest exchanged u64 DelayedFieldID is u64 max digits, plus 1 for each of the value and padding serialized length
}

pub fn calculate_width_for_integer_embedded_string(
    rest_byte_len: usize,
    snapshot_id: DelayedFieldID,
) -> PartialVMResult<usize> {
    // we need to translate byte width into string character width.
    let max_snapshot_string_width = match snapshot_id.extract_width() {
        8 => *U64_MAX_DIGITS,
        16 => *U128_MAX_DIGITS,
        x => {
            return Err(code_invariant_error(format!(
                "unexpected width ({x}) for integer snapshot id: {snapshot_id:?}"
            )))
        },
    };

    Ok(bcs_size_of_byte_array(rest_byte_len + max_snapshot_string_width) + 1) // 1 for padding length
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotToStringFormula {
    Concat { prefix: Vec<u8>, suffix: Vec<u8> },
}

impl SnapshotToStringFormula {
    pub fn apply_to(&self, base: u128) -> Vec<u8> {
        match self {
            SnapshotToStringFormula::Concat { prefix, suffix } => {
                let middle_string = base.to_string();
                let middle = middle_string.as_bytes();
                let mut result = Vec::with_capacity(prefix.len() + middle.len() + suffix.len());
                result.extend(prefix);
                result.extend(middle);
                result.extend(suffix);
                result
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;
    use move_vm_types::delayed_values::derived_string_snapshot::bytes_and_width_to_derived_string_struct;

    #[test]
    fn test_width_calculation() {
        for data in [
            vec![],
            vec![60; 1],
            vec![60; 5],
            vec![60; 127],
            vec![60; 128],
            vec![60; 129],
        ] {
            {
                let width = calculate_width_for_constant_string(data.len());
                assert_ok!(bytes_and_width_to_derived_string_struct(
                    data.clone(),
                    width
                ));
                assert_ok!(DelayedFieldID::new_with_width(u32::MAX, width as u32)
                    .into_derived_string_struct());
            }
            {
                let width = assert_ok!(calculate_width_for_integer_embedded_string(
                    data.len(),
                    DelayedFieldID::new_with_width(u32::MAX, 8)
                ));
                assert_ok!(bytes_and_width_to_derived_string_struct(
                    SnapshotToStringFormula::Concat {
                        prefix: data.clone(),
                        suffix: vec![]
                    }
                    .apply_to(u64::MAX as u128),
                    width
                ));
                assert_ok!(DelayedFieldID::new_with_width(u32::MAX, width as u32)
                    .into_derived_string_struct());
            }
            {
                let width = assert_ok!(calculate_width_for_integer_embedded_string(
                    data.len(),
                    DelayedFieldID::new_with_width(u32::MAX, 16)
                ));
                assert_ok!(bytes_and_width_to_derived_string_struct(
                    SnapshotToStringFormula::Concat {
                        prefix: data.clone(),
                        suffix: vec![]
                    }
                    .apply_to(u128::MAX),
                    width
                ));
                assert_ok!(DelayedFieldID::new_with_width(u32::MAX, width as u32)
                    .into_derived_string_struct());
            }
        }
    }
}
