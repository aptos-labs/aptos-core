// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Writes a Rust value into the VM's in-memory representation, by visiting it
//! through the Move value model.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    heap::{alloc_enum_no_gc, alloc_vec_no_gc, AllocationError, Heap},
    memory::write_ptr,
    types::VEC_DATA_OFFSET,
};
use mono_move_core::{
    types::InternedType, FieldValueLayout, LayoutKind, LayoutProvider, ValueLayout,
    ENUM_DATA_OFFSET,
};
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};
use move_value_view::{FieldVisitor, MoveValueView, MoveValueVisitor, VectorElementVisitor};

/// Writes `value` at `dst` as a VM value of type `ty`. Fails rather than
/// triggering GC when the heap is full.
///
/// # Safety
///
/// `dst` must be writable for `ty`'s in-memory size, meet the type's
/// alignment, and outlive the call; `ty` must not be a reference.
pub(crate) unsafe fn write_value<L: LayoutProvider + ?Sized, T: MoveValueView + ?Sized>(
    layouts: &L,
    heap: &mut Heap,
    ty: InternedType,
    value: &T,
    dst: *mut u8,
) -> Result<(), RuntimeError> {
    let layout = layouts.layout_by_ty(ty).ok_or({
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
    })?;
    debug_assert!(
        dst.addr().is_multiple_of(layout.align as usize),
        "write destination must meet the type's alignment"
    );
    value.visit(ValueWriter {
        layouts,
        heap,
        layout,
        dst,
    })
}

/// Writes a single value of `layout` at `dst`.
struct ValueWriter<'a, L: ?Sized> {
    layouts: &'a L,
    heap: &'a mut Heap,
    layout: &'a ValueLayout,
    dst: *mut u8,
}

/// Returns a string representation of a layout's shape, useful for error messages.
fn describe_layout(layout: &ValueLayout) -> String {
    match &layout.kind {
        LayoutKind::Bool => "a bool".to_string(),
        LayoutKind::UnsignedInt => format!("a {}-byte unsigned integer", layout.size),
        LayoutKind::SignedInt => format!("a {}-byte signed integer", layout.size),
        LayoutKind::Address => "an address".to_string(),
        LayoutKind::Signer => "a signer".to_string(),
        LayoutKind::Struct { fields } => format!("a struct with {} fields", fields.len()),
        LayoutKind::Vector { .. } => "a vector".to_string(),
        LayoutKind::FrozenEnum { variants, .. } => {
            format!("an enum with {} variants", variants.len())
        },
        LayoutKind::Ref => "a reference".to_string(),
        LayoutKind::Function => "a function value".to_string(),
    }
}

impl<'a, L: LayoutProvider + ?Sized> ValueWriter<'a, L> {
    /// The value's shape does not match this writer's target layout.
    fn mismatch(&self, value: impl Into<String>) -> RuntimeError {
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::CallArgMismatch(format!(
            "cannot write {} into {}",
            value.into(),
            describe_layout(self.layout)
        )))
    }

    /// Writes an integer's little-endian bytes into a matching integer slot.
    //
    // TODO(correctness): assumes a little-endian host, as the BCS path does.
    fn write_int(self, signed: bool, bytes: &[u8]) -> Result<(), RuntimeError> {
        let kind_matches = match self.layout.kind {
            LayoutKind::UnsignedInt => !signed,
            LayoutKind::SignedInt => signed,
            _ => false,
        };
        if !kind_matches || self.layout.size as usize != bytes.len() {
            return Err(self.mismatch(format!(
                "a {}-byte {} integer",
                bytes.len(),
                if signed { "signed" } else { "unsigned" }
            )));
        }
        // SAFETY: `dst` is writable for the layout's size, checked equal to
        // the byte count.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.dst, bytes.len()) };
        Ok(())
    }

    /// Resolves the field list a struct or variant body lays out, checking that
    /// it holds exactly the fields the value declares.
    fn fields_of(
        layout: &'a ValueLayout,
        value: &'static str,
        num_fields: usize,
    ) -> Result<&'a [FieldValueLayout], RuntimeError> {
        match &layout.kind {
            LayoutKind::Struct { fields } if fields.len() == num_fields => Ok(fields),
            _ => Err(RuntimeError::InvariantViolation(
                RuntimeInvariantViolation::CallArgMismatch(format!(
                    "cannot write a {num_fields}-field {value} into {}",
                    describe_layout(layout)
                )),
            )),
        }
    }
}

