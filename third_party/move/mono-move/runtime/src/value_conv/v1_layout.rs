// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Conversion from MonoMove's [`ValueLayout`] to the V1 [`MoveTypeLayout`].
//!
//! A serialized function value stores the V1 layout of every captured value,
//! so serializing one needs this conversion.

use crate::error::{RuntimeError, RuntimeInvariantViolation};
use mono_move_core::{
    FieldValueLayout, LayoutId, LayoutKind, LayoutProvider, VMInternalError, VMResult, ValueLayout,
};
use move_core_types::value::{MoveStructLayout, MoveTypeLayout};
use shared_dsa::UnorderedMap;
use std::sync::Arc;

/// Converts layout `id` to the [`MoveTypeLayout`] V1 builds for the same type.
/// Structs and enums use the runtime forms.
///
/// A struct reached along several paths is converted once and shared through
/// its `Arc`, so the result is a DAG no larger than the layout table.
///
/// V1 wraps `aggregator_v2::{Aggregator, AggregatorSnapshot,
/// DerivedStringSnapshot}` in [`MoveTypeLayout::Native`] when delayed fields
/// are enabled. MonoMove has no delayed fields, so this conversion treats them
/// as plain structs.
// TODO(security): revisit once delayed fields are supported.
// TODO(completeness): consider not constructing layouts at all if they are
// only needed for serialization. Needed before production: building the
// layout is a DoS vector.
// TODO(metering): ensure the layout is bounded.
pub(crate) fn to_v1_type_layout<T: LayoutProvider + ?Sized>(
    layouts: &T,
    id: LayoutId,
) -> VMResult<MoveTypeLayout> {
    Converter {
        layouts,
        struct_cache: UnorderedMap::new(),
    }
    .convert(id)
}

struct Converter<'a, T: ?Sized> {
    layouts: &'a T,
    // Cache for same instance of a struct identity, shared via the pointer.
    // Never iterated, and keyed by layout ids an attacker cannot choose.
    struct_cache: UnorderedMap<LayoutId, Arc<MoveStructLayout>>,
}

impl<'a, T: LayoutProvider + ?Sized> Converter<'a, T> {
    // TODO(metering): decide if we can avoid allocations and serialize the
    // layout directly, and switch to a non-recursive implementation.
    fn convert(&mut self, id: LayoutId) -> VMResult<MoveTypeLayout> {
        let layout = self.layout(id)?;
        Ok(match &layout.kind {
            LayoutKind::Bool => MoveTypeLayout::Bool,
            LayoutKind::UnsignedInt => match layout.size {
                1 => MoveTypeLayout::U8,
                2 => MoveTypeLayout::U16,
                4 => MoveTypeLayout::U32,
                8 => MoveTypeLayout::U64,
                16 => MoveTypeLayout::U128,
                32 => MoveTypeLayout::U256,
                size => return Err(bad_int_width(size)),
            },
            LayoutKind::SignedInt => match layout.size {
                1 => MoveTypeLayout::I8,
                2 => MoveTypeLayout::I16,
                4 => MoveTypeLayout::I32,
                8 => MoveTypeLayout::I64,
                16 => MoveTypeLayout::I128,
                32 => MoveTypeLayout::I256,
                size => return Err(bad_int_width(size)),
            },
            LayoutKind::Address => MoveTypeLayout::Address,
            LayoutKind::Signer => MoveTypeLayout::Signer,
            LayoutKind::Function => MoveTypeLayout::Function,
            LayoutKind::Vector { elem_id, .. } => {
                MoveTypeLayout::Vector(Box::new(self.convert(*elem_id)?))
            },
            LayoutKind::Struct { fields } => MoveTypeLayout::Struct(self.shared(id, |this| {
                Ok(MoveStructLayout::Runtime(this.convert_fields(fields)?))
            })?),
            LayoutKind::FrozenEnum { variants, .. } => {
                MoveTypeLayout::Struct(self.shared(id, |this| {
                    let variants = variants
                        .iter()
                        .map(|variant| {
                            let LayoutKind::Struct { fields } = &this.layout(variant.id)?.kind
                            else {
                                return Err(unreachable(
                                    "an enum variant body is a struct layout".to_string(),
                                ));
                            };
                            this.convert_fields(fields)
                        })
                        .collect::<VMResult<Vec<_>>>()?;
                    Ok(MoveStructLayout::RuntimeVariants(variants))
                })?)
            },
            LayoutKind::Ref => {
                return Err(unreachable("a reference has no MoveTypeLayout".to_string()))
            },
        })
    }

    fn convert_fields(&mut self, fields: &[FieldValueLayout]) -> VMResult<Vec<MoveTypeLayout>> {
        fields
            .iter()
            .map(|field| self.convert(field.id))
            .collect::<VMResult<Vec<_>>>()
    }

    /// Returns the converted struct or enum `id`, building it on first use.
    fn shared(
        &mut self,
        id: LayoutId,
        build: impl FnOnce(&mut Self) -> VMResult<MoveStructLayout>,
    ) -> VMResult<Arc<MoveStructLayout>> {
        if let Some(converted) = self.struct_cache.get(&id) {
            return Ok(Arc::clone(converted));
        }
        let converted = Arc::new(build(self)?);
        self.struct_cache.insert(id, Arc::clone(&converted));
        Ok(converted)
    }

    fn layout(&self, id: LayoutId) -> VMResult<&'a ValueLayout> {
        self.layouts.layout(id).ok_or_else(|| {
            VMInternalError::new(RuntimeError::InvariantViolation(
                RuntimeInvariantViolation::ValueLayoutNotFound,
            ))
        })
    }
}

