// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Property tests for the integer micro-ops: arithmetic, bitwise, shift,
//! negate, and cast.
//!
//! For every supported (type, kind) combination, the property is the same
//! shape:
//!
//! > For any (lhs, rhs) input, the runtime's output matches Rust's
//! > reference impl (`T::checked_*` for arithmetic, native `& | ^` for
//! > bitwise, `T::checked_shl` / `T::checked_shr` for shift). Both
//! > succeed with the same value, or both abort.
//!
//! Each `prop_*` macro takes an optional `specialized` flag at the end.
//! Without the flag (default), the test builds the MicroOp through the
//! unspecialized per-kind path; with `specialized`, the test builds the
//! u64 fast-path variant. Only u64 has a specialized form today, so the
//! flag is u64-only. For u64 we run both forms; for every other width
//! only the default applies.
//!
//! ```ignore
//! prop_arith!(u64_add,             u64, Add, u64::checked_add);
//! prop_arith!(u64_add_specialized, u64, Add, u64::checked_add, specialized);
//! prop_arith!(u8_add,              u8,  Add, u8::checked_add);
//! ```
//!
//! Each property fires the proptest default (256 cases). Edge values
//! (MAX, MIN, 0) get hit through proptest's shrinker + random sampling.
//!
//! The cast micro-op ([`MicroOp::IntCast`]) is also tested here, in the
//! cast section at the end, reusing the same single-op harness.
//!
//! TODO(correctness): Revisit endianness. Interpreter uses native order for built-in types.
//! I256/U256 currently do not have other endianness exposed.

mod common;

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    native::NativeExtensions, Code, FrameLayoutInfo, FrameOffset as FO, Function, IntBinaryOp,
    IntCastOp, IntNegateOp, IntOperand, IntShiftOp, IntTy, MicroOp, ShiftOperand,
    SortedSafePointEntries, FRAME_METADATA_SIZE,
};
use move_core_types::int256::{I256, U256};
use num_bigint::{BigInt, Sign};
use proptest::{prelude::*, strategy::BoxedStrategy};

// ---------------------------------------------------------------------------
// Test-local op-kind enums
// ---------------------------------------------------------------------------
//
// The `MicroOp` enum is now per-kind (`IntAdd`, `IntSub`, …) and carries no
// kind field. The test helpers still want to dispatch by kind in one place,
// so we keep two small enums here purely for that purpose — they don't
// appear on the wire.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShiftKind {
    Shl,
    Shr,
}

// ---------------------------------------------------------------------------
// Type → operand mapping (test-local)
// ---------------------------------------------------------------------------
//
// `IntTypeOperand` ties a Rust integer type to its [`IntOperand`] reg /
// imm constructors. `UnsignedTypeTag` ties an unsigned Rust integer to
// its [`IntTy`] tag for shift dispatch. Both are
// test-only — production code constructs operands directly.

trait IntTypeOperand: Copy {
    fn reg_operand(off: FO) -> IntOperand;
    fn imm_operand(self) -> IntOperand;
}

macro_rules! impl_int_type_operand_inline {
    ($ty:ident, $reg:ident, $imm:ident) => {
        impl IntTypeOperand for $ty {
            fn reg_operand(off: FO) -> IntOperand {
                IntOperand::$reg(off)
            }

            fn imm_operand(self) -> IntOperand {
                IntOperand::$imm(self)
            }
        }
    };
}

macro_rules! impl_int_type_operand_boxed {
    ($ty:ty, $reg:ident, $imm:ident) => {
        impl IntTypeOperand for $ty {
            fn reg_operand(off: FO) -> IntOperand {
                IntOperand::$reg(off)
            }

            fn imm_operand(self) -> IntOperand {
                IntOperand::$imm(Box::new(self))
            }
        }
    };
}

impl_int_type_operand_inline!(u8, SlotU8, ImmU8);
impl_int_type_operand_inline!(u16, SlotU16, ImmU16);
impl_int_type_operand_inline!(u32, SlotU32, ImmU32);
impl_int_type_operand_inline!(u64, SlotU64, ImmU64);
impl_int_type_operand_inline!(i8, SlotI8, ImmI8);
impl_int_type_operand_inline!(i16, SlotI16, ImmI16);
impl_int_type_operand_inline!(i32, SlotI32, ImmI32);
impl_int_type_operand_inline!(i64, SlotI64, ImmI64);
impl_int_type_operand_boxed!(u128, SlotU128, ImmU128);
impl_int_type_operand_boxed!(U256, SlotU256, ImmU256);
impl_int_type_operand_boxed!(i128, SlotI128, ImmI128);
impl_int_type_operand_boxed!(I256, SlotI256, ImmI256);

trait UnsignedTypeTag {
    const UNSIGNED_TY: IntTy;
}

impl UnsignedTypeTag for u8 {
    const UNSIGNED_TY: IntTy = IntTy::U8;
}
impl UnsignedTypeTag for u16 {
    const UNSIGNED_TY: IntTy = IntTy::U16;
}
impl UnsignedTypeTag for u32 {
    const UNSIGNED_TY: IntTy = IntTy::U32;
}
impl UnsignedTypeTag for u64 {
    const UNSIGNED_TY: IntTy = IntTy::U64;
}
impl UnsignedTypeTag for u128 {
    const UNSIGNED_TY: IntTy = IntTy::U128;
}
impl UnsignedTypeTag for U256 {
    const UNSIGNED_TY: IntTy = IntTy::U256;
}

trait SignedTypeTag {
    const SIGNED_TY: IntTy;
}

impl SignedTypeTag for i8 {
    const SIGNED_TY: IntTy = IntTy::I8;
}
impl SignedTypeTag for i16 {
    const SIGNED_TY: IntTy = IntTy::I16;
}
impl SignedTypeTag for i32 {
    const SIGNED_TY: IntTy = IntTy::I32;
}
impl SignedTypeTag for i64 {
    const SIGNED_TY: IntTy = IntTy::I64;
}
impl SignedTypeTag for i128 {
    const SIGNED_TY: IntTy = IntTy::I128;
}
impl SignedTypeTag for I256 {
    const SIGNED_TY: IntTy = IntTy::I256;
}

// ---------------------------------------------------------------------------
// Frame layout
// ---------------------------------------------------------------------------
//
// One 96-byte frame serves every width: three 32-byte slots at 8-byte
// aligned offsets, large enough for any supported integer type. Smaller
// widths just touch a prefix; the verifier only checks `byte_width`
// many bytes per slot.

const SLOT_DST: u32 = 0;
const SLOT_LHS: u32 = 32;
const SLOT_RHS: u32 = 64;
const FRAME_SIZE: u32 = 96;

