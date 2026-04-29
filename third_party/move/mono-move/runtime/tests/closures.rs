// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for `PackClosure` and `CallClosure` micro-ops.
//!
//! Each test builds a small "main" function that packs a closure wrapping
//! `identity` or `add_u64`, calls it, and checks the result. The first four
//! tests cover the core mask-interleaving logic: no captures, one capture
//! at position 0, one capture at position 1, and all captures. The fifth
//! exercises a nested call chain (main → vector_map → CallClosure → add_u64)
//! where the closure is `|x| x + y` with `y` captured — applied in a loop
//! to each vector element.

use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    CallClosureOp, ClosureFuncRef, CodeOffset as CO, DescriptorId, FrameLayoutInfo,
    FrameOffset as FO, Function, MicroOp, NoopTransactionContext, PackClosureOp, SizedSlot,
    SortedSafePointEntries, FRAME_METADATA_SIZE,
};
use mono_move_gas::SimpleGasMeter;
use mono_move_runtime::{InterpreterContext, ObjectDescriptor};

// ---------------------------------------------------------------------------
// Descriptors (shared — these describe object *shapes*, not test state)
// ---------------------------------------------------------------------------
//
// Closure object layout (see closure_design.md):
//     [header(8)] [func_ref(16)] [mask(8)] [captured_data_ptr(8)]
//   Payload = 32 bytes. `captured_data_ptr` is a heap pointer at payload
//   offset 24 (i.e. object offset 32).
//
// Captured data layout (Materialized):
//     [header(8)] [tag(1)] [padding(7)] [captured values packed]
//   Payload = 8 + sum(captured sizes). No pointer offsets for these
//   tests (captures are u64 scalars).

const CLOSURE_DESC: DescriptorId = DescriptorId(0);
const CAPTURED_0: DescriptorId = DescriptorId(1);
const CAPTURED_1_U64: DescriptorId = DescriptorId(2);
const CAPTURED_2_U64: DescriptorId = DescriptorId(3);
const VEC_U64_DESC: DescriptorId = DescriptorId(4);

fn test_descriptors() -> Vec<ObjectDescriptor> {
    vec![
        ObjectDescriptor::Closure,
        ObjectDescriptor::CapturedData {
            size: 0, // no captured values
            pointer_offsets: vec![],
        },
        ObjectDescriptor::CapturedData {
            size: 8, // 1 * u64
            pointer_offsets: vec![],
        },
        ObjectDescriptor::CapturedData {
            size: 16, // 2 * u64
            pointer_offsets: vec![],
        },
        ObjectDescriptor::Vector {
            elem_size: 8,
            elem_pointer_offsets: vec![],
        },
    ]
}

// ---------------------------------------------------------------------------
// Callees
// ---------------------------------------------------------------------------

/// Callee: `identity(x: u64) -> u64`.
///
/// Returns `x` by relying on the calling convention (return value lives at
/// callee frame offset 0, which already holds the first arg).
fn make_identity(arena: &ExecutableArena) -> ExecutableArenaPtr<Function> {
    use MicroOp::*;
    arena.alloc(Function {
        name: GlobalArenaPtr::from_static("identity"),
        code: arena.alloc_slice_fill_iter([Return]),
        param_sizes: arena.alloc_slice_fill_iter([8u32]),
        param_sizes_sum: 8,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 8 + FRAME_METADATA_SIZE,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })
}

/// Callee: `add_u64(a: u64, b: u64) -> u64`.
///
/// Writes `a + b` to callee frame offset 0 (overlapping `a`'s arg slot) and
/// returns.
fn make_add_u64(arena: &ExecutableArena) -> ExecutableArenaPtr<Function> {
    use MicroOp::*;

    let a = FO(0);
    let b = FO(8);

    arena.alloc(Function {
        name: GlobalArenaPtr::from_static("add_u64"),
        code: arena.alloc_slice_fill_iter([
            AddU64 {
                dst: a,
                lhs: a,
                rhs: b,
            },
            Return,
        ]),
        param_sizes: arena.alloc_slice_fill_iter([8u32, 8u32]),
        param_sizes_sum: 16,
        param_and_local_sizes_sum: 16,
        extended_frame_size: 16 + FRAME_METADATA_SIZE,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })
}

