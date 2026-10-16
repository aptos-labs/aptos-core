// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! For every micro-op, the frame slots it touches and how the interpreter
//! accesses each one. The well-formedness checker checks accesses against this
//! table; any other tool that needs it should use it rather than re-derive it
//! from the interpreter.
//!
//! A typed kind (`Bool`, `U64`, `Ptr`, ...) carries the slot's width and
//! alignment from the layout convention in `types`, which is the table the
//! specializer lays slots out with and is at least as strict as the
//! interpreter's access. `Bytes(n)` is a byte copy of arbitrary data and has no
//! alignment requirement: `Move8` and the frame side of the 8-byte heap moves
//! are `Bytes(8)`, not `U64`.

use super::{
    CallClosureOp, FrameOffset, IntBinaryOp, IntCastOp, IntCmpOp, IntNegateOp, IntShiftOp,
    JumpIntCmpOp, JumpValueCmpOp, JumpValueRefCmpOp, MicroOp, PackClosureOp, ShiftOperand,
    ValueCmpOp, ValueRefCmpOp, VecPackOp, VecUnpackOp,
};
use crate::{
    prepared_module::ConstantPoolProvider,
    types::{int_slot_size_and_align, InternedType, ADDRESS_SLOT, PTR_SLOT, REF_SLOT},
    value_layout::LayoutProvider,
};
use move_binary_format::file_format::ConstantPoolIndex;

/// How the interpreter accesses a frame slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandKind {
    /// A 1-byte `0`/`1` boolean.
    Bool,
    /// A 1-byte integer (shift amounts).
    Byte,
    /// An aligned 8-byte `u64` (`read_u64` / `write_u64`).
    U64,
    /// An aligned 8-byte heap pointer (`read_ptr` / `write_ptr`).
    Ptr,
    /// An aligned 16-byte reference `(base, byte_offset)` (`read_fat_ptr`).
    FatPtr,
    /// An inline `address` or `signer`.
    Address,
    /// `n` bytes moved with a byte copy or an explicitly unaligned load/store.
    Bytes(u32),
    /// An integer slot of `width` bytes accessed with `read_int<T>` /
    /// `write_int<T>`.
    Int(u32),
    /// A by-value comparison operand: occupies the type's in-frame size and
    /// alignment, which only a layout provider can resolve.
    Value(InternedType),
    /// The in-frame image of a constant-pool entry, sized and aligned for
    /// the constant's type, which only the constant pool can resolve.
    Constant(ConstantPoolIndex),
}

/// Why [`OperandKind::width_and_align`] could not determine an operand's
/// width and alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownOperandLayout {
    /// A `Value(ty)` whose type has no layout.
    NoLayout(InternedType),
    /// A `Constant(idx)` that is not in the constant pool.
    NoConstant(ConstantPoolIndex),
    /// A `Constant(idx)` whose type has no layout.
    ConstantHasNoLayout(ConstantPoolIndex),
}

impl OperandKind {
    /// `(width, align)` of the interpreter's access to this operand. Fixed by
    /// the slot convention for every kind except `Value`, which looks up the
    /// type's layout, and `Constant`, which looks up the constant's type and
    /// then its layout.
    pub fn width_and_align(
        self,
        layouts: &(impl LayoutProvider + ?Sized),
        constants: &(impl ConstantPoolProvider + ?Sized),
    ) -> Result<(u32, u32), UnknownOperandLayout> {
        use OperandKind::*;
        Ok(match self {
            Bool | Byte => int_slot_size_and_align(1),
            U64 => int_slot_size_and_align(8),
            Ptr => PTR_SLOT,
            FatPtr => REF_SLOT,
            Address => ADDRESS_SLOT,
            Bytes(n) => (n, 1),
            Int(w) => int_slot_size_and_align(w),
            Value(ty) => layouts
                .size_and_align(ty)
                .ok_or(UnknownOperandLayout::NoLayout(ty))?,
            Constant(idx) => {
                let ty = constants
                    .constant_type(idx)
                    .ok_or(UnknownOperandLayout::NoConstant(idx))?;
                layouts
                    .size_and_align(ty)
                    .ok_or(UnknownOperandLayout::ConstantHasNoLayout(idx))?
            },
        })
    }

    /// Whether the operand is a scalar: a non-pointer value the interpreter
    /// reads or writes as a number, which must never alias a GC pointer slot.
    pub fn is_scalar(self) -> bool {
        use OperandKind::*;
        match self {
            Bool | Byte | U64 | Int(_) | Address => true,
            Ptr | FatPtr | Bytes(_) | Value(_) | Constant(_) => false,
        }
    }
}

