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
    value_layout::{LayoutKind, LayoutProvider},
};
use move_binary_format::file_format::ConstantPoolIndex;
use std::ops::Range;

/// The op's logical access to the frame slot itself, not to memory reached
/// through a pointer stored there. A read/write slot is reported only once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandAccess {
    Read,
    Write,
    ReadWrite,
}

impl OperandAccess {
    pub fn writes(self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

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
                .layout_by_ty(ty)
                .map(|layout| (layout.size, layout.align))
                .ok_or(UnknownOperandLayout::NoLayout(ty))?,
            Constant(idx) => {
                let ty = constants
                    .constant_type(idx)
                    .ok_or(UnknownOperandLayout::NoConstant(idx))?;
                layouts
                    .layout_by_ty(ty)
                    .map(|layout| (layout.size, layout.align))
                    .ok_or(UnknownOperandLayout::ConstantHasNoLayout(idx))?
            },
        })
    }

    /// Bytes known to hold non-pointer data, relative to the operand's start.
    /// Uses only fixed representations and published layout flags, without
    /// walking aggregate fields. Raw byte copies can carry pointers.
    pub fn non_pointer_range(
        self,
        layouts: &(impl LayoutProvider + ?Sized),
        constants: &(impl ConstantPoolProvider + ?Sized),
    ) -> Option<Range<u32>> {
        use OperandKind::*;
        let layout = match self {
            Bool | Byte => return Some(0..1),
            U64 => return Some(0..8),
            Int(width) => return Some(0..width),
            Address => return Some(0..ADDRESS_SLOT.0),
            FatPtr => return Some(PTR_SLOT.0..REF_SLOT.0),
            Ptr | Bytes(_) => return None,
            Value(ty) => layouts.layout_by_ty(ty)?,
            Constant(idx) => layouts.layout_by_ty(constants.constant_type(idx)?)?,
        };
        if layout.has_no_pointers_no_padding() {
            Some(0..layout.size)
        } else if matches!(layout.kind, LayoutKind::Ref) {
            Some(PTR_SLOT.0..REF_SLOT.0)
        } else {
            None
        }
    }
}

/// Declares the frame operands of each `MicroOp` variant and expands to an
/// exhaustive `match` that reports them to `$f`. Struct-field variants list
/// the fields they bind and then `slot => (kind, access)` pairs; payload variants
/// delegate to the payload's own `for_each_frame_operand`.
macro_rules! for_each_frame_operand {
    (
        $op:expr, $f:ident;
        fields: { $( $Variant:ident { $($bind:ident),* } => [ $( $slot:expr => ($kind:expr, $access:expr) ),* ] ),* $(,)? }
        payloads: { $( $PVariant:ident ),* $(,)? }
    ) => {
        match *$op {
            $( MicroOp::$Variant { $($bind,)* .. } => { $( $f($slot, $kind, $access); )* } )*
            $( MicroOp::$PVariant(ref op) => op.for_each_frame_operand($f), )*
        }
    };
}

