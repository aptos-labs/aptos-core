// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    loaded_data::runtime_types::{StructType, Type},
    values::{
        values_impl::{Container, IndexedRef, Value},
        Locals,
    },
};
use move_core_types::value::MASTER_ADDRESS_FIELD_OFFSET;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum DebugValue {
    Invalid,
    Primitive(String),
    Address(String),
    Signer(String),
    Struct(Vec<(String, DebugValue)>),
    EnumVariant(String, Vec<(String, DebugValue)>),
    Vector(Vec<DebugValue>),
    ContainerRef(Box<DebugValue>),
    IndexedRef(Box<DebugValue>),
    Closure(String),
    Delayed,
    Error(String),
}

impl std::fmt::Display for DebugValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugValue::Invalid => write!(f, "-"),
            DebugValue::Primitive(s) | DebugValue::Address(s) | DebugValue::Error(s) => {
                write!(f, "{}", s)
            },
            DebugValue::Signer(addr) => write!(f, "signer({addr})"),
            DebugValue::Struct(fields) => {
                write!(f, "{{ ")?;
                for (i, (name, child)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if name.is_empty() {
                        write!(f, "{}", child)?;
                    } else {
                        write!(f, "{}: {}", name, child)?;
                    }
                }
                write!(f, " }}")
            },
            DebugValue::EnumVariant(name, fields) => {
                if fields.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{} {{ ", name)?;
                    for (i, (fname, child)) in fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}: {}", fname, child)?;
                    }
                    write!(f, " }}")
                }
            },
            DebugValue::Vector(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            },
            DebugValue::ContainerRef(inner) => {
                write!(f, "(&) {}", inner)
            },
            DebugValue::IndexedRef(inner) => write!(f, "{}", inner),
            DebugValue::Closure(s) => write!(f, "{}", s),
            DebugValue::Delayed => write!(f, "<?>"),
        }
    }
}

pub trait TypeNameResolver {
    fn get_struct_field_names(&self, ty: &Type) -> Option<Vec<String>>;
    fn get_enum_variant_info(&self, ty: &Type) -> Option<Vec<(String, Vec<String>)>>;
    fn load_struct_type(&self, ty: &Type) -> Option<Arc<StructType>>;
}

pub fn serialize_value_for_debug(
    locals: &Locals,
    idx: usize,
    ty: &Type,
    resolver: &impl TypeNameResolver,
) -> DebugValue {
    let locals_ref = locals.0.borrow();
    let val = &locals_ref[idx];
    serialize_value(val, Some(ty), resolver)
}

