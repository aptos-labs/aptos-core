// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared bench helpers. Included via `#[path = "helpers.rs"] mod helpers;`
//! in each bench binary. Not listed as a `[[bench]]` target, so it is never
//! compiled standalone.

use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr};
use mono_move_core::{FrameLayoutInfo, Function, MicroOpGasSchedule, SortedSafePointEntries};
use mono_move_gas::GasInstrumentor;

/// Build a gas-instrumented copy of `func_ptrs`, re-allocated in a fresh arena.
///
/// The caller should build the program fresh (without calling
/// `Function::resolve_calls`) before passing it here, so the code still
/// contains `CallFunc` (index-based) ops. After this call, invoke
/// `Function::resolve_calls` on the returned table to patch those ops to
/// direct pointers into the instrumented arena.
///
/// Frame layouts are re-created as empty; these benchmark programs do not
/// trigger GC, so the omission has no effect on execution.
///
/// # Safety
///
/// Each pointer in `func_ptrs` must be valid (the owning arena must outlive
/// this call).
pub unsafe fn gas_instrument(
    func_ptrs: &[Option<ExecutableArenaPtr<Function>>],
) -> (Vec<Option<ExecutableArenaPtr<Function>>>, ExecutableArena) {
    let arena = ExecutableArena::new();
    let instrumentor = GasInstrumentor::new(MicroOpGasSchedule);
    let new_fns = func_ptrs
        .iter()
        .map(|f| {
            let fp = (*f)?;
            // SAFETY: caller guarantees the pointer is valid.
            let func = unsafe { fp.as_ref_unchecked() };
            let raw = unsafe { func.code.as_ref_unchecked() };
            let instrumented = instrumentor.run(raw.to_vec());
            let code = arena.alloc_slice_fill_iter(instrumented);
            Some(arena.alloc(Function {
                name: func.name,
                code,
                param_sizes: func.param_sizes,
                param_sizes_sum: func.param_sizes_sum,
                param_and_local_sizes_sum: func.param_and_local_sizes_sum,
                extended_frame_size: func.extended_frame_size,
                zero_frame: func.zero_frame,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            }))
        })
        .collect();
    (new_fns, arena)
}
