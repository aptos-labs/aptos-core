// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared bench helpers. Included via `#[path = "helpers.rs"] mod helpers;`
//! in each bench binary. Not listed as a `[[bench]]` target, so it is never
//! compiled standalone.

use mono_move_core::{Code, FrameLayoutInfo, Function, MicroOpGasSchedule, SortedSafePointEntries};
use mono_move_gas::GasInstrumentor;

/// Build a gas-instrumented copy of `funcs`.
///
/// The caller should build the program fresh (without calling
/// `Function::resolve_calls`) before passing it here, so the code still
/// contains `CallFunc` (index-based) ops.
///
/// Frame layouts are re-created as empty; these benchmark programs do not
/// trigger GC, so the omission has no effect on execution.
pub fn gas_instrument(funcs: &[Option<Function>]) -> Vec<Option<Function>> {
    let instrumentor = GasInstrumentor::new(MicroOpGasSchedule);
    funcs
        .iter()
        .map(|f| {
            let func = f.as_ref()?;
            Some(Function {
                name: func.name,
                code: Code::from_vec(instrumentor.run(func.code.load().as_slice().to_vec())),
                param_sizes: func.param_sizes.clone(),
                param_sizes_sum: func.param_sizes_sum,
                param_and_local_sizes_sum: func.param_and_local_sizes_sum,
                extended_frame_size: func.extended_frame_size,
                zero_frame: func.zero_frame,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
        })
        .collect()
}