fn make_func(op: MicroOp) -> Function {
    Function {
        name: GlobalArenaPtr::from_static("op"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![op, MicroOp::Return]),
        entry_gas: 0,
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: FRAME_SIZE as usize,
        extended_frame_size: FRAME_SIZE as usize + FRAME_METADATA_SIZE,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

/// Build and run a single-op function with the given lhs/rhs bytes.
/// Returns the first `dst_size` bytes of the dst slot, or `Err` if the
/// interpreter aborted.
fn run_wide(
    op: MicroOp,
    lhs_bytes: &[u8],
    rhs_bytes: &[u8],
    dst_size: usize,
) -> Result<Vec<u8>, anyhow::Error> {
    let func = make_func(op);
    common::with_test_interpreter(&func, u64::MAX, NativeExtensions::new(), |ctx| {
        if !lhs_bytes.is_empty() {
            ctx.set_root_arg(SLOT_LHS, lhs_bytes);
        }
        if !rhs_bytes.is_empty() {
            ctx.set_root_arg(SLOT_RHS, rhs_bytes);
        }
        ctx.run().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut out = vec![0u8; dst_size];
        let mut i = 0usize;
        while i < dst_size {
            let word = ctx.root_result_at(SLOT_DST + i as u32);
            let bytes = word.to_ne_bytes();
            let copy_n = (dst_size - i).min(8);
            out[i..i + copy_n].copy_from_slice(&bytes[..copy_n]);
            i += 8;
        }
        Ok(out)
    })
}

// ---------------------------------------------------------------------------
// Byte-conversion trait
// ---------------------------------------------------------------------------
//
// Lets the property helpers (`run_binop` / `run_unop` / `run_shift`) be
// generic over both native widths (u8…i128) and the big-int wrappers
// (U256, I256). Each impl just delegates to the type's own to/from bytes
// API.

trait IntBytes: Copy + std::fmt::Debug + PartialEq {
    const SIZE: usize;
    fn to_bytes(self) -> Vec<u8>;
    fn from_bytes(b: &[u8]) -> Self;
}

macro_rules! impl_int_bytes_native {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntBytes for $ty {
                const SIZE: usize = std::mem::size_of::<$ty>();
                fn to_bytes(self) -> Vec<u8> { self.to_ne_bytes().to_vec() }
                fn from_bytes(b: &[u8]) -> Self {
                    let mut arr = [0u8; std::mem::size_of::<$ty>()];
                    arr.copy_from_slice(b);
                    <$ty>::from_ne_bytes(arr)
                }
            }
        )*
    };
}

impl_int_bytes_native!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl IntBytes for U256 {
    const SIZE: usize = 32;

    fn to_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(b: &[u8]) -> Self {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(b);
        Self::from_le_bytes(arr)
    }
}

impl IntBytes for I256 {
    const SIZE: usize = 32;

    fn to_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(b: &[u8]) -> Self {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(b);
        Self::from_le_bytes(arr)
    }
}

/// Run a typed binary op: writes `lhs` and `rhs` into the input slots,
/// runs the op, decodes the dst bytes back into `T`. `None` indicates
/// the interpreter aborted.
fn run_binop<T: IntBytes>(op: MicroOp, lhs: T, rhs: T) -> Option<T> {
    run_wide(op, &lhs.to_bytes(), &rhs.to_bytes(), T::SIZE)
        .ok()
        .map(|b| T::from_bytes(&b))
}

/// Run a typed unary op (one runtime input + an immediate baked into the
/// op): writes `src` into the lhs slot, runs the op, decodes the dst
/// bytes back into `T`.
fn run_unop<T: IntBytes>(op: MicroOp, src: T) -> Option<T> {
    run_wide(op, &src.to_bytes(), &[], T::SIZE)
        .ok()
        .map(|b| T::from_bytes(&b))
}

/// Run a shift op where `rhs` is a 1-byte shift amount.
fn run_shift<T: IntBytes>(op: MicroOp, lhs: T, shift: u8) -> Option<T> {
    run_wide(op, &lhs.to_bytes(), &[shift], T::SIZE)
        .ok()
        .map(|b| T::from_bytes(&b))
}

// ---------------------------------------------------------------------------
// MicroOp constructors
// ---------------------------------------------------------------------------
//
// Two flavours per op family, each in both reg-reg and imm forms:
// - `u64_*` for the specialized [`MicroOp::AddU64`] / [`MicroOp::AddU64Imm`]
//   etc. variants.
// - `unspec_*` for the tag-dispatched [`MicroOp::IntBinary`] /
//   [`MicroOp::IntBinaryImm`] / [`MicroOp::IntBinary`] /
//   [`MicroOp::IntBinaryImm`] / [`MicroOp::IntShift`] /
//   [`MicroOp::IntShiftImm`] families.
//
// The `*_op!` dispatch macros below pick between them based on the Rust
// type token. `bit_imm_op!` has no u64 arm because no u64 specialized
// bitwise imm variant exists.

fn u64_binary_op(kind: BinKind) -> MicroOp {
    let dst = FO(SLOT_DST);
    let lhs = FO(SLOT_LHS);
    let rhs = FO(SLOT_RHS);
    match kind {
        BinKind::Add => MicroOp::AddU64 { dst, lhs, rhs },
        BinKind::Sub => MicroOp::SubU64 { dst, lhs, rhs },
        BinKind::Mul => MicroOp::MulU64 { dst, lhs, rhs },
        BinKind::Div => MicroOp::DivU64 { dst, lhs, rhs },
        BinKind::Mod => MicroOp::ModU64 { dst, lhs, rhs },
        BinKind::BitAnd => MicroOp::BitAndU64 { dst, lhs, rhs },
        BinKind::BitOr => MicroOp::BitOrU64 { dst, lhs, rhs },
        BinKind::BitXor => MicroOp::BitXorU64 { dst, lhs, rhs },
    }
}

fn u64_shift_op(kind: ShiftKind) -> MicroOp {
    let dst = FO(SLOT_DST);
    let lhs = FO(SLOT_LHS);
    let rhs = FO(SLOT_RHS);
    match kind {
        ShiftKind::Shl => MicroOp::ShlU64 { dst, lhs, rhs },
        ShiftKind::Shr => MicroOp::ShrU64 { dst, lhs, rhs },
    }
}

/// u64-specialized imm op. Bitwise kinds aren't reachable today — no
/// `BitAndU64Imm` / etc. variant exists yet — so they're `unreachable!()`.
fn u64_binary_imm_op(kind: BinKind, imm: u64) -> MicroOp {
    let dst = FO(SLOT_DST);
    let src = FO(SLOT_LHS);
    match kind {
        BinKind::Add => MicroOp::AddU64Imm { dst, src, imm },
        BinKind::Sub => MicroOp::SubU64Imm { dst, src, imm },
        BinKind::Mul => MicroOp::MulU64Imm { dst, src, imm },
        BinKind::Div => MicroOp::DivU64Imm { dst, src, imm },
        BinKind::Mod => MicroOp::ModU64Imm { dst, src, imm },
        BinKind::BitAnd | BinKind::BitOr | BinKind::BitXor => {
            unreachable!("no u64 bitwise imm variants")
        },
    }
}

