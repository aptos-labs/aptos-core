// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end memory-check tests: a hand-built function whose static offsets
//! pass the verifier but reach outside the object at run time. The checks
//! only exist in the hardened and exhaustive tiers, so each test is a no-op
//! in a production build.

mod common;

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    native::NoNatives,
    types::{Type, EMPTY_TYPE_LIST},
    Code, FrameLayoutInfo, FrameOffset as FO, Function, FunctionDefinitionIndex, GasMeter, MicroOp,
    NoModuleProvider, NoResourceProvider, SortedSafePointEntries, FRAME_METADATA_SIZE,
};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_mem::checks::{take_last_fault, MemFaultKind, HARDENED};
use mono_move_runtime::{InterpreterContext, ProductionNativeRegistry};

/// `HeapReadOffset` at a static offset past the object's 8-byte payload. The
/// verifier only checks that `offset + size` fits a `u32`, so this passes
/// verification; the hardened `field_range` check catches it at run time.
#[test]
fn heap_read_offset_past_payload_faults() {
    if !HARDENED {
        return;
    }
    use MicroOp::*;

    // A struct descriptor whose objects have an 8-byte payload. The key only
    // has to be a distinct interned type.
    static STRUCT_TY: Type = Type::U64;
    let struct_ty = GlobalArenaPtr::from_static(&STRUCT_TY);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .expect("worker 0 arena is free in a fresh context");
    let descriptor_id = guard.publish_struct_descriptor(struct_ty, 8, &[]);

    let obj: u32 = 0; // heap pointer slot
    let obj_ref: u32 = 8; // 16-byte fat reference to `obj`
    let out: u32 = 24; // 8-byte read destination
    #[rustfmt::skip]
    let code = vec![
        HeapNew { dst: FO(obj), descriptor_id },
        SlotBorrow { dst: FO(obj_ref), local: FO(obj) },
        HeapReadOffset { dst: FO(out), obj_ref: FO(obj_ref), offset: 64, size: 8 },
        Return,
    ];
    let entry = Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        def_idx: FunctionDefinitionIndex(0),
        has_module_lock: false,
        code: Code::from_vec(code),
        entry_gas: 0,
        param_slots: vec![],
        param_tys: vec![],
        return_slots: vec![],
        return_tys: EMPTY_TYPE_LIST,
        param_region_size: 0,
        param_and_local_sizes_sum: 32,
        extended_frame_size: 32 + FRAME_METADATA_SIZE,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(obj)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    mono_move_runtime::assert_verified(&entry, &guard);

    let natives = ProductionNativeRegistry::new();
    let loader = Loader::new_with_policy(
        &guard,
        &NoModuleProvider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &NoNatives,
    );
    let mut interp = InterpreterContext::new(
        loader,
        GasMeter::new(u64::MAX),
        &NoResourceProvider,
        &natives,
    );

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        interp.build_call(&entry).unwrap().run().is_ok()
    }));
    assert!(r.is_err(), "the out-of-payload read must fault");
    let fault = take_last_fault().expect("fault recorded");
    assert_eq!(fault.kind, MemFaultKind::FieldOutOfPayload);
    assert_eq!(fault.extra, 64, "the fault names the offending offset");
    assert_eq!(fault.size, 8, "the fault names the access width");
}
