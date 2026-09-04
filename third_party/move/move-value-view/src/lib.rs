// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A data model of Move's value shapes.
//!
//! By implementing the [`MoveValueView`] trait, a Rust value describes itself
//! to a visitor, which can then decide what to do with it.
//!
//! This is similar to serde's model, but with the following critical differences:
//! - It is modeled after the Move VM's value model, without any unnecessary boilerplate.
//! - It allows for efficient handling of byte arrays (`vector<u8>`).

use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};

/// A Rust value with a Move counterpart.
///
/// Describes itself by calling the `visit_*` methods.
pub trait MoveValueView {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error>;

    /// Optional performance optimization.
    ///
    /// Reinterprets a slice of `Self`  as bytes, so a `vector<u8>` is written
    /// in one copy rather than an element at a time.
    #[doc(hidden)]
    fn slice_as_bytes(_slice: &[Self]) -> Option<&[u8]>
    where
        Self: Sized,
    {
        None
    }
}

/// Consumes the Move value that a [`MoveValueView`] describes.
pub trait MoveValueVisitor: Sized {
    type Ok;
    type Error;
    /// Consumes the fields of a struct or an enum variant body.
    type Fields: FieldVisitor<Ok = Self::Ok, Error = Self::Error>;
    /// Consumes the elements of a vector.
    type Elements: VectorElementVisitor<Ok = Self::Ok, Error = Self::Error>;

    fn visit_bool(self, v: bool) -> Result<Self::Ok, Self::Error>;

    fn visit_u8(self, v: u8) -> Result<Self::Ok, Self::Error>;
    fn visit_u16(self, v: u16) -> Result<Self::Ok, Self::Error>;
    fn visit_u32(self, v: u32) -> Result<Self::Ok, Self::Error>;
    fn visit_u64(self, v: u64) -> Result<Self::Ok, Self::Error>;
    fn visit_u128(self, v: u128) -> Result<Self::Ok, Self::Error>;
    fn visit_u256(self, v: U256) -> Result<Self::Ok, Self::Error>;

    fn visit_i8(self, v: i8) -> Result<Self::Ok, Self::Error>;
    fn visit_i16(self, v: i16) -> Result<Self::Ok, Self::Error>;
    fn visit_i32(self, v: i32) -> Result<Self::Ok, Self::Error>;
    fn visit_i64(self, v: i64) -> Result<Self::Ok, Self::Error>;
    fn visit_i128(self, v: i128) -> Result<Self::Ok, Self::Error>;
    fn visit_i256(self, v: I256) -> Result<Self::Ok, Self::Error>;

    fn visit_address(self, v: &AccountAddress) -> Result<Self::Ok, Self::Error>;

    /// A `signer`, which Move keeps distinct from the address it holds.
    fn visit_signer(self, v: &AccountAddress) -> Result<Self::Ok, Self::Error>;

    /// A `vector<u8>`, given whole.
    fn visit_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error>;

    /// A `vector<T>` of `len` elements, each described in turn.
    fn visit_vector(self, len: usize) -> Result<Self::Elements, Self::Error>;

    /// A struct with `num_fields` fields, each described in turn.
    fn visit_struct(self, num_fields: usize) -> Result<Self::Fields, Self::Error>;

    /// The `index`th variant of an enum, whose body holds `num_fields` fields.
    fn visit_variant(self, index: u32, num_fields: usize) -> Result<Self::Fields, Self::Error>;

    /// A function value, which no visitor writes today.
    fn function_values_unsupported() -> Self::Error;
}

/// Receives a struct's or variant body's fields in declaration order.
pub trait FieldVisitor {
    type Ok;
    type Error;

    fn field<T: MoveValueView + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>;
    fn end(self) -> Result<Self::Ok, Self::Error>;
}

/// Receives a vector's elements in order.
pub trait VectorElementVisitor {
    type Ok;
    type Error;

    fn element<T: MoveValueView + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>;
    fn end(self) -> Result<Self::Ok, Self::Error>;
}

impl MoveValueView for bool {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_bool(*self)
    }
}

impl MoveValueView for u8 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_u8(*self)
    }

    fn slice_as_bytes(slice: &[u8]) -> Option<&[u8]> {
        Some(slice)
    }
}

impl MoveValueView for u16 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_u16(*self)
    }
}

impl MoveValueView for u32 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_u32(*self)
    }
}

impl MoveValueView for u64 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_u64(*self)
    }
}

impl MoveValueView for u128 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_u128(*self)
    }
}

impl MoveValueView for U256 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_u256(*self)
    }
}

impl MoveValueView for i8 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_i8(*self)
    }
}

impl MoveValueView for i16 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_i16(*self)
    }
}

impl MoveValueView for i32 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_i32(*self)
    }
}

impl MoveValueView for i64 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_i64(*self)
    }
}