fn u64_shift_imm_op(kind: ShiftKind, imm: u8) -> MicroOp {
    let dst = FO(SLOT_DST);
    let src = FO(SLOT_LHS);
    match kind {
        ShiftKind::Shl => MicroOp::ShlU64Imm { dst, src, imm },
        ShiftKind::Shr => MicroOp::ShrU64Imm { dst, src, imm },
    }
}

/// Wrap an [`IntBinaryOp`] in the [`MicroOp`] variant matching `kind`.
fn wrap_binary(kind: BinKind, op: IntBinaryOp) -> MicroOp {
    match kind {
        BinKind::Add => MicroOp::IntAdd(op),
        BinKind::Sub => MicroOp::IntSub(op),
        BinKind::Mul => MicroOp::IntMul(op),
        BinKind::Div => MicroOp::IntDiv(op),
        BinKind::Mod => MicroOp::IntMod(op),
        BinKind::BitAnd => MicroOp::IntBitAnd(op),
        BinKind::BitOr => MicroOp::IntBitOr(op),
        BinKind::BitXor => MicroOp::IntBitXor(op),
    }
}

/// Wrap an [`IntShiftOp`] in the [`MicroOp`] variant matching `kind`.
fn wrap_shift(kind: ShiftKind, op: IntShiftOp) -> MicroOp {
    match kind {
        ShiftKind::Shl => MicroOp::IntShl(op),
        ShiftKind::Shr => MicroOp::IntShr(op),
    }
}

fn unspec_binary_op<T: IntTypeOperand>(kind: BinKind) -> MicroOp {
    wrap_binary(kind, IntBinaryOp {
        dst: FO(SLOT_DST),
        lhs: FO(SLOT_LHS),
        rhs: T::reg_operand(FO(SLOT_RHS)),
    })
}

fn unspec_shift_op<T: UnsignedTypeTag>(kind: ShiftKind) -> MicroOp {
    wrap_shift(kind, IntShiftOp {
        ty: T::UNSIGNED_TY,
        dst: FO(SLOT_DST),
        lhs: FO(SLOT_LHS),
        rhs: ShiftOperand::SlotU8(FO(SLOT_RHS)),
    })
}

fn unspec_binary_imm_op<T: IntTypeOperand>(kind: BinKind, imm_val: T) -> MicroOp {
    wrap_binary(kind, IntBinaryOp {
        dst: FO(SLOT_DST),
        lhs: FO(SLOT_LHS),
        rhs: imm_val.imm_operand(),
    })
}

fn unspec_shift_imm_op<T: UnsignedTypeTag>(kind: ShiftKind, imm: u8) -> MicroOp {
    wrap_shift(kind, IntShiftOp {
        ty: T::UNSIGNED_TY,
        dst: FO(SLOT_DST),
        lhs: FO(SLOT_LHS),
        rhs: ShiftOperand::ImmU8(imm),
    })
}

/// Builds an [`MicroOp::IntNegate`] for any signed integer type
/// `T: SignedTypeTag`.
fn unspec_negate_op<T: SignedTypeTag>() -> MicroOp {
    MicroOp::IntNegate(IntNegateOp {
        ty: T::SIGNED_TY,
        dst: FO(SLOT_DST),
        src: FO(SLOT_LHS),
    })
}

// ---------------------------------------------------------------------------
// Type → MicroOp dispatch
// ---------------------------------------------------------------------------
//
// Each `prop_*` macro takes an optional `specialized` flag. Without the
// flag (the default), the MicroOp is built via the unspecialized helper
// — exercising the per-kind [`IntOperand`] dispatch in the runtime.
// With the flag, the MicroOp is built via the u64 specialized helper;
// this form only matches when `$ty == u64`, so passing the flag with
// any other type is a compile error.

macro_rules! negate_op {
    ($ty:tt) => {
        unspec_negate_op::<$ty>()
    };
}

// ---------------------------------------------------------------------------
// Strategy dispatch
// ---------------------------------------------------------------------------
//
// `any::<T>()` works for native widths; U256 / I256 aren't `Arbitrary` so
// we generate 32 random bytes and reinterpret.

fn u256_strategy() -> impl Strategy<Value = U256> {
    any::<[u8; 32]>().prop_map(U256::from_le_bytes)
}

fn i256_strategy() -> impl Strategy<Value = I256> {
    any::<[u8; 32]>().prop_map(I256::from_le_bytes)
}

macro_rules! strat {
    (U256) => {
        u256_strategy()
    };
    (I256) => {
        i256_strategy()
    };
    ($ty:tt) => {
        any::<$ty>()
    };
}

/// Strategy that produces nonzero values of `$ty` (for Div / Mod imm,
/// which the verifier rejects when imm == 0).
macro_rules! nonzero {
    (U256) => {
        u256_strategy().prop_filter("nonzero", |v| *v != U256::ZERO)
    };
    (I256) => {
        i256_strategy().prop_filter("nonzero", |v| *v != I256::ZERO)
    };
    ($ty:tt) => {
        any::<$ty>().prop_filter("nonzero", |v| *v != 0)
    };
}

/// Build a [`U256`] from a `u8`. Used by the U256 shift reference impl.
fn u256_from_u8(s: u8) -> U256 {
    let mut bytes = [0u8; 32];
    bytes[0] = s;
    U256::from_le_bytes(bytes)
}

// ---------------------------------------------------------------------------
// Property macros
// ---------------------------------------------------------------------------
//
// Each generates one `proptest!` block. The shape is uniform: build the
// op via the dispatch macro, compute the reference, run the runtime,
// compare. The u64 specialized and unspecialized paths share these
// bodies — only the MicroOp produced by the dispatch differs.

/// Arithmetic property: runtime output matches `$ref_fn(a, b) -> Option<T>`.
/// Default form exercises the unspecialized per-kind path; the
/// `specialized` flag (u64-only) exercises the specialized fast path.
macro_rules! prop_arith {
    ($name:ident, $ty:tt, $kind:ident, $ref_fn:expr) => {
        proptest! {
            #[test]
            fn $name(a in strat!($ty), b in strat!($ty)) {
                let expected: Option<$ty> = $ref_fn(a, b);
                let op = unspec_binary_op::<$ty>(BinKind::$kind);
                let actual: Option<$ty> = run_binop::<$ty>(op, a, b);
                prop_assert_eq!(expected, actual);
            }
        }
    };
    ($name:ident,u64, $kind:ident, $ref_fn:expr,specialized) => {
        proptest! {
            #[test]
            fn $name(a in strat!(u64), b in strat!(u64)) {
                let expected: Option<u64> = $ref_fn(a, b);
                let op = u64_binary_op(BinKind::$kind);
                let actual: Option<u64> = run_binop::<u64>(op, a, b);
                prop_assert_eq!(expected, actual);
            }
        }
    };
}

