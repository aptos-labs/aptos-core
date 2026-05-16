// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared bench helpers. Included via `#[path = "helpers.rs"] mod helpers;`
//! in each bench binary. Not listed as a `[[bench]]` target, so it is never
//! compiled standalone.

use mono_move_core::{FunctionPtr, MicroOpGasSchedule};
use mono_move_gas::GasInstrumentor;

/// Replace each function's micro-ops with a gas-instrumented version.
pub fn gas_instrument(funcs: &[FunctionPtr]) {
    let instrumentor = GasInstrumentor::new(MicroOpGasSchedule);
    for ptr in funcs {
        let func = unsafe { ptr.as_ref_unchecked() };
        let new_code = instrumentor.run(func.code.load().as_slice().to_vec());
        func.code.store(new_code);
    }
}
