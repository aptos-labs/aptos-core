// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::{Container, SizedID, Value, ValueImpl};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use std::{collections::HashSet, hash::Hash};

// TODO[agg_v2](cleanup): This is a temporary traversal which collects
//   identifiers stored in values. We do not use ValueVisitor because
//   we want to allow for errors. It can be optimized away.
pub fn find_identifiers_in_value<I: From<SizedID> + Hash + Eq>(
    value: &Value,
    identifiers: &mut HashSet<I>,
) -> PartialVMResult<()> {
    find_identifiers_in_value_impl(&value.0, identifiers)
}

fn find_identifiers_in_value_impl<I: From<SizedID> + Hash + Eq>(
    value: &ValueImpl,
    identifiers: &mut HashSet<I>,
) -> PartialVMResult<()> {
    match value {
        ValueImpl::U8(_)
        | ValueImpl::U16(_)
        | ValueImpl::U32(_)
        | ValueImpl::U64(_)
        | ValueImpl::U128(_)
        | ValueImpl::U256(_)
        | ValueImpl::Bool(_)
        | ValueImpl::Address(_) => {},

        ValueImpl::Container(c) => match c {
            Container::Locals(_) => {
                return Err(PartialVMError::new(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                ))
            },

            Container::VecU8(_)
            | Container::VecU64(_)
            | Container::VecU128(_)
            | Container::VecBool(_)
            | Container::VecAddress(_)
            | Container::VecU16(_)
            | Container::VecU32(_)
            | Container::VecU256(_) => {},

            Container::Vec(v) | Container::Struct(v) => {
                for val in v.borrow().iter() {
                    find_identifiers_in_value_impl(val, identifiers)?;
                }
            },
        },

        ValueImpl::Invalid | ValueImpl::ContainerRef(_) | ValueImpl::IndexedRef(_) => {
            return Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            ))
        },

        ValueImpl::Native { id } => {
            if !identifiers.insert(I::from(*id)) {
                return Err(
                    PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR)
                        .with_message("Duplicated identifiers for Move value".to_string()),
                );
            }
        },
    }
    Ok(())
}