/// Callee: `vector_map(vec: heap_ptr, closure: heap_ptr, n: u64)`.
///
/// Iterates `i` from 0 to `n - 1`, calls `closure(vec[i])`, and stores the
/// result back into `vec[i]`. No return value — the caller discards the
/// callee arg region after the call.
///
/// Frame layout:
///   [0..8)   vec          (arg 0 — heap pointer)
///   [8..16)  closure      (arg 1 — heap pointer)
///   [16..24) n            (arg 2 — u64)
///   [24..32) i            (local)
///   [32..40) elem         (local — scratch for vec[i])
///   [40..56) vec_ref      (local — fat pointer to the `vec` slot)
fn make_vector_map(arena: &ExecutableArena) -> ExecutableArenaPtr<Function> {
    use MicroOp::*;

    let vec = FO(0);
    let closure = FO(8);
    let n = FO(16);
    let i = FO(24);
    let elem = FO(32);
    let vec_ref = FO(40);
    let args_and_locals: usize = 56;
    let callee_arg0 = FO((args_and_locals + FRAME_METADATA_SIZE) as u32);

    arena.alloc(Function {
        name: GlobalArenaPtr::from_static("vector_map"),
        code: arena.alloc_slice_fill_iter([
            // pc 0: vec_ref = &vec (fat pointer to the `vec` heap-ptr slot)
            SlotBorrow {
                dst: vec_ref,
                local: vec,
            },
            // pc 1: i = 0
            StoreImm8 { dst: i, imm: 0 },
            // pc 2: LOOP_HEAD — if i >= n goto END (pc 9)
            JumpGreaterEqualU64 {
                target: CO(9),
                lhs: i,
                rhs: n,
            },
            // pc 3: elem = vec[i]
            VecLoadElem {
                dst: elem,
                vec_ref,
                idx: i,
                elem_size: 8,
            },
            // pc 4: call closure(elem); return value at callee_arg0
            MicroOp::CallClosure(Box::new(CallClosureOp {
                closure_src: closure,
                provided_args: vec![SizedSlot {
                    offset: elem,
                    size: 8,
                }],
            })),
            // pc 5: elem = <return value>
            Move8 {
                dst: elem,
                src: callee_arg0,
            },
            // pc 6: vec[i] = elem
            VecStoreElem {
                vec_ref,
                idx: i,
                src: elem,
                elem_size: 8,
            },
            // pc 7: i += 1
            AddU64Imm {
                dst: i,
                src: i,
                imm: 1,
            },
            // pc 8: goto LOOP_HEAD (pc 2)
            Jump { target: CO(2) },
            // pc 9:
            Return,
        ]),
        param_sizes: arena.alloc_slice_fill_iter([8u32, 8u32, 8u32]),
        param_sizes_sum: 24,
        param_and_local_sizes_sum: args_and_locals,
        // Needs room for the closure's callee arg region. `exec_call_closure`
        // writes *all* of the callee's parameters (captured + provided) into
        // this region, so we reserve for the full `param_sizes_sum` of the underlying
        // function. The closure here wraps `add_u64`, whose two u64 params
        // require 16 bytes total.
        extended_frame_size: args_and_locals + FRAME_METADATA_SIZE + 16,
        zero_frame: true,
        // Heap pointers: vec (arg 0), closure (arg 1), vec_ref base.
        frame_layout: FrameLayoutInfo::new(arena, [vec, closure, vec_ref]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })
}

fn run_main_and_get_u64_result(main: &Function, descriptors: &[ObjectDescriptor]) -> u64 {
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, descriptors, gas_meter, main);
    ctx.run().unwrap();
    ctx.root_result()
}

// ---------------------------------------------------------------------------
// Test 1: identity closure — no captures, one provided arg
// ---------------------------------------------------------------------------
//
// Covers the "no captured data to copy" path: mask=0, empty captured list.
//
// Main frame:
//   [0..8)   result       (output)
//   [8..16)  closure_ptr  (heap pointer)
//   [16..24) input        (u64 to pass as the provided arg)
// args_and_locals = 24; callee arg region starts at 24 + 24 = 48 and needs
// 8 bytes for identity's single u64 arg → extended_frame_size = 56.