pub fn serialize_value(
    val: &Value,
    ty: Option<&Type>,
    resolver: &impl TypeNameResolver,
) -> DebugValue {
    match (val, ty) {
        (Value::Invalid, _) => DebugValue::Invalid,
        (
            Value::ContainerRef(r),
            Some(Type::Reference(inner_ty) | Type::MutableReference(inner_ty)),
        ) => DebugValue::ContainerRef(Box::new(serialize_container(
            r.container(),
            Some(inner_ty.as_ref()),
            resolver,
        ))),
        (Value::ContainerRef(r), _) => {
            DebugValue::ContainerRef(Box::new(serialize_container(r.container(), None, resolver)))
        },
        (Value::IndexedRef(r), _) => DebugValue::IndexedRef(Box::new(serialize_indexed_ref(r))),
        (Value::Container(Container::Struct(r)), Some(Type::Signer)) => {
            extract_signer_address(&r.borrow())
        },
        (Value::Container(Container::Struct(r)), Some(ty)) => {
            let struct_type = resolver.load_struct_type(ty);
            if let Some(variants) = resolver.get_enum_variant_info(ty) {
                if !variants.is_empty() {
                    return serialize_enum_fields(
                        &r.borrow(),
                        &variants,
                        struct_type.as_deref(),
                        resolver,
                    );
                }
            }
            let fields = r.borrow();
            let field_types = struct_type.and_then(|st| st.fields(None).ok().map(|f| f.to_vec()));
            let names = resolver.get_struct_field_names(ty);
            let children = fields
                .iter()
                .enumerate()
                .map(|(i, fv)| {
                    let fname = names
                        .as_ref()
                        .and_then(|n| n.get(i).cloned())
                        .unwrap_or_else(|| format!("[{}]", i));
                    let field_ty = field_types
                        .as_ref()
                        .and_then(|ft| ft.get(i).map(|(_, t)| t));
                    (fname, serialize_value(fv, field_ty, resolver))
                })
                .collect();
            DebugValue::Struct(children)
        },
        (Value::Container(c), _) => serialize_container_untyped(c),
        (Value::ClosureValue(c), _) => DebugValue::Closure(c.to_string()),
        (Value::DelayedFieldID { .. }, _) => DebugValue::Delayed,
        (Value::U8(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::U16(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::U32(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::U64(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::U128(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::U256(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::I8(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::I16(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::I32(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::I64(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::I128(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::I256(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::Bool(x), _) => DebugValue::Primitive(x.to_string()),
        (Value::Address(x), _) => DebugValue::Address(x.to_hex()),
    }
}

fn serialize_value_untyped(val: &Value) -> DebugValue {
    match val {
        Value::Invalid => DebugValue::Invalid,
        Value::U8(x) => DebugValue::Primitive(x.to_string()),
        Value::U16(x) => DebugValue::Primitive(x.to_string()),
        Value::U32(x) => DebugValue::Primitive(x.to_string()),
        Value::U64(x) => DebugValue::Primitive(x.to_string()),
        Value::U128(x) => DebugValue::Primitive(x.to_string()),
        Value::U256(x) => DebugValue::Primitive(x.to_string()),
        Value::I8(x) => DebugValue::Primitive(x.to_string()),
        Value::I16(x) => DebugValue::Primitive(x.to_string()),
        Value::I32(x) => DebugValue::Primitive(x.to_string()),
        Value::I64(x) => DebugValue::Primitive(x.to_string()),
        Value::I128(x) => DebugValue::Primitive(x.to_string()),
        Value::I256(x) => DebugValue::Primitive(x.to_string()),
        Value::Bool(x) => DebugValue::Primitive(x.to_string()),
        Value::Address(x) => DebugValue::Address(x.to_hex()),
        Value::Container(c) => serialize_container_untyped(c),
        Value::ContainerRef(r) => {
            DebugValue::ContainerRef(Box::new(serialize_container_untyped(r.container())))
        },
        Value::IndexedRef(r) => DebugValue::IndexedRef(Box::new(serialize_indexed_ref(r))),
        Value::ClosureValue(c) => DebugValue::Closure(c.to_string()),
        Value::DelayedFieldID { .. } => DebugValue::Delayed,
    }
}

fn serialize_enum_fields(
    fields: &[Value],
    variants: &[(String, Vec<String>)],
    struct_type: Option<&StructType>,
    resolver: &impl TypeNameResolver,
) -> DebugValue {
    let tag = match fields.first() {
        Some(Value::U16(t)) => *t as usize,
        _ => return DebugValue::Error("enum(<bad tag>)".into()),
    };
    match variants.get(tag) {
        Some((variant_name, field_names)) => {
            let variant_field_types =
                struct_type.and_then(|st| st.fields(Some(tag as u16)).ok().map(|f| f.to_vec()));
            let children = fields[1..]
                .iter()
                .enumerate()
                .map(|(i, fv)| {
                    let fname = field_names
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("[{}]", i));
                    let field_ty = variant_field_types
                        .as_ref()
                        .and_then(|ft| ft.get(i).map(|(_, t)| t));
                    (fname, serialize_value(fv, field_ty, resolver))
                })
                .collect();
            DebugValue::EnumVariant(variant_name.clone(), children)
        },
        None => DebugValue::Error("enum(<unknown tag>)".into()),
    }
}

fn serialize_container(
    c: &Container,
    ty: Option<&Type>,
    resolver: &impl TypeNameResolver,
) -> DebugValue {
    match (c, ty) {
        (Container::Struct(r), Some(Type::Signer)) => extract_signer_address(&r.borrow()),
        (Container::Struct(r), Some(ty)) => {
            let struct_type = resolver.load_struct_type(ty);
            if let Some(variants) = resolver.get_enum_variant_info(ty) {
                if !variants.is_empty() {
                    return serialize_enum_fields(
                        &r.borrow(),
                        &variants,
                        struct_type.as_deref(),
                        resolver,
                    );
                }
            }
            let fields = r.borrow();
            let field_types = struct_type.and_then(|st| st.fields(None).ok().map(|f| f.to_vec()));
            let names = resolver.get_struct_field_names(ty);
            let children = fields
                .iter()
                .enumerate()
                .map(|(i, fv)| {
                    let fname = names
                        .as_ref()
                        .and_then(|n| n.get(i).cloned())
                        .unwrap_or_else(|| format!("[{}]", i));
                    let field_ty = field_types
                        .as_ref()
                        .and_then(|ft| ft.get(i).map(|(_, t)| t));
                    (fname, serialize_value(fv, field_ty, resolver))
                })
                .collect();
            DebugValue::Struct(children)
        },
        _ => serialize_container_untyped(c),
    }
}

fn serialize_container_untyped(c: &Container) -> DebugValue {
    match c {
        Container::Vec(r) => {
            DebugValue::Vector(r.borrow().iter().map(serialize_value_untyped).collect())
        },
        Container::VecU8(r) => typed_vec(&r.borrow()),
        Container::VecU16(r) => typed_vec(&r.borrow()),
        Container::VecU32(r) => typed_vec(&r.borrow()),
        Container::VecU64(r) => typed_vec(&r.borrow()),
        Container::VecU128(r) => typed_vec(&r.borrow()),
        Container::VecU256(r) => typed_vec(&r.borrow()),
        Container::VecI8(r) => typed_vec(&r.borrow()),
        Container::VecI16(r) => typed_vec(&r.borrow()),
        Container::VecI32(r) => typed_vec(&r.borrow()),
        Container::VecI64(r) => typed_vec(&r.borrow()),
        Container::VecI128(r) => typed_vec(&r.borrow()),
        Container::VecI256(r) => typed_vec(&r.borrow()),
        Container::VecBool(r) => typed_vec(&r.borrow()),
        Container::VecAddress(r) => typed_vec_address(&r.borrow()),
        Container::Struct(r) => {
            let children = r
                .borrow()
                .iter()
                .map(|fv| (String::new(), serialize_value_untyped(fv)))
                .collect();
            DebugValue::Struct(children)
        },
        Container::Locals(_) => DebugValue::Error("...".into()),
    }
}

fn extract_signer_address(fields: &[Value]) -> DebugValue {
    match fields.get(MASTER_ADDRESS_FIELD_OFFSET) {
        Some(Value::Address(addr)) => DebugValue::Signer(addr.to_hex_literal()),
        _ => DebugValue::Error("signer(<unknown>)".into()),
    }
}

fn serialize_indexed_ref(r: &IndexedRef) -> DebugValue {
    let idx = r.idx as usize;
    match r.container_ref.container() {
        Container::Locals(r) | Container::Vec(r) | Container::Struct(r) => {
            serialize_slice_elem(&r.borrow(), idx, serialize_value_untyped)
        },
        Container::VecU8(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecU16(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecU32(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecU64(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecU128(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecU256(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecI8(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecI16(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecI32(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecI64(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecI128(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecI256(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecBool(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Primitive(x.to_string()))
        },
        Container::VecAddress(r) => {
            serialize_slice_elem(&r.borrow(), idx, |x| DebugValue::Address(x.to_hex()))
        },
    }
}

fn serialize_slice_elem<X, F>(v: &[X], idx: usize, f: F) -> DebugValue
where
    F: FnOnce(&X) -> DebugValue,
{
    match v.get(idx) {
        Some(x) => f(x),
        None => DebugValue::Error("slice(<out of bounds>)".into()),
    }
}

fn typed_vec<T: std::fmt::Display>(items: &[T]) -> DebugValue {
    DebugValue::Vector(
        items
            .iter()
            .map(|x| DebugValue::Primitive(x.to_string()))
            .collect(),
    )
}

fn typed_vec_address(items: &[move_core_types::account_address::AccountAddress]) -> DebugValue {
    DebugValue::Vector(
        items
            .iter()
            .map(|x| DebugValue::Address(x.to_hex()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        loaded_data::{runtime_types::AbilityInfo, struct_name_indexing::StructNameIndex},
        values::{Locals, Struct},
    };
    use move_core_types::{
        ability::AbilitySet,
        account_address::AccountAddress,
        int256::{I256, U256},
    };
    use std::collections::BTreeMap;
    use triomphe::Arc as TriompheArc;

    struct MockTypeResolver {
        struct_fields: BTreeMap<Type, Vec<String>>,
        enum_variants: BTreeMap<Type, Vec<(String, Vec<String>)>>,
    }

    impl MockTypeResolver {
        fn new(
            struct_fields: Vec<(Type, Vec<String>)>,
            enum_variants: Vec<(Type, Vec<(String, Vec<String>)>)>,
        ) -> Self {
            Self {
                struct_fields: struct_fields.into_iter().collect(),
                enum_variants: enum_variants.into_iter().collect(),
            }
        }
    }

    impl TypeNameResolver for MockTypeResolver {
        fn get_struct_field_names(&self, ty: &Type) -> Option<Vec<String>> {
            self.struct_fields.get(ty).cloned()
        }

        fn get_enum_variant_info(&self, ty: &Type) -> Option<Vec<(String, Vec<String>)>> {
            self.enum_variants.get(ty).cloned()
        }

        fn load_struct_type(&self, _ty: &Type) -> Option<Arc<StructType>> {
            None
        }
    }

    fn dummy_struct_ty(idx: u32) -> Type {
        Type::Struct {
            idx: StructNameIndex::new(idx),
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
        }
    }

    fn dummy_enum_ty() -> Type {
        dummy_struct_ty(0)
    }

    fn empty_resolver() -> MockTypeResolver {
        MockTypeResolver::new(vec![], vec![])
    }

    fn sv(val: &Value, ty: &Type, resolver: &MockTypeResolver) -> DebugValue {
        serialize_value(val, Some(ty), resolver)
    }

    #[test]
    fn test_primitives() {
        let r = empty_resolver();
        assert_eq!(sv(&Value::u8(0), &Type::U8, &r).to_string(), "0");
        assert_eq!(sv(&Value::u8(255), &Type::U8, &r).to_string(), "255");
        assert_eq!(sv(&Value::u16(1000), &Type::U16, &r).to_string(), "1000");
        assert_eq!(sv(&Value::u32(70000), &Type::U32, &r).to_string(), "70000");
        assert_eq!(
            sv(&Value::u64(1_000_000), &Type::U64, &r).to_string(),
            "1000000"
        );
        assert_eq!(
            sv(&Value::u128(99999), &Type::U128, &r).to_string(),
            "99999"
        );
        assert_eq!(
            sv(&Value::u256(U256::from(42u64)), &Type::U256, &r).to_string(),
            "42"
        );
        assert_eq!(sv(&Value::i8(-1), &Type::I8, &r).to_string(), "-1");
        assert_eq!(sv(&Value::i16(-100), &Type::I16, &r).to_string(), "-100");
        assert_eq!(
            sv(&Value::i32(-70000), &Type::I32, &r).to_string(),
            "-70000"
        );
        assert_eq!(
            sv(&Value::i64(-1_000_000), &Type::I64, &r).to_string(),
            "-1000000"
        );
        assert_eq!(
            sv(&Value::i128(-99999), &Type::I128, &r).to_string(),
            "-99999"
        );
        assert_eq!(
            sv(&Value::i256(I256::from(-42i64)), &Type::I256, &r).to_string(),
            "-42"
        );
        assert_eq!(sv(&Value::bool(true), &Type::Bool, &r).to_string(), "true");
        assert_eq!(
            sv(&Value::bool(false), &Type::Bool, &r).to_string(),
            "false"
        );
    }

    #[test]
    fn test_address() {
        let addr = AccountAddress::from_hex_literal("0xCAFE").unwrap();
        let dv = sv(&Value::address(addr), &Type::Address, &empty_resolver());
        assert!(matches!(dv, DebugValue::Address(_)));
        assert_eq!(
            dv.to_string(),
            "000000000000000000000000000000000000000000000000000000000000cafe"
        );
    }

    #[test]
    fn test_struct() {
        let ty = dummy_struct_ty(1);
        let resolver =
            MockTypeResolver::new(vec![(ty.clone(), vec!["x".into(), "y".into()])], vec![]);

        // with field names
        let val = Value::struct_(Struct::pack(vec![Value::u64(10), Value::bool(true)]));
        assert_eq!(sv(&val, &ty, &resolver).to_string(), "{ x: 10, y: true }");

        // no field names → positional fallback
        let val = Value::struct_(Struct::pack(vec![Value::u64(1), Value::u64(2)]));
        assert_eq!(
            sv(&val, &ty, &empty_resolver()).to_string(),
            "{ [0]: 1, [1]: 2 }"
        );

        // empty struct
        let val = Value::struct_(Struct::pack(vec![]));
        assert_eq!(sv(&val, &ty, &empty_resolver()).to_string(), "{  }");

        // nested struct (inner has no type info → untyped fallback, no field names)
        let inner = Struct::pack(vec![Value::u64(99)]);
        let outer = Struct::pack(vec![Value::struct_(inner), Value::bool(false)]);
        assert_eq!(
            sv(&Value::struct_(outer), &ty, &resolver).to_string(),
            "{ x: { 99 }, y: false }"
        );
    }

    #[test]
    fn test_vectors() {
        let r = empty_resolver();
        assert_eq!(
            sv(
                &Value::vector_u64(vec![]),
                &Type::Vector(TriompheArc::new(Type::U64)),
                &r
            )
            .to_string(),
            "[]"
        );
        assert_eq!(
            sv(
                &Value::vector_u8(vec![0, 255]),
                &Type::Vector(TriompheArc::new(Type::U8)),
                &r
            )
            .to_string(),
            "[0, 255]"
        );
        assert_eq!(
            sv(
                &Value::vector_u64(vec![1, 2, 3]),
                &Type::Vector(TriompheArc::new(Type::U64)),
                &r
            )
            .to_string(),
            "[1, 2, 3]"
        );
    }

    #[test]
    fn test_enum() {
        let ty = dummy_enum_ty();
        let resolver = MockTypeResolver::new(vec![], vec![(ty.clone(), vec![
            ("None".into(), vec![]),
            ("Some".into(), vec!["value".into()]),
        ])]);

        // no fields
        let val = Value::struct_(Struct::pack_variant(0, vec![]));
        let dv = sv(&val, &ty, &resolver);
        assert!(matches!(dv, DebugValue::EnumVariant(_, ref fields) if fields.is_empty()));
        assert_eq!(dv.to_string(), "None");

        // with fields
        let val = Value::struct_(Struct::pack_variant(1, vec![Value::u64(42)]));
        let dv = sv(&val, &ty, &resolver);
        assert!(matches!(dv, DebugValue::EnumVariant(_, ref fields) if fields.len() == 1));
        assert_eq!(dv.to_string(), "Some { value: 42 }");

        // bad tag type
        let val = Value::struct_(Struct::pack(vec![Value::u64(0), Value::bool(true)]));
        assert!(matches!(sv(&val, &ty, &resolver), DebugValue::Error(_)));

        // tag out of range
        let val = Value::struct_(Struct::pack_variant(5, vec![Value::u64(42)]));
        assert!(matches!(sv(&val, &ty, &resolver), DebugValue::Error(_)));

        // address field not confused with signer
        let addr = AccountAddress::from_hex_literal("0x42").unwrap();
        let val = Value::struct_(Struct::pack_variant(1, vec![Value::address(addr)]));
        let dv = sv(&val, &ty, &resolver);
        assert!(matches!(dv, DebugValue::EnumVariant(_, _)));

        // variant with a struct-typed field (no type info for child → untyped fallback)
        let inner = Struct::pack(vec![Value::u64(10), Value::bool(true)]);
        let val = Value::struct_(Struct::pack_variant(1, vec![Value::struct_(inner)]));
        let dv = sv(&val, &ty, &resolver);
        assert_eq!(dv.to_string(), "Some { value: { 10, true } }");
    }

    #[test]
    fn test_invalid_value() {
        assert!(matches!(
            sv(&Value::Invalid, &Type::U64, &empty_resolver()),
            DebugValue::Invalid
        ));
    }

    #[test]
    fn test_signer() {
        let r = empty_resolver();

        // signer value
        let addr = AccountAddress::from_hex_literal("0xCAFE").unwrap();
        let val = Value::master_signer(addr);
        let dv = sv(&val, &Type::Signer, &r);
        assert!(matches!(dv, DebugValue::Signer(_)));
        assert_eq!(dv.to_string(), "signer(0xcafe)");

        // signer reference
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let val = Value::master_signer_reference(addr);
        let ref_ty = Type::Reference(Box::new(Type::Signer));
        let dv = sv(&val, &ref_ty, &r);
        assert!(
            matches!(dv, DebugValue::ContainerRef(ref inner) if matches!(**inner, DebugValue::Signer(_)))
        );
        assert_eq!(dv.to_string(), "(&) signer(0x1)");
    }

    #[test]
    fn test_ref() {
        let enum_ty = dummy_enum_ty();
        let struct_ty = dummy_struct_ty(1);
        let resolver = MockTypeResolver::new(
            vec![(struct_ty.clone(), vec!["x".into(), "y".into()])],
            vec![(enum_ty.clone(), vec![
                ("First".into(), vec![]),
                ("Second".into(), vec!["_0".into()]),
            ])],
        );

        // enum ref
        let val = Value::struct_(Struct::pack_variant(1, vec![Value::u64(42)]));
        let locals = Locals::new_from(vec![val], 1).unwrap();
        let borrowed = locals.borrow_loc(0).unwrap();
        let ref_ty = Type::Reference(Box::new(enum_ty));
        assert_eq!(
            sv(&borrowed, &ref_ty, &resolver).to_string(),
            "(&) Second { _0: 42 }"
        );

        // struct ref
        let val = Value::struct_(Struct::pack(vec![Value::u64(10), Value::bool(true)]));
        let locals = Locals::new_from(vec![val], 1).unwrap();
        let borrowed = locals.borrow_loc(0).unwrap();
        let ref_ty = Type::Reference(Box::new(struct_ty));
        assert_eq!(
            sv(&borrowed, &ref_ty, &resolver).to_string(),
            "(&) { x: 10, y: true }"
        );
    }

    #[test]
    fn test_indexed_ref() {
        let locals = Locals::new_from(vec![Value::u64(99)], 1).unwrap();
        let indexed_ref = locals.borrow_loc(0).unwrap();
        let dv = sv(&indexed_ref, &Type::U64, &empty_resolver());
        assert!(matches!(dv, DebugValue::IndexedRef(_)));
        assert_eq!(dv.to_string(), "99");
    }

    #[test]
    fn test_closure() {
        use crate::values::function_values_impl::mock::MockAbstractFunction;
        use move_core_types::function::ClosureMask;

        let fun = MockAbstractFunction::new("test_fn", vec![], ClosureMask::new(0b01), vec![]);
        let val = Value::closure(Box::new(fun), vec![Value::u64(42)]);
        let dv = sv(&val, &Type::U64, &empty_resolver());
        assert!(matches!(dv, DebugValue::Closure(_)));
    }

    #[test]
    fn test_delayed() {
        use crate::delayed_values::delayed_field_id::DelayedFieldID;
        let val = Value::delayed_value(DelayedFieldID::new_for_test_for_u64(1));
        let dv = sv(&val, &Type::U64, &empty_resolver());
        assert!(matches!(dv, DebugValue::Delayed));
        assert_eq!(dv.to_string(), "<?>");
    }
}