impl<'a, L: LayoutProvider + ?Sized> MoveValueVisitor for ValueWriter<'a, L> {
    type Elements = SeqWriter<'a, L>;
    type Error = RuntimeError;
    type Fields = FieldsWriter<'a, L>;
    type Ok = ();

    fn function_values_unsupported() -> RuntimeError {
        RuntimeError::Unsupported("function values are not yet supported")
    }

    //======================================================================
    // Scalars
    //======================================================================

    fn visit_bool(self, v: bool) -> Result<(), RuntimeError> {
        if !matches!(self.layout.kind, LayoutKind::Bool) {
            return Err(self.mismatch("a bool"));
        }
        // SAFETY: `dst` is writable for the 1-byte bool slot.
        unsafe { std::ptr::write(self.dst, v as u8) };
        Ok(())
    }

    fn visit_u8(self, v: u8) -> Result<(), RuntimeError> {
        self.write_int(false, &v.to_le_bytes())
    }

    fn visit_u16(self, v: u16) -> Result<(), RuntimeError> {
        self.write_int(false, &v.to_le_bytes())
    }

    fn visit_u32(self, v: u32) -> Result<(), RuntimeError> {
        self.write_int(false, &v.to_le_bytes())
    }

    fn visit_u64(self, v: u64) -> Result<(), RuntimeError> {
        self.write_int(false, &v.to_le_bytes())
    }

    fn visit_u128(self, v: u128) -> Result<(), RuntimeError> {
        self.write_int(false, &v.to_le_bytes())
    }

    fn visit_u256(self, v: U256) -> Result<(), RuntimeError> {
        self.write_int(false, &v.to_le_bytes())
    }

    fn visit_i8(self, v: i8) -> Result<(), RuntimeError> {
        self.write_int(true, &v.to_le_bytes())
    }

    fn visit_i16(self, v: i16) -> Result<(), RuntimeError> {
        self.write_int(true, &v.to_le_bytes())
    }

    fn visit_i32(self, v: i32) -> Result<(), RuntimeError> {
        self.write_int(true, &v.to_le_bytes())
    }

    fn visit_i64(self, v: i64) -> Result<(), RuntimeError> {
        self.write_int(true, &v.to_le_bytes())
    }

    fn visit_i128(self, v: i128) -> Result<(), RuntimeError> {
        self.write_int(true, &v.to_le_bytes())
    }

    fn visit_i256(self, v: I256) -> Result<(), RuntimeError> {
        self.write_int(true, &v.to_le_bytes())
    }

    fn visit_address(self, v: &AccountAddress) -> Result<(), RuntimeError> {
        if !matches!(self.layout.kind, LayoutKind::Address)
            || self.layout.size as usize != AccountAddress::LENGTH
        {
            return Err(self.mismatch("an address"));
        }
        // SAFETY: `dst` is writable for the layout's size, checked to be an
        // address's length.
        unsafe {
            std::ptr::copy_nonoverlapping(v.as_ref().as_ptr(), self.dst, AccountAddress::LENGTH)
        };
        Ok(())
    }