/// Declares the frame operands of each `MicroOp` variant and expands to an
/// exhaustive `match` that reports them to `$f`. Struct-field variants list
/// the fields they bind and then `slot: kind` pairs; payload variants
/// delegate to the payload's own `for_each_frame_operand`.
macro_rules! for_each_frame_operand {
    (
        $op:expr, $f:ident;
        fields: { $( $Variant:ident { $($bind:ident),* } => [ $( $slot:expr => $kind:expr ),* ] ),* $(,)? }
        payloads: { $( $PVariant:ident ),* $(,)? }
    ) => {
        match *$op {
            $( MicroOp::$Variant { $($bind,)* .. } => { $( $f($slot, $kind); )* } )*
            $( MicroOp::$PVariant(ref op) => op.for_each_frame_operand($f), )*
        }
    };
}

impl MicroOp {
    /// Reports every frame slot this op reads or writes, with the kind of
    /// access. `DeepCopyHeapPtrs` reports `base + off` saturated, so an
    /// overflowing offset still surfaces as an out-of-frame slot.
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        use OperandKind::*;
        for_each_frame_operand! {
            self, f;
            fields: {
                // Data movement: immediates are byte arrays; `Move8` is unaligned.
                StoreImm1 { dst } => [dst => Bytes(1)],
                StoreImm2 { dst } => [dst => Bytes(2)],
                StoreImm4 { dst } => [dst => Bytes(4)],
                StoreImm8 { dst } => [dst => Bytes(8)],
                StoreImm16 { dst } => [dst => Bytes(16)],
                StoreImm32 { dst } => [dst => Bytes(32)],
                Move8 { dst, src } => [src => Bytes(8), dst => Bytes(8)],
                Move { dst, src, size } => [src => Bytes(size), dst => Bytes(size)],
                StoreRandomU64 { dst } => [dst => U64],
                // u64 arithmetic.
                AddU64 { dst, lhs, rhs } => [lhs => U64, rhs => U64, dst => U64],
                SubU64 { dst, lhs, rhs } => [lhs => U64, rhs => U64, dst => U64],
                MulU64 { dst, lhs, rhs } => [lhs => U64, rhs => U64, dst => U64],
                DivU64 { dst, lhs, rhs } => [lhs => U64, rhs => U64, dst => U64],
                ModU64 { dst, lhs, rhs } => [lhs => U64, rhs => U64, dst => U64],
                BitAndU64 { dst, lhs, rhs } => [lhs => U64, rhs => U64, dst => U64],
                BitOrU64 { dst, lhs, rhs } => [lhs => U64, rhs => U64, dst => U64],
                BitXorU64 { dst, lhs, rhs } => [lhs => U64, rhs => U64, dst => U64],
                ShlU64 { dst, lhs, rhs } => [lhs => U64, rhs => Byte, dst => U64],
                ShrU64 { dst, lhs, rhs } => [lhs => U64, rhs => Byte, dst => U64],
                AddU64Imm { dst, src } => [src => U64, dst => U64],
                SubU64Imm { dst, src } => [src => U64, dst => U64],
                RSubU64Imm { dst, src } => [src => U64, dst => U64],
                MulU64Imm { dst, src } => [src => U64, dst => U64],
                DivU64Imm { dst, src } => [src => U64, dst => U64],
                ModU64Imm { dst, src } => [src => U64, dst => U64],
                ShlU64Imm { dst, src } => [src => U64, dst => U64],
                ShrU64Imm { dst, src } => [src => U64, dst => U64],
                // Booleans.
                BoolNot { dst, src } => [src => Bool, dst => Bool],
                BoolAnd { dst, lhs, rhs } => [lhs => Bool, rhs => Bool, dst => Bool],
                BoolOr { dst, lhs, rhs } => [lhs => Bool, rhs => Bool, dst => Bool],
                // Control flow.
                CallIndirect {} => [],
                CallDirect {} => [],
                CallNative {} => [],
                Return {} => [],
                Jump {} => [],
                JumpNotZeroU64 { src } => [src => U64],
                JumpNotZeroByte { src } => [src => Byte],
                JumpZeroByte { src } => [src => Byte],
                JumpGreaterEqualU64Imm { src } => [src => U64],
                JumpLessU64Imm { src } => [src => U64],
                JumpGreaterU64Imm { src } => [src => U64],
                JumpLessEqualU64Imm { src } => [src => U64],
                JumpLessU64 { lhs, rhs } => [lhs => U64, rhs => U64],
                JumpGreaterEqualU64 { lhs, rhs } => [lhs => U64, rhs => U64],
                JumpNotEqualU64 { lhs, rhs } => [lhs => U64, rhs => U64],
                Abort { code } => [code => U64],
                // `message` is an owned `vector<u8>` heap pointer, read by value.
                AbortMsg { code, message } => [code => U64, message => Ptr],
                // Vectors: `vec_ref` is a reference to the vector slot.
                VecNew { dst } => [dst => Ptr],
                VecLen { dst, vec_ref } => [vec_ref => FatPtr, dst => U64],
                VecPushBack { vec_ref, elem, elem_size } => [vec_ref => FatPtr, elem => Bytes(elem_size)],
                VecPopBack { dst, vec_ref, elem_size } => [vec_ref => FatPtr, dst => Bytes(elem_size)],
                VecLoadElem { dst, vec_ref, idx, elem_size } => [vec_ref => FatPtr, idx => U64, dst => Bytes(elem_size)],
                VecStoreElem { vec_ref, idx, src, elem_size } => [vec_ref => FatPtr, idx => U64, src => Bytes(elem_size)],
                VecSwap { vec_ref, idx_a, idx_b } => [vec_ref => FatPtr, idx_a => U64, idx_b => U64],
                VecBorrow { dst, vec_ref, idx } => [vec_ref => FatPtr, idx => U64, dst => FatPtr],
                StoreImmVec { dst, idx } => [dst => Constant(idx)],
                // References. `SlotBorrow.local` is only addressed, never read.
                SlotBorrow { dst } => [dst => FatPtr],
                HeapBorrow { dst, obj_ref } => [obj_ref => FatPtr, dst => FatPtr],
                ReadRef { dst, ref_ptr, size } => [ref_ptr => FatPtr, dst => Bytes(size)],
                WriteRef { ref_ptr, src, size } => [ref_ptr => FatPtr, src => Bytes(size)],
                HeapReadOffset { dst, obj_ref, size } => [obj_ref => FatPtr, dst => Bytes(size)],
                HeapWriteOffset { obj_ref, src, size } => [obj_ref => FatPtr, src => Bytes(size)],
                DeriveRefOffsetImm { dst_ref, src_ref } => [src_ref => FatPtr, dst_ref => FatPtr],
                ReadRefOffset { dst, ref_ptr, size } => [ref_ptr => FatPtr, dst => Bytes(size)],
                WriteRefOffset { ref_ptr, src, size } => [ref_ptr => FatPtr, src => Bytes(size)],
                // Heap objects: the pointer slot is aligned; the 8-byte field
                // moves access the frame side unaligned.
                HeapNew { dst } => [dst => Ptr],
                HeapMoveFrom8 { dst, heap_ptr } => [heap_ptr => Ptr, dst => Bytes(8)],
                HeapMoveTo8 { heap_ptr, src } => [heap_ptr => Ptr, src => Bytes(8)],
                HeapMoveToImm8 { heap_ptr } => [heap_ptr => Ptr],
                HeapMoveFrom { dst, heap_ptr, size } => [heap_ptr => Ptr, dst => Bytes(size)],
                HeapMoveTo { heap_ptr, src, size } => [heap_ptr => Ptr, src => Bytes(size)],
                // Global storage: `addr` is an inline address.
                Exists { dst, addr } => [addr => Address, dst => Bool],
                BorrowGlobal { dst, addr } => [addr => Address, dst => FatPtr],
                BorrowGlobalMut { dst, addr } => [addr => Address, dst => FatPtr],
                MoveFrom { dst, addr } => [addr => Address, dst => Ptr],
                MoveTo { signer_ref, src } => [signer_ref => FatPtr, src => Ptr],
                // Debugging.
                ForceGC {} => [],
                // Enums.
                EnumTestTag { dst, enum_ref } => [enum_ref => FatPtr, dst => Bool],
                EnumBorrowVariantFieldByTag { dst, enum_ref } => [enum_ref => FatPtr, dst => FatPtr],
                EnumCheckVariant { enum_ptr } => [enum_ptr => Ptr],
                EnumNew { dst } => [dst => Ptr],
                EnumReadVariantFieldByTag { dst, enum_ref, size } => [enum_ref => FatPtr, dst => Bytes(size)],
                EnumWriteVariantFieldByTag { enum_ref, src, size } => [enum_ref => FatPtr, src => Bytes(size)],
                // Reported below: each slot is `base + off`.
                DeepCopyHeapPtrs {} => [],
            }
            payloads: {
                IntAdd, IntSub, IntMul, IntDiv, IntMod, IntBitAnd, IntBitOr, IntBitXor,
                IntShl, IntShr, IntNegate, IntCast,
                IntCmp, ValueCmp, ValueRefCmp,
                JumpIntCmp, JumpValueCmp, JumpValueRefCmp,
                VecPack, VecUnpack, PackClosure, CallClosure,
            }
        }
        // Not expressible as a fixed field list: each owned pointer sits at
        // `base + off`.
        if let MicroOp::DeepCopyHeapPtrs { base, ref offsets } = *self {
            for &off in offsets.iter() {
                f(FrameOffset(base.0.saturating_add(off)), Ptr);
            }
        }
    }
}