#[test]
fn identity_no_captures() {
    use MicroOp::*;

    let result = FO(0);
    let closure = FO(8);
    let input = FO(16);
    let args_and_locals: usize = 24;
    let callee_arg0 = FO((args_and_locals + FRAME_METADATA_SIZE) as u32);

    let arena = ExecutableArena::new();
    let identity = make_identity(&arena);

    let main = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("main"),
        code: arena.alloc_slice_fill_iter(vec![
            StoreImm8 {
                dst: input,
                imm: 42,
            },
            MicroOp::PackClosure(Box::new(PackClosureOp {
                dst: closure,
                func_ref: ClosureFuncRef::Resolved(identity),
                mask: 0,
                closure_descriptor_id: CLOSURE_DESC,
                captured_data_descriptor_id: CAPTURED_0,
                captured: vec![],
            })),
            MicroOp::CallClosure(Box::new(CallClosureOp {
                closure_src: closure,
                provided_args: vec![SizedSlot {
                    offset: input,
                    size: 8,
                }],
            })),
            Move8 {
                dst: result,
                src: callee_arg0,
            },
            Return,
        ]),
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: args_and_locals,
        extended_frame_size: args_and_locals + FRAME_METADATA_SIZE + 8,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [closure]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    });

    let result =
        run_main_and_get_u64_result(unsafe { main.as_ref_unchecked() }, &test_descriptors());
    assert_eq!(result, 42);
}

// ---------------------------------------------------------------------------
// Test 2: one capture at position 0, provided at position 1
// ---------------------------------------------------------------------------
//
// mask = 0b01: captures `a`, provides `b`. Verifies captured values land at
// the right callee slot and the mask-based interleaving works.
//
// Main frame:
//   [0..8)   result
//   [8..16)  closure_ptr  (heap pointer)
//   [16..24) a            (captured at pack time)
//   [24..32) b            (provided at call time)
// args_and_locals = 32; callee arg region starts at 56 and needs 16 bytes
// for add_u64's two u64 args → extended_frame_size = 72.

#[test]
fn add_captured_a_provided_b() {
    use MicroOp::*;

    let result = FO(0);
    let closure = FO(8);
    let a = FO(16);
    let b = FO(24);
    let args_and_locals: usize = 32;
    let callee_arg0 = FO((args_and_locals + FRAME_METADATA_SIZE) as u32);

    let arena = ExecutableArena::new();
    let add = make_add_u64(&arena);

    let main = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("main"),
        code: arena.alloc_slice_fill_iter(vec![
            StoreImm8 { dst: a, imm: 10 },
            StoreImm8 { dst: b, imm: 32 },
            MicroOp::PackClosure(Box::new(PackClosureOp {
                dst: closure,
                func_ref: ClosureFuncRef::Resolved(add),
                mask: 0b01, // capture position 0
                closure_descriptor_id: CLOSURE_DESC,
                captured_data_descriptor_id: CAPTURED_1_U64,
                captured: vec![SizedSlot { offset: a, size: 8 }],
            })),
            MicroOp::CallClosure(Box::new(CallClosureOp {
                closure_src: closure,
                provided_args: vec![SizedSlot { offset: b, size: 8 }],
            })),
            Move8 {
                dst: result,
                src: callee_arg0,
            },
            Return,
        ]),
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: args_and_locals,
        extended_frame_size: args_and_locals + FRAME_METADATA_SIZE + 16,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [closure]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    });

    let result =
        run_main_and_get_u64_result(unsafe { main.as_ref_unchecked() }, &test_descriptors());
    assert_eq!(result, 42);
}

// ---------------------------------------------------------------------------
// Test 3: one capture at position 1, provided at position 0
// ---------------------------------------------------------------------------
//
// mask = 0b10: captures `b`, provides `a`. Mirror of test 2 — exercises the
// case where the captured value doesn't sit at the start of the callee arg
// region.