    /// A signer is authority the VM grants, never a value a caller supplies:
    /// `CallBuilder::signer` places one from a buffer that outlives the call.
    fn visit_signer(self, _: &AccountAddress) -> Result<(), RuntimeError> {
        Err(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::CallArgMismatch(
                "a signer cannot be supplied as a value".to_string(),
            ),
        ))
    }

    //======================================================================
    // Vectors
    //======================================================================

    fn visit_bytes(self, v: &[u8]) -> Result<(), RuntimeError> {
        let LayoutKind::Vector {
            elem_id,
            descriptor_id,
        } = &self.layout.kind
        else {
            return Err(self.mismatch("a byte vector"));
        };
        let elem = self.layouts.layout(*elem_id).ok_or({
            RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
        })?;
        // TODO(completeness): `vector<bool>` and `vector<i8>` are rejected
        // here, though the BCS path accepts the same bytes.
        if !matches!(elem.kind, LayoutKind::UnsignedInt) || elem.size != 1 {
            return Err(RuntimeError::InvariantViolation(
                RuntimeInvariantViolation::CallArgMismatch(
                    "a byte vector against a vector of non-byte elements".to_string(),
                ),
            ));
        }
        if v.is_empty() {
            // The empty vector is the null pointer.
            // SAFETY: `dst` is writable for the vector's pointer slot.
            unsafe { write_ptr(self.dst, 0usize, std::ptr::null()) };
            return Ok(());
        }
        if v.len() as u64 > bcs::MAX_SEQUENCE_LENGTH as u64 {
            return Err(RuntimeError::BCSSequenceTooLong {
                len: v.len() as u64,
            });
        }
        let vec_ptr = alloc_vec_no_gc(self.heap, *descriptor_id, 1, v.len() as u64)
            .map_err(AllocationError::into_runtime_error)?;
        // SAFETY: the data region was allocated for exactly `v.len()` bytes.
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), vec_ptr.add(VEC_DATA_OFFSET), v.len()) };
        // SAFETY: `dst` is writable for the vector's pointer slot.
        unsafe { write_ptr(self.dst, 0usize, vec_ptr) };
        Ok(())
    }

    fn visit_vector(self, len: usize) -> Result<SeqWriter<'a, L>, RuntimeError> {
        let LayoutKind::Vector {
            elem_id,
            descriptor_id,
        } = &self.layout.kind
        else {
            return Err(self.mismatch("a vector"));
        };
        let elem_layout = self.layouts.layout(*elem_id).ok_or({
            RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
        })?;
        if len as u64 > bcs::MAX_SEQUENCE_LENGTH as u64 {
            return Err(RuntimeError::BCSSequenceTooLong { len: len as u64 });
        }
        let vec_ptr = if len == 0 {
            // The empty vector is the null pointer, written at `end()`.
            std::ptr::null_mut()
        } else {
            alloc_vec_no_gc(self.heap, *descriptor_id, elem_layout.size, len as u64)
                .map_err(AllocationError::into_runtime_error)?
        };
        Ok(SeqWriter {
            layouts: self.layouts,
            heap: self.heap,
            elem_layout,
            vec_ptr,
            dst: self.dst,
            expected: len,
            written: 0,
        })
    }

    //======================================================================
    // Structs and enums
    //======================================================================

    fn visit_struct(self, num_fields: usize) -> Result<FieldsWriter<'a, L>, RuntimeError> {
        let fields = Self::fields_of(self.layout, "struct", num_fields)?;
        Ok(FieldsWriter {
            layouts: self.layouts,
            heap: self.heap,
            fields,
            base: self.dst,
            next: 0,
            finish: None,
        })
    }

    fn visit_variant(
        self,
        index: u32,
        num_fields: usize,
    ) -> Result<FieldsWriter<'a, L>, RuntimeError> {
        let LayoutKind::FrozenEnum {
            descriptor_id,
            variants,
            max_size_across_variants,
        } = &self.layout.kind
        else {
            return Err(self.mismatch(format!("a {num_fields}-field enum variant")));
        };
        let variant_id = *variants.get(index as usize).ok_or({
            RuntimeError::InvariantViolation(RuntimeInvariantViolation::EnumTagOutOfRange {
                tag: index as u64,
                variant_count: variants.len(),
            })
        })?;
        let variant_layout = self.layouts.layout(variant_id).ok_or({
            RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
        })?;
        let fields = Self::fields_of(variant_layout, "variant body", num_fields)?;

        let obj_ptr = alloc_enum_no_gc(
            self.heap,
            *descriptor_id,
            index as u64,
            *max_size_across_variants as usize,
        )
        .map_err(AllocationError::into_runtime_error)?;
        // SAFETY: the variant body lives at this offset within the allocation.
        let data_ptr = unsafe { obj_ptr.add(ENUM_DATA_OFFSET) };
        Ok(FieldsWriter {
            layouts: self.layouts,
            heap: self.heap,
            fields,
            base: data_ptr,
            next: 0,
            finish: Some((obj_ptr, self.dst)),
        })
    }
}

