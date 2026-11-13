// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the value depth limit of layout-guided deserialization.

use crate::{
    account_address::AccountAddress,
    function::{ClosureMask, MoveClosure},
    ident_str,
    language_storage::{ModuleId, StructTag},
    value::{
        MoveFieldLayout, MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue,
        MoveVariantLayout, MAX_VALUE_NEST_DEPTH,
    },
};

/// Fixed stack size for decoding, independent of `RUST_MIN_STACK`.
const STACK_SIZE: usize = 2 << 20;

fn with_fixed_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(f)
        .expect("thread must spawn")
        .join()
        .expect("thread must not panic")
}

fn closure(captured: Vec<(MoveTypeLayout, MoveValue)>) -> MoveValue {
    MoveValue::closure(MoveClosure {
        module_id: ModuleId::new(AccountAddress::ONE, ident_str!("m").to_owned()),
        fun_id: ident_str!("f").to_owned(),
        ty_args: vec![],
        mask: ClosureMask::new((1u64 << captured.len()) - 1),
        captured,
    })
}

fn assert_too_deep(bytes: Vec<u8>, layout: MoveTypeLayout) {
    let err = with_fixed_stack(move || MoveValue::simple_deserialize(&bytes, &layout))
        .expect_err("decoding must fail");
    assert!(
        err.to_string().contains("maximum nesting depth"),
        "unexpected error: {}",
        err
    );
}

/// Returns the bytes of `inner` wrapped `n` times by `wrap`, where wrapping a value serializes
/// as a fixed prefix followed by the bytes of the value. Builds the bytes without recursion.
fn nested_bytes(n: usize, inner: MoveValue, wrap: impl Fn(MoveValue) -> MoveValue) -> Vec<u8> {
    let inner_bytes = inner.simple_serialize().expect("inner must serialize");
    let outer_bytes = wrap(inner)
        .simple_serialize()
        .expect("outer must serialize");
    assert!(outer_bytes.ends_with(&inner_bytes));
    let mut bytes = outer_bytes[..outer_bytes.len() - inner_bytes.len()].repeat(n);
    bytes.extend(inner_bytes);
    bytes
}

fn vector_layout(n: usize, inner: MoveTypeLayout) -> MoveTypeLayout {
    (0..n).fold(inner, |l, _| MoveTypeLayout::Vector(Box::new(l)))
}

fn vector_value(n: usize, inner: MoveValue) -> MoveValue {
    (0..n).fold(inner, |v, _| MoveValue::Vector(vec![v]))
}

#[test]
fn closure_nesting_beyond_limit() {
    let bytes = nested_bytes(2_000, closure(vec![]), |v| {
        closure(vec![(MoveTypeLayout::Function, v)])
    });
    assert_too_deep(bytes, MoveTypeLayout::Function);
}

#[test]
fn vector_captures_beyond_limit() {
    let bytes = nested_bytes(50, closure(vec![]), |v| {
        closure(vec![(
            vector_layout(100, MoveTypeLayout::Function),
            vector_value(100, v),
        )])
    });
    assert_too_deep(bytes, MoveTypeLayout::Function);
}

#[test]
fn deep_captured_layout_decodes() {
    // BCS bounds the nesting of the derived layout decoder at 500 enums. A captured layout of
    // depth 499 with an empty vector value is within every limit and must decode.
    let layout = vector_layout(498, MoveTypeLayout::U8);
    let bytes = nested_bytes(1, MoveValue::Vector(vec![]), |v| {
        closure(vec![(layout.clone(), v)])
    });
    let value = with_fixed_stack(move || {
        MoveValue::simple_deserialize(&bytes, &MoveTypeLayout::Function)
            .map(|v| matches!(v, MoveValue::Closure(_)))
    });
    assert!(value.expect("decoding must succeed"));
}

/// Wraps a layout and value of depth `d` into a container at depth `d + 1`.
type Wrap = fn(MoveTypeLayout, MoveValue) -> (MoveTypeLayout, MoveValue);