#[test]
fn add_provided_a_captured_b() {
    use MicroOp::*;

    let result = FO(0);
    let closure = FO(8);
    let a = FO(16);
    let b = FO(24);
    let args_and_locals: usize = 32;
    let callee_arg0 = FO((args_and_locals + FRAME_METADATA_SIZE) as u32);

    let arena = ExecutableArena::new();
    let add = make_add_u64(&arena);

    let main = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("main"),
        code: arena.alloc_slice_fill_iter(vec![
            StoreImm8 { dst: a, imm: 7 },
            StoreImm8 { dst: b, imm: 35 },
            MicroOp::PackClosure(Box::new(PackClosureOp {
                dst: closure,
                func_ref: ClosureFuncRef::Resolved(add),
                mask: 0b10, // capture position 1
                closure_descriptor_id: CLOSURE_DESC,
                captured_data_descriptor_id: CAPTURED_1_U64,
                captured: vec![SizedSlot { offset: b, size: 8 }],
            })),
            MicroOp::CallClosure(Box::new(CallClosureOp {
                closure_src: closure,
                provided_args: vec![SizedSlot { offset: a, size: 8 }],
            })),
            Move8 {
                dst: result,
                src: callee_arg0,
            },
            Return,
        ]),
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: args_and_locals,
        extended_frame_size: args_and_locals + FRAME_METADATA_SIZE + 16,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [closure]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    });

    let result =
        run_main_and_get_u64_result(unsafe { main.as_ref_unchecked() }, &test_descriptors());
    assert_eq!(result, 42);
}

// ---------------------------------------------------------------------------
// Test 4: all captured — no provided args
// ---------------------------------------------------------------------------
//
// mask = 0b11: both params captured. Verifies the "no provided args" code
// path and that successive captured values advance through the captured
// data region correctly.

#[test]
fn add_all_captured() {
    use MicroOp::*;

    let result = FO(0);
    let closure = FO(8);
    let a = FO(16);
    let b = FO(24);
    let args_and_locals: usize = 32;
    let callee_arg0 = FO((args_and_locals + FRAME_METADATA_SIZE) as u32);

    let arena = ExecutableArena::new();
    let add = make_add_u64(&arena);

    let main = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("main"),
        code: arena.alloc_slice_fill_iter(vec![
            StoreImm8 { dst: a, imm: 15 },
            StoreImm8 { dst: b, imm: 27 },
            MicroOp::PackClosure(Box::new(PackClosureOp {
                dst: closure,
                func_ref: ClosureFuncRef::Resolved(add),
                mask: 0b11,
                closure_descriptor_id: CLOSURE_DESC,
                captured_data_descriptor_id: CAPTURED_2_U64,
                captured: vec![SizedSlot { offset: a, size: 8 }, SizedSlot {
                    offset: b,
                    size: 8,
                }],
            })),
            MicroOp::CallClosure(Box::new(CallClosureOp {
                closure_src: closure,
                provided_args: vec![],
            })),
            Move8 {
                dst: result,
                src: callee_arg0,
            },
            Return,
        ]),
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: args_and_locals,
        extended_frame_size: args_and_locals + FRAME_METADATA_SIZE + 16,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [closure]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    });

    let result =
        run_main_and_get_u64_result(unsafe { main.as_ref_unchecked() }, &test_descriptors());
    assert_eq!(result, 42);
}

// ---------------------------------------------------------------------------
// Test 5: vector map — apply `|x| x + y` with `y` captured
// ---------------------------------------------------------------------------
//
// Main builds `[2, 3, 4]`, packs a closure wrapping `add_u64` with the
// second parameter captured (mask=0b10, captured y=10), then calls
// `vector_map(vec, closure, 3)` which applies the closure to each element
// in place — i.e. computes `elem + y` for each element. Finally main sums
// the resulting vector and returns it.
// Expected: (2+10) + (3+10) + (4+10) = 39.
//
// The call chain is: main → vector_map → CallClosure → add_u64. This
// exercises nested function calls, a closure with a capture called in a
// loop, and heap pointers rooted across call boundaries.
//
// Main frame:
//   [0..8)   result
//   [8..16)  closure_ptr  (heap pointer)
//   [16..24) vec          (heap pointer to the Vec<u64>)
//   [24..32) i            (loop counter for sum)
//   [32..40) elem         (scratch for vec[i])
//   [40..48) tmp          (scratch for push immediates)
//   [48..64) vec_ref      (16-byte fat pointer, base @ 48 is a heap pointer)
//   [64..72) y            (captured by the closure)
// args_and_locals = 72; callee (vector_map) arg region is 24 bytes →
// extended_frame_size = 72 + 24 + 24 = 120.

