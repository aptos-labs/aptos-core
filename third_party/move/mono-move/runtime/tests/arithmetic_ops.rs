// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for u64 arithmetic / bitwise / shift micro-ops.
//!
//! Each test builds a one-instruction "function" (op + Return), seeds the
//! input slots via `set_root_arg`, runs to completion, and checks the
//! result. Abort-path tests assert that `run()` returns an error.
//!
//! TODO: revisit which of these belong here vs. in the differential-test
//! harness (`mono-move-testsuite/.../snapshot/masm/arithmetic.masm`),
//! once we have a clearer story on running the same `.masm` programs
//! against the old VM for cross-validation. Some of these are "micro-op
//! unit tests" that don't need the specializer pipeline and probably
//! belong here regardless; others (the happy-path arithmetic checks)
//! could move. Check with @vineethk before migrating.

use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    FrameLayoutInfo, FrameOffset as FO, Function, LocalExecutionContext, MicroOp,
    SortedSafePointEntries, FRAME_METADATA_SIZE,
};
use mono_move_runtime::{InterpreterContext, ObjectDescriptor};

// ---------------------------------------------------------------------------
// Frame layout used by every test in this file
// ---------------------------------------------------------------------------
//   [SLOT_DST] = result (output)
//   [SLOT_LHS] = first input (lhs for binary ops; src for imm/unary ops)
//   [SLOT_RHS] = second input (rhs for binary ops; unused for unary)

const SLOT_DST: u32 = 0;
const SLOT_LHS: u32 = 8;
const SLOT_RHS: u32 = 16;
/// Imm ops read their input from the same slot as a binary op's lhs.
const SLOT_SRC: u32 = SLOT_LHS;
const FRAME_SIZE: u32 = 24;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_func(arena: &ExecutableArena, op: MicroOp) -> ExecutableArenaPtr<Function> {
    arena.alloc(Function {
        name: GlobalArenaPtr::from_static("op"),
        code: arena.alloc_slice_fill_iter([op, MicroOp::Return]),
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: FRAME_SIZE as usize,
        extended_frame_size: FRAME_SIZE as usize + FRAME_METADATA_SIZE,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })
}

/// Run a reg-reg binary u64 op: writes `lhs` and `rhs` into the input
/// slots, runs the op + Return, returns the value at `SLOT_DST` (or err).
fn run_binary_u64_op(op: MicroOp, lhs: u64, rhs: u64) -> Result<u64, anyhow::Error> {
    let arena = ExecutableArena::new();
    let func = make_func(&arena, op);
    let descriptors: Vec<ObjectDescriptor> = vec![];
    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        func.as_ref_unchecked()
    });
    ctx.set_root_arg(SLOT_LHS, &lhs.to_ne_bytes());
    ctx.set_root_arg(SLOT_RHS, &rhs.to_ne_bytes());
    ctx.run()?;
    Ok(ctx.root_result())
}

/// Run an immediate-form u64 op (one runtime input + a baked-in imm):
/// writes `src` into `SLOT_SRC`, runs the op + Return, returns the value
/// at `SLOT_DST` (or err).
fn run_unary_u64_op(op: MicroOp, src: u64) -> Result<u64, anyhow::Error> {
    let arena = ExecutableArena::new();
    let func = make_func(&arena, op);
    let descriptors: Vec<ObjectDescriptor> = vec![];
    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        func.as_ref_unchecked()
    });
    ctx.set_root_arg(SLOT_SRC, &src.to_ne_bytes());
    ctx.run()?;
    Ok(ctx.root_result())
}

// ---------------------------------------------------------------------------
// Macros
// ---------------------------------------------------------------------------
//
// `binop!` / `imm_op!` build a `MicroOp` value with dst/lhs/rhs/src wired
// to the shared frame slots. `binary_ok!`/`binary_err!` and `imm_ok!`/
// `imm_err!` build the op, run it, and assert.
//
// Argument order is positional: variant first, then operands left-to-right
// in the same order they appear in the op's struct fields.

/// Build a reg-reg binary u64 op (dst/lhs/rhs ← shared frame slots).
macro_rules! binop {
    ($variant:ident) => {
        MicroOp::$variant {
            dst: FO(SLOT_DST),
            lhs: FO(SLOT_LHS),
            rhs: FO(SLOT_RHS),
        }
    };
}

/// Build an immediate-form u64 op (dst/src ← shared frame slots).
macro_rules! imm_op {
    ($variant:ident, $imm:expr) => {
        MicroOp::$variant {
            dst: FO(SLOT_DST),
            src: FO(SLOT_SRC),
            imm: $imm,
        }
    };
}