impl IntBinaryOp {
    /// `lhs`, `dst`, and a slot `rhs` are all `rhs.byte_width()` wide.
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        let w = self.rhs.byte_width() as u32;
        f(self.lhs, OperandKind::Int(w));
        if let Some(rhs) = self.rhs.slot_offset() {
            f(rhs, OperandKind::Int(w));
        }
        f(self.dst, OperandKind::Int(w));
    }
}

impl IntShiftOp {
    /// `lhs` and `dst` are `ty.byte_width()` wide; a slot `rhs` is one byte.
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        let w = self.ty.byte_width() as u32;
        f(self.lhs, OperandKind::Int(w));
        if let ShiftOperand::SlotU8(rhs) = self.rhs {
            f(rhs, OperandKind::Byte);
        }
        f(self.dst, OperandKind::Int(w));
    }
}

impl IntNegateOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        let w = self.ty.byte_width() as u32;
        f(self.src, OperandKind::Int(w));
        f(self.dst, OperandKind::Int(w));
    }
}

impl IntCastOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        f(self.src, OperandKind::Int(self.from.byte_width() as u32));
        f(self.dst, OperandKind::Int(self.to.byte_width() as u32));
    }
}

impl IntCmpOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        let w = self.rhs.byte_width() as u32;
        f(self.lhs, OperandKind::Int(w));
        if let Some(rhs) = self.rhs.slot_offset() {
            f(rhs, OperandKind::Int(w));
        }
        f(self.dst, OperandKind::Bool);
    }
}

