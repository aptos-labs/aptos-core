// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tight loop of integer arithmetic ops — a microbenchmark for comparing
//! the runtime's per-op cost between the u64 specialized fast path
//! ([`MicroOp::MulU64Imm`] etc.) and the unspecialized per-kind
//! encoding ([`MicroOp::IntMul`] etc. carrying an
//! [`IntOperand::ImmI64`] rhs).
//!
//! Each loop iteration runs [`ROUNDS_PER_ITER`] rounds of
//! `acc = ((acc * MUL) + ADD) % MOD` — three checked imm ops per round —
//! so 90 body ops vs ~2 loop-overhead ops per iteration. The constants
//! keep `acc` bounded to `[0, MOD)`, so checked arithmetic never aborts.
//!
//! The loop variable itself is a `u64` counter decremented via
//! [`MicroOp::SubU64Imm`] in both flavors. Only the body type changes,
//! isolating the dispatcher-cost difference.
//!
//! [`MicroOp`]: mono_move_core::MicroOp
//! [`MicroOp::MulU64Imm`]: mono_move_core::MicroOp::MulU64Imm
//! [`MicroOp::IntMul`]: mono_move_core::MicroOp::IntMul
//! [`IntOperand::ImmI64`]: mono_move_core::IntOperand::ImmI64

/// Rounds per loop iteration. 30 rounds × 3 ops = 90 body ops per iter,
/// vs. ~2 loop-overhead ops — body dominates ~98%.
pub const ROUNDS_PER_ITER: usize = 30;

/// Per-round arithmetic constants. `((acc * MUL) + ADD) % MOD`. Picked
/// so that `acc * MUL + ADD < u64::MAX` for any `acc < MOD`, and so the
/// signed `i64` flavor sees no sign-extension surprises.
pub const MUL: i64 = 31;
pub const ADD: i64 = 17;
pub const MOD: i64 = 1_000_003; // prime, fits well below 2^20

// Test cases: a single small iters value is enough — the loop is
// deterministic, so identical results across (native, micro_op_u64,
// micro_op_i64) is the actual correctness check.
pub const TEST_ITERS: u64 = 100;

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_u64_loop(iters: u64) -> u64 {
    let mut acc: u64 = 1;
    let mut i: u64 = 0;
    while i < iters {
        for _ in 0..ROUNDS_PER_ITER {
            acc = ((acc * MUL as u64) + ADD as u64) % MOD as u64;
        }
        i += 1;
    }
    acc
}

pub fn native_i64_loop(iters: u64) -> i64 {
    let mut acc: i64 = 1;
    let mut i: u64 = 0;
    while i < iters {
        for _ in 0..ROUNDS_PER_ITER {
            acc = ((acc * MUL) + ADD) % MOD;
        }
        i += 1;
    }
    acc
}

// ---------------------------------------------------------------------------
// Micro-op
// ---------------------------------------------------------------------------

/// Frame layout (both u64 and i64 flavors):
///   [0]  iters (arg) / result  (8 bytes)
///   [8]  acc                    (8 bytes — u64 or i64)
///   [16] counter                (8 bytes, u64 — loop variable)
///   [24] metadata               (24 bytes — saved pc / fp / func_ptr)
///
/// Code shape:
///   counter = iters
///   acc = 1
///   LOOP_TOP:
///     body (ROUNDS_PER_ITER × 3 imm ops)
///     counter -= 1
///     if counter != 0 → LOOP_TOP
///   result = acc
///   return
///
/// Caller must pass `iters > 0`; the no-prologue-check loop runs the
/// body once unconditionally and would underflow for iters == 0. Tests
/// and the bench always satisfy this precondition.
#[cfg(feature = "micro-op")]
mod micro_op {
    use super::{ADD, MOD, MUL, ROUNDS_PER_ITER};
    use mono_move_alloc::GlobalArenaPtr;
    use mono_move_core::{
        Code, CodeOffset as CO, FrameLayoutInfo, FrameOffset as FO, Function, FunctionPtr,
        IntBinaryOp, IntOperand,
        MicroOp::{self, *},
        SortedSafePointEntries, FRAME_METADATA_SIZE,
    };
    use mono_move_runtime::ObjectDescriptorTable;

    const ITERS: u32 = 0;
    const RESULT: u32 = ITERS;
    const ACC: u32 = 8;
    const COUNTER: u32 = 16;
    const PARAM_AND_LOCAL_SIZES_SUM: u32 = 24;

