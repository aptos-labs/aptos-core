// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Specializer subsystem error types.
//!
//! Following the two-layer error model (see
//! `mono-move/docs/error_design.md`), the specializer owns a fully-typed
//! internal error taxonomy. [`SpecializerError`] is a thin dispatcher over
//! one error enum per pipeline pass ([`SsaConversionError`],
//! [`SlotAllocError`], [`XferVerifierError`], [`LoweringError`]); each pass
//! enum carries its own exhaustive `IntoExecutionError` impl, the single
//! place where a variant is assigned a public `ExecutionErrorKind` category
//! — adding a variant fails to compile until that decision is made.
//!
//! Like the loader and runtime subsystems, the definitions currently live
//! in `mono-move-core` (see `mono_move_core::vm_error`, tracked by the
//! `TODO(cleanup)` there to extract the internal enums into their own
//! crate) and are re-exported here so the specializer presents its own
//! error surface.

pub use mono_move_core::{
    GasInstrumentationError, GasInstrumentationResult, LoweringError, LoweringResult,
    SlotAllocError, SlotAllocResult, SpecializerError, SpecializerResult, SsaConversionError,
    SsaConversionResult, XferVerifierError, XferVerifierResult,
};