/// Bitwise property: infallible, so the reference returns `T` directly
/// and the runtime is expected to never abort. Default → unspec; flag →
/// u64-specialized.
macro_rules! prop_bit {
    ($name:ident, $ty:tt, $kind:ident, $ref_fn:expr) => {
        proptest! {
            #[test]
            fn $name(a in strat!($ty), b in strat!($ty)) {
                let expected: $ty = $ref_fn(a, b);
                let op = unspec_binary_op::<$ty>(BinKind::$kind);
                let actual: Option<$ty> = run_binop::<$ty>(op, a, b);
                prop_assert_eq!(Some(expected), actual);
            }
        }
    };
    ($name:ident,u64, $kind:ident, $ref_fn:expr,specialized) => {
        proptest! {
            #[test]
            fn $name(a in strat!(u64), b in strat!(u64)) {
                let expected: u64 = $ref_fn(a, b);
                let op = u64_binary_op(BinKind::$kind);
                let actual: Option<u64> = run_binop::<u64>(op, a, b);
                prop_assert_eq!(Some(expected), actual);
            }
        }
    };
}

/// Shift property: shift amount is always `u8`; the reference returns
/// `Option<T>` so native widths can express the out-of-range abort case
/// and U256 (where shift < 256 by construction) returns `Some(_)` always.
/// Default → unspec; flag → u64-specialized.
macro_rules! prop_shift {
    ($name:ident, $ty:tt, $kind:ident, $ref_fn:expr) => {
        proptest! {
            #[test]
            fn $name(a in strat!($ty), s in any::<u8>()) {
                let expected: Option<$ty> = $ref_fn(a, s);
                let op = unspec_shift_op::<$ty>(ShiftKind::$kind);
                let actual: Option<$ty> = run_shift::<$ty>(op, a, s);
                prop_assert_eq!(expected, actual);
            }
        }
    };
    ($name:ident,u64, $kind:ident, $ref_fn:expr,specialized) => {
        proptest! {
            #[test]
            fn $name(a in strat!(u64), s in any::<u8>()) {
                let expected: Option<u64> = $ref_fn(a, s);
                let op = u64_shift_op(ShiftKind::$kind);
                let actual: Option<u64> = run_shift::<u64>(op, a, s);
                prop_assert_eq!(expected, actual);
            }
        }
    };
}

/// Property for u64 immediate-form ops with no unspecialized counterpart
/// (currently just `RSubU64Imm`).
macro_rules! prop_imm {
    ($name:ident, $variant:ident, $imm_strategy:expr, $ref_fn:expr) => {
        proptest! {
            #[test]
            fn $name(a in any::<u64>(), imm in $imm_strategy) {
                let op = MicroOp::$variant { dst: FO(SLOT_DST), src: FO(SLOT_LHS), imm };
                let expected: Option<u64> = $ref_fn(a, imm);
                let actual: Option<u64> = run_unop::<u64>(op, a);
                prop_assert_eq!(expected, actual);
            }
        }
    };
}

/// Arithmetic imm property: runtime matches `$ref_fn(a, imm) -> Option<T>`.
/// `$imm_strategy` is bounded for Div / Mod kinds (must be nonzero) and
/// open for Add / Sub / Mul. Default → unspec; flag → u64-specialized.
macro_rules! prop_arith_imm {
    ($name:ident, $ty:tt, $kind:ident, $imm_strategy:expr, $ref_fn:expr) => {
        proptest! {
            #[test]
            fn $name(a in strat!($ty), imm in $imm_strategy) {
                let expected: Option<$ty> = $ref_fn(a, imm);
                let op = unspec_binary_imm_op::<$ty>(BinKind::$kind, imm);
                let actual: Option<$ty> = run_unop::<$ty>(op, a);
                prop_assert_eq!(expected, actual);
            }
        }
    };
    ($name:ident,u64, $kind:ident, $imm_strategy:expr, $ref_fn:expr,specialized) => {
        proptest! {
            #[test]
            fn $name(a in strat!(u64), imm in $imm_strategy) {
                let expected: Option<u64> = $ref_fn(a, imm);
                let op = u64_binary_imm_op(BinKind::$kind, imm);
                let actual: Option<u64> = run_unop::<u64>(op, a);
                prop_assert_eq!(expected, actual);
            }
        }
    };
}

/// Bitwise imm property. Infallible — no Option wrapping on the
/// reference side. No specialized form: u64 bitwise imm has no
/// specialized variant, so every width goes through the unspecialized
/// path.
macro_rules! prop_bit_imm {
    ($name:ident, $ty:tt, $kind:ident, $ref_fn:expr) => {
        proptest! {
            #[test]
            fn $name(a in strat!($ty), imm in strat!($ty)) {
                let expected: $ty = $ref_fn(a, imm);
                let op = unspec_binary_imm_op::<$ty>(BinKind::$kind, imm);
                let actual: Option<$ty> = run_unop::<$ty>(op, a);
                prop_assert_eq!(Some(expected), actual);
            }
        }
    };
}

/// Shift imm property. `$imm_strategy` is bounded to `0..bit_width($ty)`
/// for native widths so the verifier accepts the op; u256 uses
/// `any::<u8>()` because u8 caps at 255 < 256 = bit_width. Default →
/// unspec; flag → u64-specialized.
macro_rules! prop_shift_imm {
    ($name:ident, $ty:tt, $kind:ident, $imm_strategy:expr, $ref_fn:expr) => {
        proptest! {
            #[test]
            fn $name(a in strat!($ty), imm in $imm_strategy) {
                let expected: Option<$ty> = $ref_fn(a, imm);
                let op = unspec_shift_imm_op::<$ty>(ShiftKind::$kind, imm);
                let actual: Option<$ty> = run_unop::<$ty>(op, a);
                prop_assert_eq!(expected, actual);
            }
        }
    };
    ($name:ident,u64, $kind:ident, $imm_strategy:expr, $ref_fn:expr,specialized) => {
        proptest! {
            #[test]
            fn $name(a in strat!(u64), imm in $imm_strategy) {
                let expected: Option<u64> = $ref_fn(a, imm);
                let op = u64_shift_imm_op(ShiftKind::$kind, imm);
                let actual: Option<u64> = run_unop::<u64>(op, a);
                prop_assert_eq!(expected, actual);
            }
        }
    };
}

macro_rules! prop_negate {
    ($name:ident, $ty:tt, $ref_fn:expr) => {
        proptest! {
            #[test]
            fn $name(a in strat!($ty)) {
                let expected: Option<$ty> = $ref_fn(a);
                let actual: Option<$ty> = run_unop::<$ty>(negate_op!($ty), a);
                prop_assert_eq!(expected, actual);
            }
        }
    };
}

// ---------------------------------------------------------------------------
// IntBinaryArith (u64 specialized + unspecialized) — 12 widths × 5 kinds
// ---------------------------------------------------------------------------