fn bad_int_width(size: u32) -> VMInternalError {
    unreachable(format!("{size} is not a Move integer width"))
}

fn unreachable(what: String) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::Unreachable(what),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_core::{
        interner::InternedIdentifier,
        types::{U64_TY, U8_TY},
        value_layout::{
            ADDRESS_LAYOUT_ID, BOOL_LAYOUT_ID, FUNCTION_LAYOUT_ID, I128_LAYOUT_ID, I16_LAYOUT_ID,
            I256_LAYOUT_ID, I32_LAYOUT_ID, I64_LAYOUT_ID, I8_LAYOUT_ID, REF_LAYOUT_ID,
            SIGNER_LAYOUT_ID, U128_LAYOUT_ID, U16_LAYOUT_ID, U256_LAYOUT_ID, U32_LAYOUT_ID,
            U64_LAYOUT_ID, U8_LAYOUT_ID,
        },
        DescriptorId, LayoutFlags, ValueLayoutTable, VariantValueLayout,
    };

    fn field(name: &'static str, offset: u32, id: LayoutId) -> FieldValueLayout {
        FieldValueLayout {
            offset,
            id,
            name: InternedIdentifier::from_static(name),
        }
    }

    fn struct_layout(size: u32, fields: Vec<FieldValueLayout>) -> ValueLayout {
        ValueLayout::struct_layout(
            None,
            size,
            8,
            None,
            LayoutFlags::empty(),
            fields.into_boxed_slice(),
        )
    }

    #[test]
    fn convert_primitives() {
        let layouts = ValueLayoutTable::new();
        let cases = [
            (BOOL_LAYOUT_ID, MoveTypeLayout::Bool),
            (U8_LAYOUT_ID, MoveTypeLayout::U8),
            (U16_LAYOUT_ID, MoveTypeLayout::U16),
            (U32_LAYOUT_ID, MoveTypeLayout::U32),
            (U64_LAYOUT_ID, MoveTypeLayout::U64),
            (U128_LAYOUT_ID, MoveTypeLayout::U128),
            (U256_LAYOUT_ID, MoveTypeLayout::U256),
            (I8_LAYOUT_ID, MoveTypeLayout::I8),
            (I16_LAYOUT_ID, MoveTypeLayout::I16),
            (I32_LAYOUT_ID, MoveTypeLayout::I32),
            (I64_LAYOUT_ID, MoveTypeLayout::I64),
            (I128_LAYOUT_ID, MoveTypeLayout::I128),
            (I256_LAYOUT_ID, MoveTypeLayout::I256),
            (ADDRESS_LAYOUT_ID, MoveTypeLayout::Address),
            (SIGNER_LAYOUT_ID, MoveTypeLayout::Signer),
            (FUNCTION_LAYOUT_ID, MoveTypeLayout::Function),
        ];
        for (id, expected) in cases {
            assert_eq!(to_v1_type_layout(&layouts, id).unwrap(), expected);
        }
    }

    #[test]
    fn convert_rejects_reference() {
        let layouts = ValueLayoutTable::new();
        assert!(to_v1_type_layout(&layouts, REF_LAYOUT_ID).is_err());
    }

    #[test]
    fn convert_vector() {
        let mut layouts = ValueLayoutTable::new();
        let id = layouts.push(ValueLayout::vector(U8_TY, U8_LAYOUT_ID, DescriptorId(2)));
        assert_eq!(
            to_v1_type_layout(&layouts, id).unwrap(),
            MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8))
        );
    }

    #[test]
    fn convert_struct() {
        let mut layouts = ValueLayoutTable::new();
        let id = layouts.push_anonymous(struct_layout(16, vec![
            field("a", 0, U64_LAYOUT_ID),
            field("b", 8, BOOL_LAYOUT_ID),
        ]));
        let expected = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
            MoveTypeLayout::Bool,
        ])));
        assert_eq!(to_v1_type_layout(&layouts, id).unwrap(), expected);
    }

    #[test]
    fn convert_enum() {
        let mut layouts = ValueLayoutTable::new();
        let none = layouts.push_anonymous(struct_layout(0, vec![]));
        let some = layouts.push_anonymous(struct_layout(8, vec![field("x", 0, U64_LAYOUT_ID)]));
        let id = layouts.push(ValueLayout::frozen_enum(
            U64_TY,
            DescriptorId(3),
            vec![
                VariantValueLayout {
                    name: InternedIdentifier::from_static("None"),
                    id: none,
                },
                VariantValueLayout {
                    name: InternedIdentifier::from_static("Some"),
                    id: some,
                },
            ]
            .into_boxed_slice(),
            16,
        ));
        let expected = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::RuntimeVariants(vec![
            vec![],
            vec![MoveTypeLayout::U64],
        ])));
        assert_eq!(to_v1_type_layout(&layouts, id).unwrap(), expected);
    }

    #[test]
    fn convert_shares_repeated_struct() {
        let mut layouts = ValueLayoutTable::new();
        let inner = layouts.push_anonymous(struct_layout(8, vec![field("x", 0, U64_LAYOUT_ID)]));
        let outer = layouts.push_anonymous(struct_layout(16, vec![
            field("a", 0, inner),
            field("b", 8, inner),
        ]));

        let MoveTypeLayout::Struct(outer) = to_v1_type_layout(&layouts, outer).unwrap() else {
            panic!("expected a struct layout");
        };
        let MoveStructLayout::Runtime(fields) = outer.as_ref() else {
            panic!("expected a runtime struct layout");
        };
        let [MoveTypeLayout::Struct(a), MoveTypeLayout::Struct(b)] = fields.as_slice() else {
            panic!("expected two struct fields");
        };
        assert!(Arc::ptr_eq(a, b));
    }
}