const WRAPS: [(&str, Wrap); 4] = [
    ("vector", |l, v| {
        (
            MoveTypeLayout::Vector(Box::new(l)),
            MoveValue::Vector(vec![v]),
        )
    }),
    ("struct", |l, v| {
        (
            MoveTypeLayout::new_struct(MoveStructLayout::Runtime(vec![l])),
            MoveValue::Struct(MoveStruct::Runtime(vec![v])),
        )
    }),
    ("variant", |l, v| {
        (
            MoveTypeLayout::new_struct(MoveStructLayout::RuntimeVariants(vec![vec![], vec![l]])),
            MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![v])),
        )
    }),
    ("closure", |l, v| {
        (MoveTypeLayout::Function, closure(vec![(l, v)]))
    }),
];

/// `n` nested containers built by `wrap` around a `u64`.
fn nested(n: u64, wrap: Wrap) -> (MoveTypeLayout, MoveValue) {
    (0..n).fold((MoveTypeLayout::U64, MoveValue::U64(7)), |(l, v), _| {
        wrap(l, v)
    })
}

#[test]
fn depth_limit_is_exact() {
    for (name, wrap) in WRAPS {
        let (layout, value) = nested(MAX_VALUE_NEST_DEPTH, wrap);
        let bytes = value.simple_serialize().expect("value must serialize");
        let decoded = MoveValue::simple_deserialize(&bytes, &layout)
            .unwrap_or_else(|e| panic!("{} at maximum depth must decode: {}", name, e));
        assert_eq!(decoded, value, "{} must round trip", name);

        let (layout, value) = nested(MAX_VALUE_NEST_DEPTH + 1, wrap);
        let bytes = value.simple_serialize().expect("value must serialize");
        assert_too_deep(bytes, layout);
    }
}

#[test]
fn explicit_depth_limit() {
    let (layout, value) = nested(3, WRAPS[0].1);
    let bytes = value.simple_serialize().expect("value must serialize");
    assert!(MoveValue::simple_deserialize_with_max_depth(&bytes, &layout, 3).is_ok());
    assert!(MoveValue::simple_deserialize_with_max_depth(&bytes, &layout, 2).is_err());
}

/// Nested decorated structs whose innermost field is a `u64`. Each level is a struct with one
/// field if `variants` is false, or the second variant of an enum otherwise.
fn decorated_layout(n: u64, variants: bool) -> MoveStructLayout {
    let tag = StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("m").to_owned(),
        name: ident_str!("S").to_owned(),
        type_args: vec![],
    };
    let mut field_layout = MoveTypeLayout::U64;
    let mut outer = None;
    for _ in 0..n {
        let field = MoveFieldLayout::new(ident_str!("f").to_owned(), field_layout);
        let struct_layout = if variants {
            MoveStructLayout::WithVariants {
                type_: tag.clone(),
                variants: vec![
                    MoveVariantLayout {
                        name: ident_str!("A").to_owned(),
                        fields: vec![],
                    },
                    MoveVariantLayout {
                        name: ident_str!("B").to_owned(),
                        fields: vec![field],
                    },
                ],
            }
        } else {
            MoveStructLayout::WithTypes {
                type_: tag.clone(),
                fields: vec![field],
            }
        };
        field_layout = MoveTypeLayout::new_struct(struct_layout.clone());
        outer = Some(struct_layout);
    }
    outer.expect("n must be positive")
}

#[test]
fn decorated_depth_limit_is_exact() {
    for (variants, wrap) in [(false, WRAPS[1].1), (true, WRAPS[2].1)] {
        for (n, ok) in [
            (MAX_VALUE_NEST_DEPTH, true),
            (MAX_VALUE_NEST_DEPTH + 1, false),
        ] {
            let (_, value) = nested(n, wrap);
            let bytes = value.simple_serialize().expect("value must serialize");
            let layout = decorated_layout(n, variants);
            let result = MoveStruct::simple_deserialize(&bytes, &layout);
            assert_eq!(result.is_ok(), ok, "variants: {}, depth: {}", variants, n);
        }
    }
}