prop_arith!(u8_add, u8, Add, u8::checked_add);
prop_arith!(u8_sub, u8, Sub, u8::checked_sub);
prop_arith!(u8_mul, u8, Mul, u8::checked_mul);
prop_arith!(u8_div, u8, Div, u8::checked_div);
prop_arith!(u8_mod, u8, Mod, u8::checked_rem);

prop_arith!(u16_add, u16, Add, u16::checked_add);
prop_arith!(u16_sub, u16, Sub, u16::checked_sub);
prop_arith!(u16_mul, u16, Mul, u16::checked_mul);
prop_arith!(u16_div, u16, Div, u16::checked_div);
prop_arith!(u16_mod, u16, Mod, u16::checked_rem);

prop_arith!(u32_add, u32, Add, u32::checked_add);
prop_arith!(u32_sub, u32, Sub, u32::checked_sub);
prop_arith!(u32_mul, u32, Mul, u32::checked_mul);
prop_arith!(u32_div, u32, Div, u32::checked_div);
prop_arith!(u32_mod, u32, Mod, u32::checked_rem);

prop_arith!(u64_add, u64, Add, u64::checked_add);
prop_arith!(u64_sub, u64, Sub, u64::checked_sub);
prop_arith!(u64_mul, u64, Mul, u64::checked_mul);
prop_arith!(u64_div, u64, Div, u64::checked_div);
prop_arith!(u64_mod, u64, Mod, u64::checked_rem);

prop_arith!(u64_add_specialized, u64, Add, u64::checked_add, specialized);
prop_arith!(u64_sub_specialized, u64, Sub, u64::checked_sub, specialized);
prop_arith!(u64_mul_specialized, u64, Mul, u64::checked_mul, specialized);
prop_arith!(u64_div_specialized, u64, Div, u64::checked_div, specialized);
prop_arith!(u64_mod_specialized, u64, Mod, u64::checked_rem, specialized);

prop_arith!(u128_add, u128, Add, u128::checked_add);
prop_arith!(u128_sub, u128, Sub, u128::checked_sub);
prop_arith!(u128_mul, u128, Mul, u128::checked_mul);
prop_arith!(u128_div, u128, Div, u128::checked_div);
prop_arith!(u128_mod, u128, Mod, u128::checked_rem);

prop_arith!(u256_add, U256, Add, U256::checked_add);
prop_arith!(u256_sub, U256, Sub, U256::checked_sub);
prop_arith!(u256_mul, U256, Mul, U256::checked_mul);
prop_arith!(u256_div, U256, Div, U256::checked_div);
prop_arith!(u256_mod, U256, Mod, U256::checked_rem);

prop_arith!(i8_add, i8, Add, i8::checked_add);
prop_arith!(i8_sub, i8, Sub, i8::checked_sub);
prop_arith!(i8_mul, i8, Mul, i8::checked_mul);
prop_arith!(i8_div, i8, Div, i8::checked_div);
prop_arith!(i8_mod, i8, Mod, i8::checked_rem);

prop_arith!(i16_add, i16, Add, i16::checked_add);
prop_arith!(i16_sub, i16, Sub, i16::checked_sub);
prop_arith!(i16_mul, i16, Mul, i16::checked_mul);
prop_arith!(i16_div, i16, Div, i16::checked_div);
prop_arith!(i16_mod, i16, Mod, i16::checked_rem);

prop_arith!(i32_add, i32, Add, i32::checked_add);
prop_arith!(i32_sub, i32, Sub, i32::checked_sub);
prop_arith!(i32_mul, i32, Mul, i32::checked_mul);
prop_arith!(i32_div, i32, Div, i32::checked_div);
prop_arith!(i32_mod, i32, Mod, i32::checked_rem);

prop_arith!(i64_add, i64, Add, i64::checked_add);
prop_arith!(i64_sub, i64, Sub, i64::checked_sub);
prop_arith!(i64_mul, i64, Mul, i64::checked_mul);
prop_arith!(i64_div, i64, Div, i64::checked_div);
prop_arith!(i64_mod, i64, Mod, i64::checked_rem);

prop_arith!(i128_add, i128, Add, i128::checked_add);
prop_arith!(i128_sub, i128, Sub, i128::checked_sub);
prop_arith!(i128_mul, i128, Mul, i128::checked_mul);
prop_arith!(i128_div, i128, Div, i128::checked_div);
prop_arith!(i128_mod, i128, Mod, i128::checked_rem);

prop_arith!(i256_add, I256, Add, I256::checked_add);
prop_arith!(i256_sub, I256, Sub, I256::checked_sub);
prop_arith!(i256_mul, I256, Mul, I256::checked_mul);
prop_arith!(i256_div, I256, Div, I256::checked_div);
prop_arith!(i256_mod, I256, Mod, I256::checked_rem);

// ---------------------------------------------------------------------------
// IntBitwise (u64 specialized + unspecialized) — 6 unsigned widths × 3 kinds
// ---------------------------------------------------------------------------

prop_bit!(u8_and, u8, BitAnd, |a: u8, b: u8| a & b);
prop_bit!(u8_or, u8, BitOr, |a: u8, b: u8| a | b);
prop_bit!(u8_xor, u8, BitXor, |a: u8, b: u8| a ^ b);

prop_bit!(u16_and, u16, BitAnd, |a: u16, b: u16| a & b);
prop_bit!(u16_or, u16, BitOr, |a: u16, b: u16| a | b);
prop_bit!(u16_xor, u16, BitXor, |a: u16, b: u16| a ^ b);

prop_bit!(u32_and, u32, BitAnd, |a: u32, b: u32| a & b);
prop_bit!(u32_or, u32, BitOr, |a: u32, b: u32| a | b);
prop_bit!(u32_xor, u32, BitXor, |a: u32, b: u32| a ^ b);

prop_bit!(u64_and, u64, BitAnd, |a: u64, b: u64| a & b);
prop_bit!(u64_or, u64, BitOr, |a: u64, b: u64| a | b);
prop_bit!(u64_xor, u64, BitXor, |a: u64, b: u64| a ^ b);

prop_bit!(
    u64_and_specialized,
    u64,
    BitAnd,
    |a: u64, b: u64| a & b,
    specialized
);
prop_bit!(
    u64_or_specialized,
    u64,
    BitOr,
    |a: u64, b: u64| a | b,
    specialized
);
prop_bit!(
    u64_xor_specialized,
    u64,
    BitXor,
    |a: u64, b: u64| a ^ b,
    specialized
);

prop_bit!(u128_and, u128, BitAnd, |a: u128, b: u128| a & b);
prop_bit!(u128_or, u128, BitOr, |a: u128, b: u128| a | b);
prop_bit!(u128_xor, u128, BitXor, |a: u128, b: u128| a ^ b);

