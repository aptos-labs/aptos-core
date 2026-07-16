// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod stackless_exec_ir;

pub mod destack;
pub mod error;
mod gas;
pub mod lower;

pub use destack::destack;
pub use error::{
    GasInstrumentationError, GasInstrumentationResult, LoweringError, LoweringResult,
    SlotAllocError, SlotAllocResult, SpecializerError, SpecializerResult, SsaConversionError,
    SsaConversionResult, XferVerifierError, XferVerifierResult,
};
pub use stackless_exec_ir::{FunctionIR, ModuleIR};