impl JumpIntCmpOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        let w = self.rhs.byte_width() as u32;
        f(self.lhs, OperandKind::Int(w));
        if let Some(rhs) = self.rhs.slot_offset() {
            f(rhs, OperandKind::Int(w));
        }
    }
}

impl ValueCmpOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        f(self.lhs, OperandKind::Value(self.ty));
        f(self.rhs, OperandKind::Value(self.ty));
        f(self.dst, OperandKind::Bool);
    }
}

impl ValueRefCmpOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        f(self.lhs, OperandKind::FatPtr);
        f(self.rhs, OperandKind::FatPtr);
        f(self.dst, OperandKind::Bool);
    }
}

impl JumpValueCmpOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        f(self.lhs, OperandKind::Value(self.ty));
        f(self.rhs, OperandKind::Value(self.ty));
    }
}

impl JumpValueRefCmpOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        f(self.lhs, OperandKind::FatPtr);
        f(self.rhs, OperandKind::FatPtr);
    }
}

impl VecPackOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        for &src in &self.srcs {
            f(src, OperandKind::Bytes(self.elem_size));
        }
        f(self.dst, OperandKind::Ptr);
    }
}

impl VecUnpackOp {
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        f(self.src, OperandKind::Ptr);
        for &dst in &self.dsts {
            f(dst, OperandKind::Bytes(self.elem_size));
        }
    }
}

impl PackClosureOp {
    /// Captured values are byte-copied into the captured-data object.
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        for slot in &self.captured {
            f(slot.offset, OperandKind::Bytes(slot.size));
        }
        f(self.dst, OperandKind::Ptr);
    }
}

impl CallClosureOp {
    /// Provided arguments are byte-copied into the callee frame.
    pub fn for_each_frame_operand(&self, f: &mut dyn FnMut(FrameOffset, OperandKind)) {
        f(self.closure_src, OperandKind::Ptr);
        for slot in &self.provided_args {
            f(slot.offset, OperandKind::Bytes(slot.size));
        }
    }
}