/// Writes a vector's elements into a preallocated vector object.
struct SeqWriter<'a, L: ?Sized> {
    layouts: &'a L,
    heap: &'a mut Heap,
    elem_layout: &'a ValueLayout,
    /// Null for the empty vector.
    vec_ptr: *mut u8,
    /// The slot the vector pointer lands in.
    dst: *mut u8,
    expected: usize,
    written: usize,
}

impl<L: LayoutProvider + ?Sized> VectorElementVisitor for SeqWriter<'_, L> {
    type Error = RuntimeError;
    type Ok = ();

    fn element<T: MoveValueView + ?Sized>(&mut self, value: &T) -> Result<(), RuntimeError> {
        if self.written >= self.expected {
            return Err(RuntimeError::InvariantViolation(
                RuntimeInvariantViolation::CallArgMismatch(format!(
                    "a vector wrote more than its {} declared elements",
                    self.expected
                )),
            ));
        }
        // SAFETY: the data region was allocated for `expected` elements and
        // `written < expected`.
        let elem_dst = unsafe {
            self.vec_ptr
                .add(VEC_DATA_OFFSET + self.written * self.elem_layout.size as usize)
        };
        value.visit(ValueWriter {
            layouts: self.layouts,
            heap: self.heap,
            layout: self.elem_layout,
            dst: elem_dst,
        })?;
        self.written += 1;
        Ok(())
    }

    fn end(self) -> Result<(), RuntimeError> {
        if self.written != self.expected {
            return Err(RuntimeError::InvariantViolation(
                RuntimeInvariantViolation::CallArgMismatch(format!(
                    "a vector wrote {} of its {} declared elements",
                    self.written, self.expected
                )),
            ));
        }
        // SAFETY: `dst` is writable for the vector's pointer slot.
        unsafe { write_ptr(self.dst, 0usize, self.vec_ptr) };
        Ok(())
    }
}

/// Writes a struct's (or enum variant body's) fields in declaration order.
struct FieldsWriter<'a, L: ?Sized> {
    layouts: &'a L,
    heap: &'a mut Heap,
    fields: &'a [FieldValueLayout],
    /// Base of the field region: the value's slot, or an enum's data region.
    base: *mut u8,
    next: usize,
    /// Set for an enum variant: the object and the slot to link it into.
    finish: Option<(*mut u8, *mut u8)>,
}