impl MicroOp {
    /// Reports every frame slot this op reads or writes, with the kind of
    /// access. `DeepCopyHeapPtrs` reports `base + off` saturated, so an
    /// overflowing offset still surfaces as an out-of-frame slot. Calls have
    /// implicit frame effects checked through their ABI; heap effects and GC
    /// relocation are not additional logical frame destinations.
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        use OperandAccess::*;
        use OperandKind::*;
        for_each_frame_operand! {
            self, f;
            fields: {
                // Data movement: immediates are byte arrays; `Move8` is unaligned.
                StoreImm1 { dst } => [dst => (Bytes(1), Write)],
                StoreImm2 { dst } => [dst => (Bytes(2), Write)],
                StoreImm4 { dst } => [dst => (Bytes(4), Write)],
                StoreImm8 { dst } => [dst => (Bytes(8), Write)],
                StoreImm16 { dst } => [dst => (Bytes(16), Write)],
                StoreImm32 { dst } => [dst => (Bytes(32), Write)],
                Move8 { dst, src } => [src => (Bytes(8), Read), dst => (Bytes(8), Write)],
                Move { dst, src, size } => [src => (Bytes(size), Read), dst => (Bytes(size), Write)],
                StoreRandomU64 { dst } => [dst => (U64, Write)],
                // u64 arithmetic.
                AddU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read), dst => (U64, Write)],
                SubU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read), dst => (U64, Write)],
                MulU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read), dst => (U64, Write)],
                DivU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read), dst => (U64, Write)],
                ModU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read), dst => (U64, Write)],
                BitAndU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read), dst => (U64, Write)],
                BitOrU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read), dst => (U64, Write)],
                BitXorU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read), dst => (U64, Write)],
                ShlU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (Byte, Read), dst => (U64, Write)],
                ShrU64 { dst, lhs, rhs } => [lhs => (U64, Read), rhs => (Byte, Read), dst => (U64, Write)],
                AddU64Imm { dst, src } => [src => (U64, Read), dst => (U64, Write)],
                SubU64Imm { dst, src } => [src => (U64, Read), dst => (U64, Write)],
                RSubU64Imm { dst, src } => [src => (U64, Read), dst => (U64, Write)],
                MulU64Imm { dst, src } => [src => (U64, Read), dst => (U64, Write)],
                DivU64Imm { dst, src } => [src => (U64, Read), dst => (U64, Write)],
                ModU64Imm { dst, src } => [src => (U64, Read), dst => (U64, Write)],
                ShlU64Imm { dst, src } => [src => (U64, Read), dst => (U64, Write)],
                ShrU64Imm { dst, src } => [src => (U64, Read), dst => (U64, Write)],
                // Booleans.
                BoolNot { dst, src } => [src => (Bool, Read), dst => (Bool, Write)],
                BoolAnd { dst, lhs, rhs } => [lhs => (Bool, Read), rhs => (Bool, Read), dst => (Bool, Write)],
                BoolOr { dst, lhs, rhs } => [lhs => (Bool, Read), rhs => (Bool, Read), dst => (Bool, Write)],
                // Control flow.
                CallIndirect {} => [],
                CallDirect {} => [],
                CallNative {} => [],
                Return {} => [],
                Jump {} => [],
                JumpNotZeroU64 { src } => [src => (U64, Read)],
                JumpNotZeroByte { src } => [src => (Byte, Read)],
                JumpZeroByte { src } => [src => (Byte, Read)],
                JumpGreaterEqualU64Imm { src } => [src => (U64, Read)],
                JumpLessU64Imm { src } => [src => (U64, Read)],
                JumpGreaterU64Imm { src } => [src => (U64, Read)],
                JumpLessEqualU64Imm { src } => [src => (U64, Read)],
                JumpLessU64 { lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read)],
                JumpGreaterEqualU64 { lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read)],
                JumpNotEqualU64 { lhs, rhs } => [lhs => (U64, Read), rhs => (U64, Read)],
                Abort { code } => [code => (U64, Read)],
                // `message` is an owned `vector<u8>` heap pointer, read by value.
                AbortMsg { code, message } => [code => (U64, Read), message => (Ptr, Read)],
                // Vectors: `vec_ref` is a reference to the vector slot.
                VecNew { dst } => [dst => (Ptr, Write)],
                VecLen { dst, vec_ref } => [vec_ref => (FatPtr, Read), dst => (U64, Write)],
                VecPushBack { vec_ref, elem, elem_size } => [vec_ref => (FatPtr, Read), elem => (Bytes(elem_size), Read)],
                VecPopBack { dst, vec_ref, elem_size } => [vec_ref => (FatPtr, Read), dst => (Bytes(elem_size), Write)],
                VecLoadElem { dst, vec_ref, idx, elem_size } => [vec_ref => (FatPtr, Read), idx => (U64, Read), dst => (Bytes(elem_size), Write)],
                VecStoreElem { vec_ref, idx, src, elem_size } => [vec_ref => (FatPtr, Read), idx => (U64, Read), src => (Bytes(elem_size), Read)],
                VecSwap { vec_ref, idx_a, idx_b } => [vec_ref => (FatPtr, Read), idx_a => (U64, Read), idx_b => (U64, Read)],
                VecBorrow { dst, vec_ref, idx } => [vec_ref => (FatPtr, Read), idx => (U64, Read), dst => (FatPtr, Write)],
                StoreImmVec { dst, idx } => [dst => (Constant(idx), Write)],
                // References. `SlotBorrow.local` is only addressed, never read.
                SlotBorrow { dst } => [dst => (FatPtr, Write)],
                HeapBorrow { dst, obj_ref } => [obj_ref => (FatPtr, Read), dst => (FatPtr, Write)],
                ReadRef { dst, ref_ptr, size } => [ref_ptr => (FatPtr, Read), dst => (Bytes(size), Write)],
                WriteRef { ref_ptr, src, size } => [ref_ptr => (FatPtr, Read), src => (Bytes(size), Read)],
                HeapReadOffset { dst, obj_ref, size } => [obj_ref => (FatPtr, Read), dst => (Bytes(size), Write)],
                HeapWriteOffset { obj_ref, src, size } => [obj_ref => (FatPtr, Read), src => (Bytes(size), Read)],
                DeriveRefOffsetImm { dst_ref, src_ref } => [src_ref => (FatPtr, Read), dst_ref => (FatPtr, Write)],
                ReadRefOffset { dst, ref_ptr, size } => [ref_ptr => (FatPtr, Read), dst => (Bytes(size), Write)],
                WriteRefOffset { ref_ptr, src, size } => [ref_ptr => (FatPtr, Read), src => (Bytes(size), Read)],
                // Heap objects: the pointer slot is aligned; the 8-byte field
                // moves access the frame side unaligned.
                HeapNew { dst } => [dst => (Ptr, Write)],
                HeapMoveFrom8 { dst, heap_ptr } => [heap_ptr => (Ptr, Read), dst => (Bytes(8), Write)],
                HeapMoveTo8 { heap_ptr, src } => [heap_ptr => (Ptr, Read), src => (Bytes(8), Read)],
                HeapMoveToImm8 { heap_ptr } => [heap_ptr => (Ptr, Read)],
                HeapMoveFrom { dst, heap_ptr, size } => [heap_ptr => (Ptr, Read), dst => (Bytes(size), Write)],
                HeapMoveTo { heap_ptr, src, size } => [heap_ptr => (Ptr, Read), src => (Bytes(size), Read)],
                // Global storage: `addr` is an inline address.
                Exists { dst, addr } => [addr => (Address, Read), dst => (Bool, Write)],
                BorrowGlobal { dst, addr } => [addr => (Address, Read), dst => (FatPtr, Write)],
                BorrowGlobalMut { dst, addr } => [addr => (Address, Read), dst => (FatPtr, Write)],
                MoveFrom { dst, addr } => [addr => (Address, Read), dst => (Ptr, Write)],
                MoveTo { signer_ref, src } => [signer_ref => (FatPtr, Read), src => (Ptr, Read)],
                // Debugging.
                ForceGC {} => [],
                // Enums.
                EnumTestTag { dst, enum_ref } => [enum_ref => (FatPtr, Read), dst => (Bool, Write)],
                EnumBorrowVariantFieldByTag { dst, enum_ref } => [enum_ref => (FatPtr, Read), dst => (FatPtr, Write)],
                EnumCheckVariant { enum_ptr } => [enum_ptr => (Ptr, Read)],
                EnumNew { dst } => [dst => (Ptr, Write)],
                EnumReadVariantFieldByTag { dst, enum_ref, size } => [enum_ref => (FatPtr, Read), dst => (Bytes(size), Write)],
                EnumWriteVariantFieldByTag { enum_ref, src, size } => [enum_ref => (FatPtr, Read), src => (Bytes(size), Read)],
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
                f(FrameOffset(base.0.saturating_add(off)), Ptr, ReadWrite);
            }
        }
    }
}