#[test]
fn vector_map_add_captured() {
    use MicroOp::*;

    let result = FO(0);
    let closure = FO(8);
    let vec = FO(16);
    let i = FO(24);
    let elem = FO(32);
    let tmp = FO(40);
    let vec_ref = FO(48);
    let y = FO(64);
    let args_and_locals: usize = 72;
    let callee_arg0: u32 = (args_and_locals + FRAME_METADATA_SIZE) as u32;

    let n: u64 = 3;
    let y_val: u64 = 10;

    let arena = ExecutableArena::new();
    let add = make_add_u64(&arena);
    let vector_map = make_vector_map(&arena);

    let main = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("main"),
        code: arena.alloc_slice_fill_iter(vec![
            // === Build vector [2, 3, 4] ===
            // pc 0: vec = new
            VecNew { dst: vec },
            // pc 1: vec_ref = &vec
            SlotBorrow {
                dst: vec_ref,
                local: vec,
            },
            // pc 2..7: push 2, 3, 4
            StoreImm8 { dst: tmp, imm: 2 },
            VecPushBack {
                vec_ref,
                elem: tmp,
                elem_size: 8,
                descriptor_id: VEC_U64_DESC,
            },
            StoreImm8 { dst: tmp, imm: 3 },
            VecPushBack {
                vec_ref,
                elem: tmp,
                elem_size: 8,
                descriptor_id: VEC_U64_DESC,
            },
            StoreImm8 { dst: tmp, imm: 4 },
            VecPushBack {
                vec_ref,
                elem: tmp,
                elem_size: 8,
                descriptor_id: VEC_U64_DESC,
            },
            // === Pack closure: `|x| x + y`, wrapping add_u64 with `b` captured ===
            // pc 8: y = 10
            StoreImm8 { dst: y, imm: y_val },
            // pc 9: pack with mask=0b10 (capture position 1 = `b`)
            MicroOp::PackClosure(Box::new(PackClosureOp {
                dst: closure,
                func_ref: ClosureFuncRef::Resolved(add),
                mask: 0b10,
                closure_descriptor_id: CLOSURE_DESC,
                captured_data_descriptor_id: CAPTURED_1_U64,
                captured: vec![SizedSlot { offset: y, size: 8 }],
            })),
            // === Call vector_map(vec, closure, n) ===
            // pc 10..12: place args in callee arg region.
            Move8 {
                dst: FO(callee_arg0),
                src: vec,
            },
            Move8 {
                dst: FO(callee_arg0 + 8),
                src: closure,
            },
            StoreImm8 {
                dst: FO(callee_arg0 + 16),
                imm: n,
            },
            // pc 13: call vector_map
            CallDirect { ptr: vector_map },
            // === Sum loop: result = sum(vec) ===
            // pc 14: result = 0
            StoreImm8 {
                dst: result,
                imm: 0,
            },
            // pc 15: i = 0
            StoreImm8 { dst: i, imm: 0 },
            // pc 16: SUM_HEAD — if i >= n goto END (pc 21)
            JumpGreaterEqualU64Imm {
                target: CO(21),
                src: i,
                imm: n,
            },
            // pc 17: elem = vec[i]
            VecLoadElem {
                dst: elem,
                vec_ref,
                idx: i,
                elem_size: 8,
            },
            // pc 18: result += elem
            AddU64 {
                dst: result,
                lhs: result,
                rhs: elem,
            },
            // pc 19: i += 1
            AddU64Imm {
                dst: i,
                src: i,
                imm: 1,
            },
            // pc 20: goto SUM_HEAD (pc 16)
            Jump { target: CO(16) },
            // pc 21: Return
            Return,
        ]),
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: args_and_locals,
        extended_frame_size: args_and_locals + FRAME_METADATA_SIZE + 24,
        zero_frame: true,
        // Heap pointers held across safe points: closure object, vec, and
        // the base of vec_ref.
        frame_layout: FrameLayoutInfo::new(&arena, [closure, vec, vec_ref]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    });

    let result =
        run_main_and_get_u64_result(unsafe { main.as_ref_unchecked() }, &test_descriptors());
    assert_eq!(result, (2 + y_val) + (3 + y_val) + (4 + y_val));
}