/// `binary_ok!(AddU64, 7, 35 => 42)` → `(7 + 35) == 42`.
macro_rules! binary_ok {
    ($variant:ident, $lhs:expr, $rhs:expr => $expected:expr) => {
        assert_eq!(
            run_binary_u64_op(binop!($variant), $lhs, $rhs).unwrap(),
            $expected,
        );
    };
}

/// `binary_err!(AddU64, u64::MAX, 1)` → expects abort.
macro_rules! binary_err {
    ($variant:ident, $lhs:expr, $rhs:expr) => {
        assert!(run_binary_u64_op(binop!($variant), $lhs, $rhs).is_err());
    };
}

/// `imm_ok!(SubU64Imm, 8, 50 => 42)` → `(50 - 8) == 42`. Args are
/// `variant, imm, src => expected`.
macro_rules! imm_ok {
    ($variant:ident, $imm:expr, $src:expr => $expected:expr) => {
        assert_eq!(
            run_unary_u64_op(imm_op!($variant, $imm), $src).unwrap(),
            $expected,
        );
    };
}

/// `imm_err!(SubU64Imm, 8, 0)` → expects abort. Args are
/// `variant, imm, src`.
macro_rules! imm_err {
    ($variant:ident, $imm:expr, $src:expr) => {
        assert!(run_unary_u64_op(imm_op!($variant, $imm), $src).is_err());
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// --- Add / Sub ---

#[test]
fn add_u64() {
    binary_ok!(AddU64, 7, 35 => 42);
    binary_err!(AddU64, u64::MAX, 1); // overflow
}

#[test]
fn sub_u64() {
    binary_ok!(SubU64, 50, 8 => 42);
    binary_err!(SubU64, 0, 1); // underflow
}

#[test]
fn sub_u64_imm() {
    imm_ok!(SubU64Imm, 8, 50 => 42);
    imm_err!(SubU64Imm, 8, 0); // underflow
}

#[test]
fn rsub_u64_imm() {
    imm_ok!(RSubU64Imm, 50, 8 => 42); // 50 - 8
    imm_err!(RSubU64Imm, 50, 51); // underflow
}

// --- Mul / Div / Mod ---

#[test]
fn mul_u64() {
    binary_ok!(MulU64, 6, 7 => 42);
    binary_err!(MulU64, u64::MAX, 2); // overflow
}

#[test]
fn mul_u64_imm() {
    imm_ok!(MulU64Imm, 7, 6 => 42);
    imm_err!(MulU64Imm, 2, u64::MAX); // overflow
}

#[test]
fn div_u64() {
    binary_ok!(DivU64, 84, 2 => 42);
    binary_err!(DivU64, 1, 0); // div by zero
}

#[test]
fn div_u64_imm() {
    imm_ok!(DivU64Imm, 2, 84 => 42);
    // Note: imm == 0 is rejected statically by the verifier; see
    // verifier_test::div_u64_imm_zero.
}

#[test]
fn mod_u64() {
    binary_ok!(ModU64, 100, 7 => 2);
    binary_err!(ModU64, 1, 0); // div by zero
}

#[test]
fn mod_u64_imm() {
    imm_ok!(ModU64Imm, 7, 100 => 2);
    // Note: imm == 0 is rejected statically by the verifier; see
    // verifier_test::mod_u64_imm_zero.
}

// --- Bitwise ---

#[test]
fn bit_and_u64() {
    binary_ok!(BitAndU64, 0xFF00, 0x0FF0 => 0x0F00);
}

#[test]
fn bit_or_u64() {
    binary_ok!(BitOrU64, 0xFF00, 0x0FF0 => 0xFFF0);
}

#[test]
fn bit_xor_u64() {
    binary_ok!(BitXorU64, 0xFF00, 0x0FF0 => 0xF0F0);
}

// --- Shifts ---

#[test]
fn shl_u64() {
    binary_ok!(ShlU64, 1, 4 => 16);
    binary_ok!(ShlU64, 1, 63 => 1u64 << 63);
    binary_err!(ShlU64, 1, 64); // shift >= 64
}

#[test]
fn shl_u64_imm() {
    imm_ok!(ShlU64Imm, 4, 1 => 16);
    // Note: imm >= 64 is rejected statically by the verifier; see
    // verifier_test::shl_u64_imm_oversize.
}

#[test]
fn shr_u64() {
    binary_ok!(ShrU64, 256, 4 => 16);
    binary_ok!(ShrU64, u64::MAX, 63 => 1);
    binary_err!(ShrU64, 1, 64); // shift >= 64
}

#[test]
fn shr_u64_imm() {
    imm_ok!(ShrU64Imm, 4, 256 => 16);
    // Note: imm >= 64 is rejected statically by the verifier; see
    // verifier_test::shr_u64_imm_oversize.
}