impl IntBinaryOp {
    /// `lhs`, `dst`, and a slot `rhs` are all `rhs.byte_width()` wide.
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        let w = self.rhs.byte_width() as u32;
        f(self.lhs, OperandKind::Int(w), OperandAccess::Read);
        if let Some(rhs) = self.rhs.slot_offset() {
            f(rhs, OperandKind::Int(w), OperandAccess::Read);
        }
        f(self.dst, OperandKind::Int(w), OperandAccess::Write);
    }
}

impl IntShiftOp {
    /// `lhs` and `dst` are `ty.byte_width()` wide; a slot `rhs` is one byte.
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        let w = self.ty.byte_width() as u32;
        f(self.lhs, OperandKind::Int(w), OperandAccess::Read);
        if let ShiftOperand::SlotU8(rhs) = self.rhs {
            f(rhs, OperandKind::Byte, OperandAccess::Read);
        }
        f(self.dst, OperandKind::Int(w), OperandAccess::Write);
    }
}

impl IntNegateOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        let w = self.ty.byte_width() as u32;
        f(self.src, OperandKind::Int(w), OperandAccess::Read);
        f(self.dst, OperandKind::Int(w), OperandAccess::Write);
    }
}

impl IntCastOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        f(
            self.src,
            OperandKind::Int(self.from.byte_width() as u32),
            OperandAccess::Read,
        );
        f(
            self.dst,
            OperandKind::Int(self.to.byte_width() as u32),
            OperandAccess::Write,
        );
    }
}

