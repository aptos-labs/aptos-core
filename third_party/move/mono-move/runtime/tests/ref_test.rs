// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Low-level reference tests that cannot be expressed in Move source. The
//! reference behaviors that are expressible (borrows surviving GC, cross-frame
//! refs, struct-field refs) live in the differential suite under
//! `testsuite/tests/test_cases/differential/{refs,structs}/`.

mod common;

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    native::NativeExtensions, Code, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp,
    SortedSafePointEntries, FRAME_METADATA_SIZE,
};

/// `ReadRef`/`WriteRef` whose runtime target aliases the dst/src slot.
/// Guards the interpreter's overlap-safe copy. Not expressible in Move source:
/// the borrow checker forbids reading or writing a local while it is borrowed,
/// which is exactly the aliasing this exercises.
#[test]
fn ref_self_copy() {
    use MicroOp::*;

    let result: u32 = 0;
    let local: u32 = 8;
    let r#ref: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        StoreImm8 { dst: FO(local), imm: 99u64.to_le_bytes() },
        SlotBorrow { dst: FO(r#ref), local: FO(local) },
        // Self-overlapping ReadRef: dst == target == fp+local.
        ReadRef { dst: FO(local), ref_ptr: FO(r#ref), size: 8 },
        // Self-overlapping WriteRef: src == target == fp+local.
        WriteRef { ref_ptr: FO(r#ref), src: FO(local), size: 8 },
        Move8 { dst: FO(result), src: FO(local) },
        Return,
    ];
    let function = Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(code),
        entry_gas: 0,
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 32,
        extended_frame_size: 32 + FRAME_METADATA_SIZE,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let result =
        common::with_test_interpreter(&function, u64::MAX, NativeExtensions::new(), |ctx| {
            ctx.run().unwrap();
            ctx.root_result()
        });

    assert_eq!(
        result, 99,
        "self-overlapping ReadRef/WriteRef should preserve the value"
    );
}
