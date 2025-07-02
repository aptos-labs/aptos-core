// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delayed_values::error::code_invariant_error,
    values::{walk_preorder, Container, Value, ValueImpl},
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use std::collections::HashSet;

/// For a given VM value, traverses it to find all delayed fields. For each delayed field, its ID
/// is added to the set. In case of duplicates, an error is returned. Value must be serializable,
/// i.e., do not contain references.
pub fn find_delayed_field_ids_in_values(
    value: &Value,
    identifiers: &mut HashSet<u64>,
) -> PartialVMResult<()> {
    walk_preorder(&value.0, |value, _| {
        use ValueImpl as V;

        match value {
            V::DelayedFieldID { id } => {
                if !identifiers.insert(id.as_u64()) {
                    return Err(code_invariant_error(
                        "Duplicated identifiers for Move value".to_string(),
                    ));
                }
            },

            // Irrelevant.
            V::U8(_)
            | V::U16(_)
            | V::U32(_)
            | V::U64(_)
            | V::U128(_)
            | V::U256(_)
            | V::Bool(_)
            | V::Address(_)
            | V::ClosureValue(_)
            | V::Container(Container::Vec(_))
            | V::Container(Container::Struct(_))
            | V::Container(Container::VecU8(_))
            | V::Container(Container::VecU16(_))
            | V::Container(Container::VecU32(_))
            | V::Container(Container::VecU64(_))
            | V::Container(Container::VecU128(_))
            | V::Container(Container::VecU256(_))
            | V::Container(Container::VecBool(_))
            | V::Container(Container::VecAddress(_)) => (),

            // There variants should not be stored, so they are unreachable.
            V::Invalid
            | V::Container(Container::Locals(_))
            | V::ContainerRef(_)
            | V::IndexedRef(_) => {
                return Err(PartialVMError::new(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                ))
            },
        }
        Ok(())
    })?;
    Ok(())
}

pub fn check_value_depth(value: &Value, limit: u64) -> PartialVMResult<()> {
    walk_preorder(&value.0, |value, depth| {
        if depth > limit {
            return Err(PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED));
        }

        use ValueImpl as V;
        match value {
            // Irrelevant.
            V::U8(_)
            | V::U16(_)
            | V::U32(_)
            | V::U64(_)
            | V::U128(_)
            | V::U256(_)
            | V::Bool(_)
            | V::Address(_)
            | V::ClosureValue(_)
            | V::DelayedFieldID { .. }
            | V::Container(Container::Vec(_))
            | V::Container(Container::Struct(_))
            | V::Container(Container::VecU8(_))
            | V::Container(Container::VecU16(_))
            | V::Container(Container::VecU32(_))
            | V::Container(Container::VecU64(_))
            | V::Container(Container::VecU128(_))
            | V::Container(Container::VecU256(_))
            | V::Container(Container::VecBool(_))
            | V::Container(Container::VecAddress(_)) => (),

            // We should not be checking the depth for these variants: depth is only check when
            // containers / closures are packed.
            V::Invalid
            | V::Container(Container::Locals(_))
            | V::ContainerRef(_)
            | V::IndexedRef(_) => {
                return Err(PartialVMError::new(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                ))
            },
        }
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{delayed_values::delayed_field_id::DelayedFieldID, values::Struct};
    use claims::{assert_err, assert_ok, assert_some};
    use move_core_types::account_address::AccountAddress;

    #[test]
    fn test_traversal_in_invalid_value() {
        let a = Value::master_signer_reference(AccountAddress::random());
        assert_err!(find_delayed_field_ids_in_values(&a, &mut HashSet::new()));
    }

    #[test]
    fn test_traversal_in_value_without_delayed_fields() {
        let a = Value::u64(10);
        let b = Value::vector_u32(vec![1, 2, 3, 4]);
        let c = Value::struct_(Struct::pack(vec![a, b]));
        let d = Value::u128(20);
        let e = Value::struct_(Struct::pack(vec![c, d]));

        let mut ids = HashSet::new();
        assert_ok!(find_delayed_field_ids_in_values(&e, &mut ids));
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
        assert_ok!(find_delayed_field_ids_in_values(&e, &mut ids));
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
        assert_err!(find_delayed_field_ids_in_values(&c, &mut ids));
    }
}