prop_bit!(u256_and, U256, BitAnd, |a: U256, b: U256| a & b);
prop_bit!(u256_or, U256, BitOr, |a: U256, b: U256| a | b);
prop_bit!(u256_xor, U256, BitXor, |a: U256, b: U256| a ^ b);

// ---------------------------------------------------------------------------
// IntShift (u64 specialized + unspecialized) — 6 unsigned widths × 2 kinds
// ---------------------------------------------------------------------------
//
// Native widths use `T::checked_shl` / `T::checked_shr` (returns `None`
// when shift >= bit_width). U256's u8 shift is always < 256 so the
// reference never returns None; this property is primarily an integration
// test for the byte-conversion + dispatch plumbing.

prop_shift!(u8_shl, u8, Shl, |a: u8, s: u8| u8::checked_shl(a, s as u32));
prop_shift!(u8_shr, u8, Shr, |a: u8, s: u8| u8::checked_shr(a, s as u32));

prop_shift!(u16_shl, u16, Shl, |a: u16, s: u8| u16::checked_shl(
    a, s as u32
));
prop_shift!(u16_shr, u16, Shr, |a: u16, s: u8| u16::checked_shr(
    a, s as u32
));

prop_shift!(u32_shl, u32, Shl, |a: u32, s: u8| u32::checked_shl(
    a, s as u32
));
prop_shift!(u32_shr, u32, Shr, |a: u32, s: u8| u32::checked_shr(
    a, s as u32
));

prop_shift!(u64_shl, u64, Shl, |a: u64, s: u8| u64::checked_shl(
    a, s as u32
));
prop_shift!(u64_shr, u64, Shr, |a: u64, s: u8| u64::checked_shr(
    a, s as u32
));

prop_shift!(
    u64_shl_specialized,
    u64,
    Shl,
    |a: u64, s: u8| u64::checked_shl(a, s as u32),
    specialized
);
prop_shift!(
    u64_shr_specialized,
    u64,
    Shr,
    |a: u64, s: u8| u64::checked_shr(a, s as u32),
    specialized
);

prop_shift!(u128_shl, u128, Shl, |a: u128, s: u8| {
    u128::checked_shl(a, s as u32)
});
prop_shift!(u128_shr, u128, Shr, |a: u128, s: u8| {
    u128::checked_shr(a, s as u32)
});

prop_shift!(u256_shl, U256, Shl, |a: U256, s: u8| Some(
    a << u256_from_u8(s)
));
prop_shift!(u256_shr, U256, Shr, |a: U256, s: u8| Some(
    a >> u256_from_u8(s)
));

// ---------------------------------------------------------------------------
// Arithmetic imm — u64 specialized + 11 unspecialized widths × 5 kinds
// ---------------------------------------------------------------------------

prop_arith_imm!(u8_add_imm, u8, Add, strat!(u8), u8::checked_add);
prop_arith_imm!(u8_sub_imm, u8, Sub, strat!(u8), u8::checked_sub);
prop_arith_imm!(u8_mul_imm, u8, Mul, strat!(u8), u8::checked_mul);
prop_arith_imm!(u8_div_imm, u8, Div, nonzero!(u8), u8::checked_div);
prop_arith_imm!(u8_mod_imm, u8, Mod, nonzero!(u8), u8::checked_rem);

prop_arith_imm!(u16_add_imm, u16, Add, strat!(u16), u16::checked_add);
prop_arith_imm!(u16_sub_imm, u16, Sub, strat!(u16), u16::checked_sub);
prop_arith_imm!(u16_mul_imm, u16, Mul, strat!(u16), u16::checked_mul);
prop_arith_imm!(u16_div_imm, u16, Div, nonzero!(u16), u16::checked_div);
prop_arith_imm!(u16_mod_imm, u16, Mod, nonzero!(u16), u16::checked_rem);

prop_arith_imm!(u32_add_imm, u32, Add, strat!(u32), u32::checked_add);
prop_arith_imm!(u32_sub_imm, u32, Sub, strat!(u32), u32::checked_sub);
prop_arith_imm!(u32_mul_imm, u32, Mul, strat!(u32), u32::checked_mul);
prop_arith_imm!(u32_div_imm, u32, Div, nonzero!(u32), u32::checked_div);
prop_arith_imm!(u32_mod_imm, u32, Mod, nonzero!(u32), u32::checked_rem);

prop_arith_imm!(u64_add_imm, u64, Add, any::<u64>(), u64::checked_add);
prop_arith_imm!(u64_sub_imm, u64, Sub, any::<u64>(), u64::checked_sub);
prop_arith_imm!(u64_mul_imm, u64, Mul, any::<u64>(), u64::checked_mul);
prop_arith_imm!(u64_div_imm, u64, Div, 1u64.., u64::checked_div);
prop_arith_imm!(u64_mod_imm, u64, Mod, 1u64.., u64::checked_rem);

prop_arith_imm!(
    u64_add_imm_specialized,
    u64,
    Add,
    any::<u64>(),
    u64::checked_add,
    specialized
);
prop_arith_imm!(
    u64_sub_imm_specialized,
    u64,
    Sub,
    any::<u64>(),
    u64::checked_sub,
    specialized
);
prop_arith_imm!(
    u64_mul_imm_specialized,
    u64,
    Mul,
    any::<u64>(),
    u64::checked_mul,
    specialized
);
prop_arith_imm!(
    u64_div_imm_specialized,
    u64,
    Div,
    1u64..,
    u64::checked_div,
    specialized
);
prop_arith_imm!(
    u64_mod_imm_specialized,
    u64,
    Mod,
    1u64..,
    u64::checked_rem,
    specialized
);

prop_arith_imm!(u128_add_imm, u128, Add, strat!(u128), u128::checked_add);
prop_arith_imm!(u128_sub_imm, u128, Sub, strat!(u128), u128::checked_sub);
prop_arith_imm!(u128_mul_imm, u128, Mul, strat!(u128), u128::checked_mul);
prop_arith_imm!(u128_div_imm, u128, Div, nonzero!(u128), u128::checked_div);
prop_arith_imm!(u128_mod_imm, u128, Mod, nonzero!(u128), u128::checked_rem);

prop_arith_imm!(u256_add_imm, U256, Add, u256_strategy(), U256::checked_add);
prop_arith_imm!(u256_sub_imm, U256, Sub, u256_strategy(), U256::checked_sub);
prop_arith_imm!(u256_mul_imm, U256, Mul, u256_strategy(), U256::checked_mul);
prop_arith_imm!(u256_div_imm, U256, Div, nonzero!(U256), U256::checked_div);
prop_arith_imm!(u256_mod_imm, U256, Mod, nonzero!(U256), U256::checked_rem);

prop_arith_imm!(i8_add_imm, i8, Add, strat!(i8), i8::checked_add);
prop_arith_imm!(i8_sub_imm, i8, Sub, strat!(i8), i8::checked_sub);
prop_arith_imm!(i8_mul_imm, i8, Mul, strat!(i8), i8::checked_mul);
prop_arith_imm!(i8_div_imm, i8, Div, nonzero!(i8), i8::checked_div);
prop_arith_imm!(i8_mod_imm, i8, Mod, nonzero!(i8), i8::checked_rem);

