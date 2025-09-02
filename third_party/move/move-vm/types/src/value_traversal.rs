// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delayed_values::error::code_invariant_error,
    values::{Closure, Container, Value, ValueImpl},
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use std::collections::HashSet;

// TODO[agg_v2](cleanup): This is a temporary traversal which collects
//   identifiers stored in values. We do not use ValueVisitor because
//   we want to allow for errors. It can be optimized away.
pub fn find_identifiers_in_value(
    value: &Value,
    identifiers: &mut HashSet<u64>,
) -> PartialVMResult<()> {
    find_identifiers_in_value_impl(&value.0, identifiers)
}

fn find_identifiers_in_value_impl(
    value: &ValueImpl,
    identifiers: &mut HashSet<u64>,
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

        ValueImpl::Container(c) => match &**c {
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

        ValueImpl::ClosureValue(c) => {
            let Closure(_, captured) = &**c;
            for val in captured.iter() {
                find_identifiers_in_value_impl(val, identifiers)?;
            }
        },

        ValueImpl::Invalid | ValueImpl::ContainerRef(_) | ValueImpl::IndexedRef(_) => {
            return Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            ))
        },

        ValueImpl::DelayedFieldID { id } => {
            if !identifiers.insert(id.as_u64()) {
                return Err(code_invariant_error(
                    "Duplicated identifiers for Move value".to_string(),
                ));
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{delayed_values::delayed_field_id::DelayedFieldID, values::Struct};
    use claims::{assert_err, assert_ok, assert_some};
    use move_core_types::account_address::AccountAddress;

    #[test]
    fn test_traversal_in_invalid_value() {
        let a = Value::master_signer_reference(AccountAddress::random());
        assert_err!(find_identifiers_in_value(&a, &mut HashSet::new()));
    }

    #[test]
    fn test_traversal_in_value_without_delayed_fields() {
        let a = Value::u64(10);
        let b = Value::vector_u32(vec![1, 2, 3, 4]);
        let c = Value::struct_(Struct::pack(vec![a, b]));
        let d = Value::u128(20);
        let e = Value::struct_(Struct::pack(vec![c, d]));

        let mut ids = HashSet::new();
        assert_ok!(find_identifiers_in_value(&e, &mut ids));
        assert!(ids.is_empty())
    }

    #[test]
    fn test_traversal_in_value_with_delayed_fields() {
        let a = Value::delayed_value(DelayedFieldID::from(0));
        let b = Value::vector_u32(vec![1, 2, 3, 4]);
        let c = Value::struct_(Struct::pack(vec![a, b]));

        let x = Value::delayed_value(DelayedFieldID::from(1));
        let y = Value::delayed_value(DelayedFieldID::from(2));
        let z = Value::delayed_value(DelayedFieldID::from(3));
        let d = Value::vector_for_testing_only(vec![x, y, z]);

        let e = Value::struct_(Struct::pack(vec![c, d]));

        let mut ids = HashSet::new();
        assert_ok!(find_identifiers_in_value(&e, &mut ids));
        assert_eq!(ids.len(), 4);

        assert_some!(ids.get(&0));
        assert_some!(ids.get(&1));
        assert_some!(ids.get(&2));
        assert_some!(ids.get(&3));
    }

    #[test]
    fn test_duplicated_ids() {
        let a = Value::delayed_value(DelayedFieldID::from(0));
        let b = Value::delayed_value(DelayedFieldID::from(0));
        let c = Value::struct_(Struct::pack(vec![a, b]));

        let mut ids = HashSet::new();
        assert_err!(find_identifiers_in_value(&c, &mut ids));
    }
}