impl<L: LayoutProvider + ?Sized> FieldVisitor for FieldsWriter<'_, L> {
    type Error = RuntimeError;
    type Ok = ();

    fn field<T: MoveValueView + ?Sized>(&mut self, value: &T) -> Result<(), RuntimeError> {
        let field = self.fields.get(self.next).ok_or_else(|| {
            RuntimeError::InvariantViolation(RuntimeInvariantViolation::CallArgMismatch(format!(
                "a value wrote more than its {} declared fields",
                self.fields.len()
            )))
        })?;
        let layout = self.layouts.layout(field.id).ok_or({
            RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
        })?;
        // SAFETY: a field offset lies within the field region, which was
        // sized for the whole struct.
        let dst = unsafe { self.base.add(field.offset as usize) };
        value.visit(ValueWriter {
            layouts: self.layouts,
            heap: self.heap,
            layout,
            dst,
        })?;
        self.next += 1;
        Ok(())
    }

    fn end(self) -> Result<(), RuntimeError> {
        if self.next != self.fields.len() {
            return Err(RuntimeError::InvariantViolation(
                RuntimeInvariantViolation::CallArgMismatch(format!(
                    "a value wrote {} of its {} declared fields",
                    self.next,
                    self.fields.len()
                )),
            ));
        }
        if let Some((obj_ptr, slot)) = self.finish {
            // SAFETY: `slot` is writable for the enum's pointer slot.
            unsafe { write_ptr(slot, 0usize, obj_ptr) };
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_conv::bcs::AlignedBuf;
    use mono_move_core::{
        intern_type_tag, reserved_layout_id, types::Type, DescriptorId, LayoutFlags, LayoutId,
    };
    use mono_move_global_context::{ExecutionGuard, GlobalContext};
    use move_core_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };
    use move_value_view::MoveValueView;
    use move_value_view_derive::MoveValueView;
    use serde::Serialize;

    /// A pointer-slot layout, as vectors and enums use.
    fn ptr_layout(kind: LayoutKind) -> ValueLayout {
        ValueLayout::new(8, 8, None, LayoutFlags::empty(), kind)
    }

    fn struct_layout(size: u32, align: u32, fields: Vec<FieldValueLayout>) -> ValueLayout {
        ValueLayout::new(
            size,
            align,
            None,
            LayoutFlags::empty(),
            LayoutKind::Struct {
                fields: fields.into_boxed_slice(),
            },
        )
    }

    fn field(offset: u32, id: LayoutId) -> FieldValueLayout {
        FieldValueLayout { offset, id }
    }

    fn tag(module: &str, name: &str, type_args: Vec<TypeTag>) -> TypeTag {
        TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new(module).unwrap(),
            name: Identifier::new(name).unwrap(),
            type_args,
        }))
    }

    fn reserved(ty: &Type) -> LayoutId {
        reserved_layout_id(ty).expect("reserved layout")
    }

    fn publish_vector(
        guard: &ExecutionGuard<'_>,
        elem_tag: &TypeTag,
        elem_id: LayoutId,
    ) -> InternedType {
        let ty = intern_type_tag(&TypeTag::Vector(Box::new(elem_tag.clone())), guard).unwrap();
        guard.publish_layout(
            ty,
            ptr_layout(LayoutKind::Vector {
                elem_id,
                descriptor_id: DescriptorId(0),
            }),
        );
        ty
    }

    /// Publishes an `Option<elem>` mirroring the stdlib's `None`/`Some` enum.
    fn publish_option(
        guard: &ExecutionGuard<'_>,
        elem_tag: &TypeTag,
        elem_id: LayoutId,
        elem_size: u32,
    ) -> InternedType {
        let ty = intern_type_tag(&tag("option", "Option", vec![elem_tag.clone()]), guard).unwrap();
        let variants = guard.publish_variant_layouts(ty, vec![
            struct_layout(0, 1, vec![]),
            struct_layout(elem_size, elem_size.max(1), vec![field(0, elem_id)]),
        ]);
        guard.publish_layout(
            ty,
            ptr_layout(LayoutKind::FrozenEnum {
                descriptor_id: DescriptorId(0),
                variants,
                max_size_across_variants: 8 + elem_size,
            }),
        );
        ty
    }

    /// Checks that the model write and the BCS path agree byte for byte.
    fn check<T: MoveValueView + Serialize + ?Sized>(
        guard: &ExecutionGuard<'_>,
        ty: InternedType,
        value: &T,
    ) {
        let bytes = bcs::to_bytes(value).expect("value BCS-encodes");
        let mut written_heap = Heap::new(1 << 20);
        let mut decoded_heap = Heap::new(1 << 20);
        let mut written_slot = AlignedBuf::zeroed(128);
        let mut decoded_slot = AlignedBuf::zeroed(128);
        // SAFETY: the slots are aligned and wider than any test type's
        // in-memory size.
        unsafe {
            write_value(
                guard,
                &mut written_heap,
                ty,
                value,
                written_slot.as_mut_ptr(),
            )
            .expect("write_value succeeds");
            crate::value_conv::bcs::deserialize_into(
                guard,
                &mut decoded_heap,
                ty,
                &bytes,
                decoded_slot.as_mut_ptr(),
            )
            .expect("deserialize_into succeeds");
        }
        let (written_slot, decoded_slot) = (written_slot.as_ptr(), decoded_slot.as_ptr());
        // SAFETY: both slots hold initialized values of `ty` on live heaps.
        let (written, decoded) = unsafe {
            (
                crate::value_conv::bcs::serialize(guard, written_slot, ty)
                    .expect("the written value serializes"),
                crate::value_conv::bcs::serialize(guard, decoded_slot, ty)
                    .expect("the decoded value serializes"),
            )
        };
        assert_eq!(written, bytes, "the model write diverges from the BCS path");
        assert_eq!(decoded, bytes, "the BCS write does not round-trip");
    }

    #[test]
    fn writes_every_integer_width() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        check(&guard, intern_type_tag(&TypeTag::U8, &guard).unwrap(), &7u8);
        check(
            &guard,
            intern_type_tag(&TypeTag::U16, &guard).unwrap(),
            &7u16,
        );
        check(
            &guard,
            intern_type_tag(&TypeTag::U32, &guard).unwrap(),
            &7u32,
        );
        check(
            &guard,
            intern_type_tag(&TypeTag::U64, &guard).unwrap(),
            &7u64,
        );
        check(
            &guard,
            intern_type_tag(&TypeTag::U128, &guard).unwrap(),
            &7u128,
        );
        check(
            &guard,
            intern_type_tag(&TypeTag::U256, &guard).unwrap(),
            &U256::from_le_bytes([9u8; 32]),
        );
        check(
            &guard,
            intern_type_tag(&TypeTag::Bool, &guard).unwrap(),
            &true,
        );
    }

    #[test]
    fn writes_an_address_in_one_go() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let ty = intern_type_tag(&TypeTag::Address, &guard).unwrap();
        check(&guard, ty, &AccountAddress::ONE);
        check(
            &guard,
            ty,
            &AccountAddress::from_hex_literal("0xbeef").unwrap(),
        );
    }

    #[test]
    fn writes_vectors() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let vec_u8 = publish_vector(&guard, &TypeTag::U8, reserved(&Type::U8));
        let vec_u64 = publish_vector(&guard, &TypeTag::U64, reserved(&Type::U64));
        // `Vec<u8>` takes the bulk-copy path, `Vec<u64>` the element loop.
        check(&guard, vec_u8, &Vec::<u8>::new());
        check(&guard, vec_u8, &vec![1u8, 2, 3, 4, 5]);
        check(&guard, vec_u64, &Vec::<u64>::new());
        check(&guard, vec_u64, &vec![1u64, 2, 3]);
    }

    #[test]
    fn writes_a_string_as_a_wrapped_byte_vector() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let vec_u8 = publish_vector(&guard, &TypeTag::U8, reserved(&Type::U8));
        let vec_u8_id = guard.layout_id(vec_u8).unwrap();
        let string_ty = intern_type_tag(&tag("string", "String", vec![]), &guard).unwrap();
        guard.publish_layout(string_ty, struct_layout(8, 8, vec![field(0, vec_u8_id)]));
        check(&guard, string_ty, &"hello Move".to_string());
        check(&guard, string_ty, &String::new());
    }

    #[test]
    fn writes_options_as_enums() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let opt_u64 = publish_option(&guard, &TypeTag::U64, reserved(&Type::U64), 8);
        check(&guard, opt_u64, &None::<u64>);
        check(&guard, opt_u64, &Some(42u64));
    }

    #[derive(Serialize, MoveValueView)]
    struct Metadata {
        proposer: AccountAddress,
        round: u64,
        votes: Vec<u8>,
        name: String,
    }

    #[test]
    fn derives_for_a_struct() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let vec_u8 = publish_vector(&guard, &TypeTag::U8, reserved(&Type::U8));
        let vec_u8_id = guard.layout_id(vec_u8).unwrap();
        let string_ty = intern_type_tag(&tag("string", "String", vec![]), &guard).unwrap();
        guard.publish_layout(string_ty, struct_layout(8, 8, vec![field(0, vec_u8_id)]));
        let string_id = guard.layout_id(string_ty).unwrap();

        let ty = intern_type_tag(&tag("m", "Metadata", vec![]), &guard).unwrap();
        guard.publish_layout(
            ty,
            struct_layout(56, 8, vec![
                field(0, reserved(&Type::Address)),
                field(32, reserved(&Type::U64)),
                field(40, vec_u8_id),
                field(48, string_id),
            ]),
        );
        check(&guard, ty, &Metadata {
            proposer: AccountAddress::ONE,
            round: 9,
            votes: vec![1, 2, 3],
            name: "block".to_string(),
        });
    }

    #[derive(Serialize, MoveValueView)]
    struct Wrapper<T> {
        inner: T,
    }

    #[test]
    fn derives_for_a_generic_struct() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let ty = intern_type_tag(&tag("m", "Wrapper", vec![]), &guard).unwrap();
        guard.publish_layout(
            ty,
            struct_layout(8, 8, vec![field(0, reserved(&Type::U64))]),
        );
        check(&guard, ty, &Wrapper { inner: 7u64 });
    }

    #[derive(Serialize, MoveValueView)]
    enum Protector {
        Nonce(u64),
        Pair { lo: u64, hi: u64 },
        Nothing,
    }

    #[test]
    fn derives_for_an_enum() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let ty = intern_type_tag(&tag("m", "Protector", vec![]), &guard).unwrap();
        let variants = guard.publish_variant_layouts(ty, vec![
            struct_layout(8, 8, vec![field(0, reserved(&Type::U64))]),
            struct_layout(16, 8, vec![
                field(0, reserved(&Type::U64)),
                field(8, reserved(&Type::U64)),
            ]),
            struct_layout(0, 1, vec![]),
        ]);
        guard.publish_layout(
            ty,
            ptr_layout(LayoutKind::FrozenEnum {
                descriptor_id: DescriptorId(0),
                variants,
                max_size_across_variants: 24,
            }),
        );
        check(&guard, ty, &Protector::Nonce(7));
        check(&guard, ty, &Protector::Pair { lo: 1, hi: 2 });
        check(&guard, ty, &Protector::Nothing);
    }

    #[test]
    fn rejects_shape_mismatches() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let u64_ty = intern_type_tag(&TypeTag::U64, &guard).unwrap();
        let addr_ty = intern_type_tag(&TypeTag::Address, &guard).unwrap();
        let mut heap = Heap::new(1 << 20);
        let mut slot = AlignedBuf::zeroed(48);
        // SAFETY: the slot is aligned and wider than any test type's
        // in-memory size.
        unsafe {
            write_value(&guard, &mut heap, u64_ty, &1u32, slot.as_mut_ptr())
                .expect_err("a u32 must not write as u64");
            write_value(&guard, &mut heap, u64_ty, &vec![1u64], slot.as_mut_ptr())
                .expect_err("a vector must not write as u64");
            write_value(&guard, &mut heap, addr_ty, &1u64, slot.as_mut_ptr())
                .expect_err("a u64 must not write as an address");
            write_value(
                &guard,
                &mut heap,
                u64_ty,
                &AccountAddress::ONE,
                slot.as_mut_ptr(),
            )
            .expect_err("an address must not write as u64");
        }
    }

    #[test]
    fn rejects_a_signer_supplied_as_a_value() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let addr_ty = intern_type_tag(&TypeTag::Address, &guard).unwrap();
        let signer = move_core_types::value::MoveValue::Signer(AccountAddress::ONE);
        let mut heap = Heap::new(1 << 20);
        let mut slot = AlignedBuf::zeroed(48);
        // SAFETY: the slot is aligned and wider than an address's in-memory
        // size.
        unsafe {
            write_value(&guard, &mut heap, addr_ty, &signer, slot.as_mut_ptr())
                .expect_err("a signer must not be placed as an argument");
            // The same address, as an address, still writes.
            let address = move_core_types::value::MoveValue::Address(AccountAddress::ONE);
            write_value(&guard, &mut heap, addr_ty, &address, slot.as_mut_ptr())
                .expect("an address writes as an address");
        }
    }

    #[test]
    fn rejects_a_variant_whose_body_has_the_wrong_field_count() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        // `Some` carries one field, but this enum's second variant has two.
        let ty = intern_type_tag(&tag("m", "Wrong", vec![]), &guard).unwrap();
        let variants = guard.publish_variant_layouts(ty, vec![
            struct_layout(0, 1, vec![]),
            struct_layout(16, 8, vec![
                field(0, reserved(&Type::U64)),
                field(8, reserved(&Type::U64)),
            ]),
        ]);
        guard.publish_layout(
            ty,
            ptr_layout(LayoutKind::FrozenEnum {
                descriptor_id: DescriptorId(0),
                variants,
                max_size_across_variants: 24,
            }),
        );
        let mut heap = Heap::new(1 << 20);
        let mut slot = AlignedBuf::zeroed(48);
        // SAFETY: the slot is aligned and wider than an enum's in-memory
        // size.
        unsafe {
            write_value(&guard, &mut heap, ty, &Some(1u64), slot.as_mut_ptr())
                .expect_err("`Some` must not fill a two-field variant body");
        }
    }
}