prop_arith_imm!(i16_add_imm, i16, Add, strat!(i16), i16::checked_add);
prop_arith_imm!(i16_sub_imm, i16, Sub, strat!(i16), i16::checked_sub);
prop_arith_imm!(i16_mul_imm, i16, Mul, strat!(i16), i16::checked_mul);
prop_arith_imm!(i16_div_imm, i16, Div, nonzero!(i16), i16::checked_div);
prop_arith_imm!(i16_mod_imm, i16, Mod, nonzero!(i16), i16::checked_rem);

prop_arith_imm!(i32_add_imm, i32, Add, strat!(i32), i32::checked_add);
prop_arith_imm!(i32_sub_imm, i32, Sub, strat!(i32), i32::checked_sub);
prop_arith_imm!(i32_mul_imm, i32, Mul, strat!(i32), i32::checked_mul);
prop_arith_imm!(i32_div_imm, i32, Div, nonzero!(i32), i32::checked_div);
prop_arith_imm!(i32_mod_imm, i32, Mod, nonzero!(i32), i32::checked_rem);

prop_arith_imm!(i64_add_imm, i64, Add, strat!(i64), i64::checked_add);
prop_arith_imm!(i64_sub_imm, i64, Sub, strat!(i64), i64::checked_sub);
prop_arith_imm!(i64_mul_imm, i64, Mul, strat!(i64), i64::checked_mul);
prop_arith_imm!(i64_div_imm, i64, Div, nonzero!(i64), i64::checked_div);
prop_arith_imm!(i64_mod_imm, i64, Mod, nonzero!(i64), i64::checked_rem);

prop_arith_imm!(i128_add_imm, i128, Add, strat!(i128), i128::checked_add);
prop_arith_imm!(i128_sub_imm, i128, Sub, strat!(i128), i128::checked_sub);
prop_arith_imm!(i128_mul_imm, i128, Mul, strat!(i128), i128::checked_mul);
prop_arith_imm!(i128_div_imm, i128, Div, nonzero!(i128), i128::checked_div);
prop_arith_imm!(i128_mod_imm, i128, Mod, nonzero!(i128), i128::checked_rem);

prop_arith_imm!(i256_add_imm, I256, Add, i256_strategy(), I256::checked_add);
prop_arith_imm!(i256_sub_imm, I256, Sub, i256_strategy(), I256::checked_sub);
prop_arith_imm!(i256_mul_imm, I256, Mul, i256_strategy(), I256::checked_mul);
prop_arith_imm!(i256_div_imm, I256, Div, nonzero!(I256), I256::checked_div);
prop_arith_imm!(i256_mod_imm, I256, Mod, nonzero!(I256), I256::checked_rem);

// `RSubU64Imm` (`dst = imm - src`) is a u64-only fast path with no
// unspecialized counterpart. Tested separately via the original
// variant-name dispatch macro.
prop_imm!(rsub_u64_imm, RSubU64Imm, any::<u64>(), |s: u64, i: u64| {
    u64::checked_sub(i, s)
});

// ---------------------------------------------------------------------------
// Bitwise imm — 5 unsigned unspecialized widths × 3 kinds
// ---------------------------------------------------------------------------
// (No u64 specialized bitwise imm variants exist.)

prop_bit_imm!(u8_and_imm, u8, BitAnd, |a: u8, b: u8| a & b);
prop_bit_imm!(u8_or_imm, u8, BitOr, |a: u8, b: u8| a | b);
prop_bit_imm!(u8_xor_imm, u8, BitXor, |a: u8, b: u8| a ^ b);

prop_bit_imm!(u16_and_imm, u16, BitAnd, |a: u16, b: u16| a & b);
prop_bit_imm!(u16_or_imm, u16, BitOr, |a: u16, b: u16| a | b);
prop_bit_imm!(u16_xor_imm, u16, BitXor, |a: u16, b: u16| a ^ b);

prop_bit_imm!(u32_and_imm, u32, BitAnd, |a: u32, b: u32| a & b);
prop_bit_imm!(u32_or_imm, u32, BitOr, |a: u32, b: u32| a | b);
prop_bit_imm!(u32_xor_imm, u32, BitXor, |a: u32, b: u32| a ^ b);

prop_bit_imm!(u128_and_imm, u128, BitAnd, |a: u128, b: u128| a & b);
prop_bit_imm!(u128_or_imm, u128, BitOr, |a: u128, b: u128| a | b);
prop_bit_imm!(u128_xor_imm, u128, BitXor, |a: u128, b: u128| a ^ b);

prop_bit_imm!(u256_and_imm, U256, BitAnd, |a: U256, b: U256| a & b);
prop_bit_imm!(u256_or_imm, U256, BitOr, |a: U256, b: U256| a | b);
prop_bit_imm!(u256_xor_imm, U256, BitXor, |a: U256, b: U256| a ^ b);

// ---------------------------------------------------------------------------
// Shift imm — u64 specialized + 5 unsigned unspecialized widths × 2 kinds
// ---------------------------------------------------------------------------

prop_shift_imm!(u8_shl_imm, u8, Shl, 0u8..8, |a: u8, s: u8| {
    u8::checked_shl(a, s as u32)
});
prop_shift_imm!(u8_shr_imm, u8, Shr, 0u8..8, |a: u8, s: u8| {
    u8::checked_shr(a, s as u32)
});

prop_shift_imm!(u16_shl_imm, u16, Shl, 0u8..16, |a: u16, s: u8| {
    u16::checked_shl(a, s as u32)
});
prop_shift_imm!(u16_shr_imm, u16, Shr, 0u8..16, |a: u16, s: u8| {
    u16::checked_shr(a, s as u32)
});

prop_shift_imm!(u32_shl_imm, u32, Shl, 0u8..32, |a: u32, s: u8| {
    u32::checked_shl(a, s as u32)
});
prop_shift_imm!(u32_shr_imm, u32, Shr, 0u8..32, |a: u32, s: u8| {
    u32::checked_shr(a, s as u32)
});

prop_shift_imm!(u64_shl_imm, u64, Shl, 0u8..64, |a: u64, s: u8| {
    u64::checked_shl(a, s as u32)
});
prop_shift_imm!(u64_shr_imm, u64, Shr, 0u8..64, |a: u64, s: u8| {
    u64::checked_shr(a, s as u32)
});

prop_shift_imm!(
    u64_shl_imm_specialized,
    u64,
    Shl,
    0u8..64,
    |a: u64, s: u8| { u64::checked_shl(a, s as u32) },
    specialized
);
prop_shift_imm!(
    u64_shr_imm_specialized,
    u64,
    Shr,
    0u8..64,
    |a: u64, s: u8| { u64::checked_shr(a, s as u32) },
    specialized
);

