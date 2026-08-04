// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The interpreter's `checkpoint()` / `rollback()` drive the native extensions'
//! checkpoint hooks in lockstep with the read-write set.

mod common;

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    native::{NativeExtension, NativeExtensions},
    Code, FrameLayoutInfo, Function, MicroOp, SortedSafePointEntries, VMResult,
};

/// Test extension that records the checkpoint hooks the interpreter fires, so
/// the test can confirm extensions move in lockstep with the read-write set.
#[derive(Default)]
struct CheckpointProbe {
    depth: usize,
    checkpoints: usize,
    rolled_back: usize,
}

impl NativeExtension for CheckpointProbe {
    unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {}

    fn on_checkpoint(&mut self) {
        self.depth += 1;
        self.checkpoints += 1;
    }

    fn on_rollback(&mut self, n: usize) -> VMResult<()> {
        self.depth -= n;
        self.rolled_back += n;
        Ok(())
    }
}

fn trivial_program() -> Function {
    Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![MicroOp::Return]),
        entry_gas: 0,
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 40,
        extended_frame_size: 64,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

#[test]
fn checkpoint_rollback_drives_extensions_in_lockstep() {
    let func = trivial_program();
    let mut extensions = NativeExtensions::new();
    extensions.add(CheckpointProbe::default());
    common::with_test_interpreter(&func, u64::MAX, extensions, |ctx| {
        ctx.checkpoint().unwrap();
        ctx.checkpoint().unwrap();
        assert_eq!(ctx.checkpoint_depth(), 2);
        {
            let probe = ctx.extensions().get_mut::<CheckpointProbe>().unwrap();
            assert_eq!(probe.depth, 2, "extension advanced with the read-write set");
            assert_eq!(probe.checkpoints, 2);
        }

        // A partial rollback undoes the extension's checkpoints too.
        ctx.rollback(1).unwrap();
        assert_eq!(ctx.checkpoint_depth(), 1);
        {
            let probe = ctx.extensions().get_mut::<CheckpointProbe>().unwrap();
            assert_eq!(probe.depth, 1);
            assert_eq!(probe.rolled_back, 1);
        }

        // n == 0 leaves the extensions untouched.
        ctx.rollback(0).unwrap();
        let probe = ctx.extensions().get_mut::<CheckpointProbe>().unwrap();
        assert_eq!(probe.depth, 1);
        assert_eq!(probe.rolled_back, 1);
    });
}