impl MoveValueView for i128 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_i128(*self)
    }
}

impl MoveValueView for I256 {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_i256(*self)
    }
}

impl MoveValueView for AccountAddress {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        visitor.visit_address(self)
    }
}

/// Move models a string as a struct holding its bytes.
impl MoveValueView for str {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        let mut fields = visitor.visit_struct(1)?;
        fields.field(self.as_bytes())?;
        fields.end()
    }
}

impl MoveValueView for String {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        self.as_str().visit(visitor)
    }
}

/// Move's `Option` is an enum of `None` and `Some`; the pre-enum
/// struct-of-vector representation is not supported.
impl<T: MoveValueView> MoveValueView for Option<T> {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        match self {
            None => visitor.visit_variant(0, 0)?.end(),
            Some(value) => {
                let mut fields = visitor.visit_variant(1, 1)?;
                fields.field(value)?;
                fields.end()
            },
        }
    }
}

impl<T: MoveValueView> MoveValueView for [T] {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        if let Some(bytes) = T::slice_as_bytes(self) {
            return visitor.visit_bytes(bytes);
        }
        let mut elements = visitor.visit_vector(self.len())?;
        for element in self {
            elements.element(element)?;
        }
        elements.end()
    }
}

/// Views an exact-size iterator's items as a Move `vector`, so a projected
/// sequence can be visited without collecting it first. `Clone` lets each
/// visit consume a fresh copy of the iterator.
pub struct IterAsMoveVector<I>(pub I);

impl<T: MoveValueView, I: ExactSizeIterator<Item = T> + Clone> MoveValueView
    for IterAsMoveVector<I>
{
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        let iter = self.0.clone();
        let mut elements = visitor.visit_vector(iter.len())?;
        for item in iter {
            elements.element(&item)?;
        }
        elements.end()
    }
}

impl<T: MoveValueView> MoveValueView for Vec<T> {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        self.as_slice().visit(visitor)
    }
}

impl<T: MoveValueView, const N: usize> MoveValueView for [T; N] {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        self.as_slice().visit(visitor)
    }
}

impl<T: MoveValueView + ?Sized> MoveValueView for &T {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        (**self).visit(visitor)
    }
}

impl<T: MoveValueView + ?Sized> MoveValueView for Box<T> {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        (**self).visit(visitor)
    }
}

/// The dynamic Move value used by legacy VM, some other tools and tests
impl MoveValueView for move_core_types::value::MoveValue {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        use move_core_types::value::MoveValue as MV;
        match self {
            MV::Bool(v) => visitor.visit_bool(*v),
            MV::U8(v) => visitor.visit_u8(*v),
            MV::U16(v) => visitor.visit_u16(*v),
            MV::U32(v) => visitor.visit_u32(*v),
            MV::U64(v) => visitor.visit_u64(*v),
            MV::U128(v) => visitor.visit_u128(*v),
            MV::U256(v) => visitor.visit_u256(*v),
            MV::I8(v) => visitor.visit_i8(*v),
            MV::I16(v) => visitor.visit_i16(*v),
            MV::I32(v) => visitor.visit_i32(*v),
            MV::I64(v) => visitor.visit_i64(*v),
            MV::I128(v) => visitor.visit_i128(*v),
            MV::I256(v) => visitor.visit_i256(*v),
            MV::Address(v) => visitor.visit_address(v),
            MV::Signer(v) => visitor.visit_signer(v),
            MV::Vector(elements) => {
                let mut vector = visitor.visit_vector(elements.len())?;
                for element in elements {
                    vector.element(element)?;
                }
                vector.end()
            },
            MV::Struct(v) => v.visit(visitor),
            MV::Closure(_) => Err(V::function_values_unsupported()),
        }
    }
}

impl MoveValueView for move_core_types::value::MoveStruct {
    fn visit<V: MoveValueVisitor>(&self, visitor: V) -> Result<V::Ok, V::Error> {
        use move_core_types::value::MoveStruct as S;
        match self {
            S::Runtime(values) => {
                let mut fields = visitor.visit_struct(values.len())?;
                for value in values {
                    fields.field(value)?;
                }
                fields.end()
            },
            S::WithFields(named) | S::WithTypes { _fields: named, .. } => {
                let mut fields = visitor.visit_struct(named.len())?;
                for (_, value) in named {
                    fields.field(value)?;
                }
                fields.end()
            },
            S::RuntimeVariant(tag, values) => {
                let mut fields = visitor.visit_variant(u32::from(*tag), values.len())?;
                for value in values {
                    fields.field(value)?;
                }
                fields.end()
            },
            S::WithVariantFields(_, tag, named) => {
                let mut fields = visitor.visit_variant(u32::from(*tag), named.len())?;
                for (_, value) in named {
                    fields.field(value)?;
                }
                fields.end()
            },
        }
    }
}