prop_shift_imm!(u128_shl_imm, u128, Shl, 0u8..128, |a: u128, s: u8| {
    u128::checked_shl(a, s as u32)
});
prop_shift_imm!(u128_shr_imm, u128, Shr, 0u8..128, |a: u128, s: u8| {
    u128::checked_shr(a, s as u32)
});

prop_shift_imm!(u256_shl_imm, U256, Shl, any::<u8>(), |a: U256, s: u8| Some(
    a << u256_from_u8(s)
));
prop_shift_imm!(u256_shr_imm, U256, Shr, any::<u8>(), |a: U256, s: u8| Some(
    a >> u256_from_u8(s)
));

// ---------------------------------------------------------------------------
// IntNegate (signed unspecialized) — 6 widths × Negate
// ---------------------------------------------------------------------------
//
// Reference is `T::checked_neg`, which returns `None` exactly when
// `src == T::MIN`. The proptest sampler hits MIN often enough through
// shrinking that we don't need to special-case it.

prop_negate!(i8_negate, i8, i8::checked_neg);
prop_negate!(i16_negate, i16, i16::checked_neg);
prop_negate!(i32_negate, i32, i32::checked_neg);
prop_negate!(i64_negate, i64, i64::checked_neg);
prop_negate!(i128_negate, i128, i128::checked_neg);
prop_negate!(i256_negate, I256, I256::checked_neg);

// ---------------------------------------------------------------------------
// IntCast — all 12×12 (from, to) pairs
// ---------------------------------------------------------------------------

/// Helper trait to define various properties/operations for all integer types.
/// BigInt is used as a convenience as it can represent integers of any size and be
/// used for range checks.
trait CastType: Copy + std::fmt::Debug {
    const TAG: IntTy;
    const WIDTH: usize;
    fn to_slot_bytes(self) -> Vec<u8>;
    fn to_bigint(self) -> BigInt;
    fn range_bigint() -> (BigInt, BigInt);
    fn decode(bytes: &[u8]) -> String;
    fn strategy() -> BoxedStrategy<Self>;
}

macro_rules! impl_cast_type_native {
    ($ty:ty, $tag:expr) => {
        impl CastType for $ty {
            const TAG: IntTy = $tag;
            const WIDTH: usize = std::mem::size_of::<$ty>();

            fn to_slot_bytes(self) -> Vec<u8> {
                self.to_ne_bytes().to_vec()
            }

            fn to_bigint(self) -> BigInt {
                BigInt::from(self)
            }

            fn range_bigint() -> (BigInt, BigInt) {
                (BigInt::from(<$ty>::MIN), BigInt::from(<$ty>::MAX))
            }

            fn decode(bytes: &[u8]) -> String {
                <$ty>::from_ne_bytes(bytes.try_into().unwrap()).to_string()
            }

            fn strategy() -> BoxedStrategy<Self> {
                any::<$ty>().boxed()
            }
        }
    };
}

impl_cast_type_native!(u8, IntTy::U8);
impl_cast_type_native!(u16, IntTy::U16);
impl_cast_type_native!(u32, IntTy::U32);
impl_cast_type_native!(u64, IntTy::U64);
impl_cast_type_native!(u128, IntTy::U128);
impl_cast_type_native!(i8, IntTy::I8);
impl_cast_type_native!(i16, IntTy::I16);
impl_cast_type_native!(i32, IntTy::I32);
impl_cast_type_native!(i64, IntTy::I64);
impl_cast_type_native!(i128, IntTy::I128);

macro_rules! impl_cast_type_wide {
    ($ty:ty, $tag:expr, $bytes_to_big:expr) => {
        impl CastType for $ty {
            const TAG: IntTy = $tag;
            const WIDTH: usize = 32;

            fn to_slot_bytes(self) -> Vec<u8> {
                self.to_le_bytes().to_vec()
            }

            fn to_bigint(self) -> BigInt {
                $bytes_to_big(&self.to_le_bytes())
            }

            fn range_bigint() -> (BigInt, BigInt) {
                (
                    $bytes_to_big(&<$ty>::MIN.to_le_bytes()),
                    $bytes_to_big(&<$ty>::MAX.to_le_bytes()),
                )
            }

            fn decode(bytes: &[u8]) -> String {
                <$ty>::from_le_bytes(bytes.try_into().unwrap()).to_string()
            }

            fn strategy() -> BoxedStrategy<Self> {
                any::<[u8; 32]>().prop_map(<$ty>::from_le_bytes).boxed()
            }
        }
    };
}

impl_cast_type_wide!(U256, IntTy::U256, |b: &[u8]| BigInt::from_bytes_le(
    Sign::Plus,
    b
));
impl_cast_type_wide!(I256, IntTy::I256, |b: &[u8]| {
    BigInt::from_signed_bytes_le(b)
});

/// Run the cast micro-op (from `S` to `D`) using the VM runtime.
fn cast_runtime<S: CastType, D: CastType>(v: S) -> Option<String> {
    let op = MicroOp::IntCast(IntCastOp {
        from: S::TAG,
        to: D::TAG,
        dst: FO(SLOT_DST),
        src: FO(SLOT_LHS),
    });
    run_wide(op, &v.to_slot_bytes(), &[], D::WIDTH)
        .ok()
        .map(|bytes| D::decode(&bytes))
}

/// Reference impl of a cast (from `S` to `D`).
fn cast_reference_impl<S: CastType, D: CastType>(v: S) -> Option<String> {
    let value = v.to_bigint();
    let (lo, hi) = D::range_bigint();
    (value >= lo && value <= hi).then(|| value.to_string())
}

/// Defines one proptest for the (src, dst) cast pair.
macro_rules! cast_case {
    ($src:ident, $dst:ident) => {
        paste::paste! {
            proptest! {
                #[test]
                fn [<cast_from_ $src:lower _to_ $dst:lower>](
                    v in <$src as CastType>::strategy()
                ) {
                    prop_assert_eq!(
                        cast_runtime::<$src, $dst>(v),
                        cast_reference_impl::<$src, $dst>(v)
                    );
                }
            }
        }
    };
}

/// Generate a [`cast_case!`] for the full cross product of a type list --
/// every type cast into every type, including itself.
macro_rules! cast_matrix {
    ($($ty:ident),+ $(,)?) => {
        cast_matrix!(@outer [$($ty),+] [$($ty),+]);
    };
    (@outer [$($src:ident),+] $dsts:tt) => {
        $( cast_matrix!(@inner $src $dsts); )+
    };
    (@inner $src:ident [$($dst:ident),+]) => {
        $( cast_case!($src, $dst); )+
    };
}

// Generates all 144 (12 × 12) cast combinations, one test per pair.
cast_matrix!(u8, u16, u32, u64, u128, U256, i8, i16, i32, i64, i128, I256);