impl IntCmpOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        let w = self.rhs.byte_width() as u32;
        f(self.lhs, OperandKind::Int(w), OperandAccess::Read);
        if let Some(rhs) = self.rhs.slot_offset() {
            f(rhs, OperandKind::Int(w), OperandAccess::Read);
        }
        f(self.dst, OperandKind::Bool, OperandAccess::Write);
    }
}

impl JumpIntCmpOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        let w = self.rhs.byte_width() as u32;
        f(self.lhs, OperandKind::Int(w), OperandAccess::Read);
        if let Some(rhs) = self.rhs.slot_offset() {
            f(rhs, OperandKind::Int(w), OperandAccess::Read);
        }
    }
}

impl ValueCmpOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        f(self.lhs, OperandKind::Value(self.ty), OperandAccess::Read);
        f(self.rhs, OperandKind::Value(self.ty), OperandAccess::Read);
        f(self.dst, OperandKind::Bool, OperandAccess::Write);
    }
}

impl ValueRefCmpOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        f(self.lhs, OperandKind::FatPtr, OperandAccess::Read);
        f(self.rhs, OperandKind::FatPtr, OperandAccess::Read);
        f(self.dst, OperandKind::Bool, OperandAccess::Write);
    }
}

impl JumpValueCmpOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        f(self.lhs, OperandKind::Value(self.ty), OperandAccess::Read);
        f(self.rhs, OperandKind::Value(self.ty), OperandAccess::Read);
    }
}

impl JumpValueRefCmpOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        f(self.lhs, OperandKind::FatPtr, OperandAccess::Read);
        f(self.rhs, OperandKind::FatPtr, OperandAccess::Read);
    }
}

impl VecPackOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        for &src in &self.srcs {
            f(src, OperandKind::Bytes(self.elem_size), OperandAccess::Read);
        }
        f(self.dst, OperandKind::Ptr, OperandAccess::Write);
    }
}

impl VecUnpackOp {
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        f(self.src, OperandKind::Ptr, OperandAccess::Read);
        for &dst in &self.dsts {
            f(
                dst,
                OperandKind::Bytes(self.elem_size),
                OperandAccess::Write,
            );
        }
    }
}

impl PackClosureOp {
    /// Captured values are byte-copied into the captured-data object.
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        for slot in &self.captured {
            f(
                slot.offset,
                OperandKind::Bytes(slot.size),
                OperandAccess::Read,
            );
        }
        f(self.dst, OperandKind::Ptr, OperandAccess::Write);
    }
}

impl CallClosureOp {
    /// Provided arguments are byte-copied into the callee frame.
    pub fn for_each_frame_operand(
        &self,
        f: &mut dyn FnMut(FrameOffset, OperandKind, OperandAccess),
    ) {
        f(self.closure_src, OperandKind::Ptr, OperandAccess::Read);
        for slot in &self.provided_args {
            f(
                slot.offset,
                OperandKind::Bytes(slot.size),
                OperandAccess::Read,
            );
        }
    }
}
