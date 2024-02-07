// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::serde_helper::bcs_utils::bcs_size_of_byte_array;
use move_binary_format::errors::PartialVMError;
use move_core_types::vm_status::StatusCode;
use move_vm_types::delayed_values::sized_id::SizedID;
use once_cell::sync::Lazy;

const BITS_FOR_SIZE: usize = 32;

/// Ephemeral identifier type used by delayed fields (aggregators, snapshots)
/// during execution.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DelayedFieldID {
    unique_index: u32,
    // Exact number of bytes serialized delayed field will take.
    width: u32,
}

impl DelayedFieldID {
    pub fn new_with_width(unique_index: u32, width: u32) -> Self {
        Self {
            unique_index,
            width,
        }
    }

    pub fn new_for_test_for_u64(unique_index: u32) -> Self {
        Self::new_with_width(unique_index, 8)
    }

    pub fn as_u64(&self) -> u64 {
        ((self.unique_index as u64) << BITS_FOR_SIZE) | self.width as u64
    }

    pub fn extract_width(&self) -> u32 {
        self.width
    }
}

// Used for ID generation from exchanged value/exchanges serialized value.
impl From<u64> for DelayedFieldID {
    fn from(value: u64) -> Self {
        Self {
            unique_index: u32::try_from(value >> BITS_FOR_SIZE).unwrap(),
            width: u32::try_from(value & ((1u64 << BITS_FOR_SIZE) - 1)).unwrap(),
        }
    }
}

// Used for ID generation from u32 counter with width.
impl From<(u32, u32)> for DelayedFieldID {
    fn from(value: (u32, u32)) -> Self {
        let (index, width) = value;
        Self::new_with_width(index, width)
    }
}

impl From<DelayedFieldID> for SizedID {
    fn from(id: DelayedFieldID) -> Self {
        Self::new(id.unique_index, id.width)
    }
}

impl From<SizedID> for DelayedFieldID {
    fn from(sized_id: SizedID) -> Self {
        Self::new_with_width(sized_id.id(), sized_id.serialized_size())
    }
}

// Represents something that should never happen - i.e. a code invariant error,
// which we would generally just panic, but since we are inside of the VM,
// we cannot do that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanicError {
    CodeInvariantError(String),
}

impl ToString for PanicError {
    fn to_string(&self) -> String {
        match self {
            PanicError::CodeInvariantError(e) => e.clone(),
        }
    }
}

impl From<PanicError> for PartialVMError {
    fn from(err: PanicError) -> Self {
        match err {
            PanicError::CodeInvariantError(msg) => {
                PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR)
                    .with_message(msg)
            },
        }
    }
}

pub trait ExtractUniqueIndex: Sized {
    fn extract_unique_index(&self) -> u32;
}

impl ExtractUniqueIndex for DelayedFieldID {
    fn extract_unique_index(&self) -> u32 {
        self.unique_index
    }
}

fn code_invariant_error<M: std::fmt::Debug>(message: M) -> PanicError {
    let msg = format!(
        "Delayed logic code invariant broken (there is a bug in the code), {:?}",
        message
    );
    println!("ERROR: {}", msg);
    // cannot link aptos_logger in aptos-types crate
    // error!("{}", msg);
    PanicError::CodeInvariantError(msg)
}

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
) -> Result<usize, PanicError> {
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
                assert_ok!(
                    SizedID::from(DelayedFieldID::new_with_width(u32::MAX, width as u32))
                        .into_derived_string_struct()
                );
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
                assert_ok!(
                    SizedID::from(DelayedFieldID::new_with_width(u32::MAX, width as u32))
                        .into_derived_string_struct()
                );
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
                assert_ok!(
                    SizedID::from(DelayedFieldID::new_with_width(u32::MAX, width as u32))
                        .into_derived_string_struct()
                );
            }
        }
    }
}