    /// Build the program. `body_round` emits one round of `(mul, add, mod)`
    /// imm ops on the `acc` slot — caller picks whether they're the u64
    /// specialized variants or the unspecialized `IntBinaryImm` form.
    fn build(
        name: &'static str,
        mut body_round: impl FnMut(&mut Vec<MicroOp>),
    ) -> (Vec<FunctionPtr>, ObjectDescriptorTable) {
        let mut code: Vec<MicroOp> = Vec::with_capacity(8 + ROUNDS_PER_ITER * 3);

        // counter = iters
        code.push(Move8 {
            dst: FO(COUNTER),
            src: FO(ITERS),
        });
        // acc = 1
        code.push(StoreImm8 {
            dst: FO(ACC),
            imm: 1,
        });

        let loop_top = code.len() as u32;

        // body: ROUNDS_PER_ITER rounds × 3 imm ops
        for _ in 0..ROUNDS_PER_ITER {
            body_round(&mut code);
        }

        // counter -= 1; if counter != 0 → LOOP_TOP
        code.push(SubU64Imm {
            dst: FO(COUNTER),
            src: FO(COUNTER),
            imm: 1,
        });
        code.push(JumpNotZeroU64 {
            target: CO(loop_top),
            src: FO(COUNTER),
        });

        // result = acc; return
        code.push(Move8 {
            dst: FO(RESULT),
            src: FO(ACC),
        });
        code.push(Return);

        let func_ptr = FunctionPtr::new(Box::new(Function {
            name: GlobalArenaPtr::from_static(name),
            code: Code::from_vec(code),
            param_sizes: vec![],
            param_sizes_sum: 8,
            param_and_local_sizes_sum: PARAM_AND_LOCAL_SIZES_SUM as usize,
            extended_frame_size: (PARAM_AND_LOCAL_SIZES_SUM + FRAME_METADATA_SIZE as u32) as usize,
            zero_frame: false,
            frame_layout: FrameLayoutInfo::empty(),
            safe_point_layouts: SortedSafePointEntries::empty(),
        }));

        (vec![func_ptr], ObjectDescriptorTable::new())
    }

    /// u64 flavor — each round emits the specialized fast-path variants.
    pub fn program_u64() -> (Vec<FunctionPtr>, ObjectDescriptorTable) {
        build("int_arith_loop_u64", |code| {
            code.push(MulU64Imm {
                dst: FO(ACC),
                src: FO(ACC),
                imm: MUL as u64,
            });
            code.push(AddU64Imm {
                dst: FO(ACC),
                src: FO(ACC),
                imm: ADD as u64,
            });
            code.push(ModU64Imm {
                dst: FO(ACC),
                src: FO(ACC),
                imm: MOD as u64,
            });
        })
    }

    /// i64 flavor — each round emits the unspecialized per-kind form. Body
    /// op count and frame layout are identical to the u64 flavor.
    pub fn program_i64() -> (Vec<FunctionPtr>, ObjectDescriptorTable) {
        build("int_arith_loop_i64", |code| {
            code.push(IntMul(IntBinaryOp {
                dst: FO(ACC),
                lhs: FO(ACC),
                rhs: IntOperand::ImmI64(MUL),
            }));
            code.push(IntAdd(IntBinaryOp {
                dst: FO(ACC),
                lhs: FO(ACC),
                rhs: IntOperand::ImmI64(ADD),
            }));
            code.push(IntMod(IntBinaryOp {
                dst: FO(ACC),
                lhs: FO(ACC),
                rhs: IntOperand::ImmI64(MOD),
            }));
        })
    }
}

#[cfg(feature = "micro-op")]
pub use micro_op::{program_i64 as micro_op_i64_loop, program_u64 as micro_op_u64_loop};

// ---------------------------------------------------------------------------
// Move bytecode
// ---------------------------------------------------------------------------

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use super::{ADD, MOD, MUL, ROUNDS_PER_ITER};
    use move_binary_format::file_format::CompiledModule;

    /// Generate the Move source for the loop. The body lines (one per
    /// round) are identical between the u64 and i64 functions —
    /// integer-literal type inference picks the right element type from
    /// `acc`'s declared type, so we reuse the same generated body.
    fn source() -> String {
        let mut rounds = String::new();
        for _ in 0..ROUNDS_PER_ITER {
            rounds.push_str(&format!(
                "            acc = ((acc * {MUL}) + {ADD}) % {MOD};\n"
            ));
        }
        format!(
            "module 0x1::int_arith_loop {{
    public fun u64_loop(iters: u64): u64 {{
        let acc: u64 = 1;
        let i: u64 = 0;
        while (i < iters) {{
{rounds}            i = i + 1;
        }};
        acc
    }}

    public fun i64_loop(iters: u64): i64 {{
        let acc: i64 = 1;
        let i: u64 = 0;
        while (i < iters) {{
{rounds}            i = i + 1;
        }};
        acc
    }}
}}"
        )
    }

    /// Compile the generated Move source into a [`CompiledModule`].
    pub fn program() -> CompiledModule {
        crate::compile_move_source(&source())
    }
}

#[cfg(feature = "move-bytecode")]
pub use move_bytecode::program as move_bytecode_int_arith_loop;
