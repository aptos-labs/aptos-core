// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lightweight translation validation: checks of the final stackless execution
//! IR against the Move bytecode it was translated from.
//!
//! Passes are plain functions over a read-only per-function [`FuncCtx`], listed
//! in the body of [`validate_module`]. Structural passes produce certified
//! artifacts ([`cfg_equivalence::BlockCorrespondence`]) that later semantic
//! passes can consume instead of re-trusting the raw witness.
//!
//! No user error should show up as translation validation failure.

pub(crate) mod cfg_equivalence;
pub(crate) mod origins;
pub(crate) mod transfer;

use crate::stackless_exec_ir::{FunctionIR, ModuleIR};
use cfg_equivalence::CfgEquivalenceError;
use mono_move_core::{ExecutionErrorKind, IntoExecutionError};
use move_binary_format::{
    access::ModuleAccess,
    control_flow_graph::VMControlFlowGraph,
    file_format::{Bytecode, CodeOffset},
};
use origins::OriginsError;
use thiserror::Error;
use transfer::TransferVerifierError;

/// Facts translation knows and validation needs, carried on [`FunctionIR`].
///
/// Untrusted by validation passes. A corrupted witness must cause a check
/// failure (see `cfg_equivalence`).
pub(crate) struct TranslationWitness {
    /// `Label` -> start offset of the originating basic block in the function's
    /// original Move bytecode. Dense, indexed by `Label.0`. Recorded during
    /// bytecode-to-SSA conversion; no pass creates, deletes, or reorders
    /// blocks.
    pub label_to_offset: Vec<CodeOffset>,
}

/// Read-only per-function view handed to every validation pass.
pub(crate) struct FuncCtx<'a> {
    pub func_ir: &'a FunctionIR,
    /// The function's original bytecode.
    pub code: &'a [Bytecode],
    /// Independent bytecode-CFG oracle, derived directly from the bytecode
    /// via a separate (non-mono-move) implementation.
    pub bytecode_cfg: VMControlFlowGraph,
}

impl<'a> FuncCtx<'a> {
    fn new(
        module_ir: &'a ModuleIR,
        def_idx: usize,
        func_ir: &'a FunctionIR,
    ) -> Result<Self, PassError> {
        let fdef = module_ir
            .module
            .function_defs
            .get(def_idx)
            .ok_or(PassError::MissingBytecode)?;
        if fdef.function != func_ir.handle_idx {
            return Err(PassError::FunctionHandleMismatch {
                ir_handle: func_ir.handle_idx.0,
                def_handle: fdef.function.0,
            });
        }
        if func_ir.def_idx.0 as usize != def_idx {
            return Err(PassError::FunctionDefIdxMismatch {
                ir_def_idx: func_ir.def_idx.0,
                position: def_idx,
            });
        }
        let code = fdef.code.as_ref().ok_or(PassError::MissingBytecode)?;
        Ok(FuncCtx {
            func_ir,
            code: &code.code,
            bytecode_cfg: VMControlFlowGraph::new(&code.code),
        })
    }
}

pub type ValidationResult<T> = Result<T, ValidationError>;

/// Translation-validation failure, carrying the identity of the violating
/// function and the failing pass's error. Fail-fast: the first violating
/// function aborts module validation.
#[derive(Debug, Error)]
#[error("translation validation failed for `{function}`: {source}")]
pub struct ValidationError {
    /// Qualified `module::function` name of the violating function.
    function: String,
    source: PassError,
}

/// A single pass's failure, wrapped by the driver into [`ValidationError`].
#[derive(Debug, Error)]
enum PassError {
    #[error("function has IR but its bytecode code unit is unavailable")]
    MissingBytecode,

    #[error("module has {num_ir} function IR entries for {num_defs} function definitions")]
    FunctionCountMismatch { num_ir: usize, num_defs: usize },

    #[error("function definition has code but no IR")]
    MissingFunctionIR,

    #[error("native function definition has IR")]
    UnexpectedFunctionIR,

    #[error("IR carries function handle {ir_handle}, definition expects {def_handle}")]
    FunctionHandleMismatch { ir_handle: u16, def_handle: u16 },

    #[error("IR carries def index {ir_def_idx}, but sits at position {position}")]
    FunctionDefIdxMismatch { ir_def_idx: u16, position: usize },

    #[error("CFG equivalence: {0}")]
    CfgEquivalence(#[from] CfgEquivalenceError),

    #[error("origin provenance: {0}")]
    Origins(#[from] OriginsError),

    #[error("transfer invariants: {0}")]
    Transfer(#[from] TransferVerifierError),
}

impl IntoExecutionError for ValidationError {
    fn kind(&self) -> ExecutionErrorKind {
        match &self.source {
            PassError::MissingBytecode
            | PassError::FunctionCountMismatch { .. }
            | PassError::MissingFunctionIR
            | PassError::UnexpectedFunctionIR
            | PassError::FunctionHandleMismatch { .. }
            | PassError::FunctionDefIdxMismatch { .. } => ExecutionErrorKind::InvariantViolation,
            PassError::CfgEquivalence(err) => err.kind(),
            PassError::Origins(err) => err.kind(),
            PassError::Transfer(err) => err.kind(),
        }
    }
}

/// Run all validation passes over every non-native function of the module.
pub fn validate_module(module_ir: &ModuleIR) -> ValidationResult<()> {
    check_function_totality(module_ir)?;
    for (def_idx, func_ir) in module_ir.functions.iter().enumerate() {
        let Some(func_ir) = func_ir else { continue };
        validate_function(module_ir, def_idx, func_ir).map_err(|source| ValidationError {
            function: qualified_function_name(module_ir, def_idx),
            source,
        })?;
    }
    Ok(())
}

/// Check that every function definition with code has IR, and every native
/// definition (no code) has no IR.
fn check_function_totality(module_ir: &ModuleIR) -> ValidationResult<()> {
    let function_defs = &module_ir.module.function_defs;
    if module_ir.functions.len() != function_defs.len() {
        return Err(ValidationError {
            function: module_ir.module.self_name().to_string(),
            source: PassError::FunctionCountMismatch {
                num_ir: module_ir.functions.len(),
                num_defs: function_defs.len(),
            },
        });
    }
    for (def_idx, (fdef, func_ir)) in function_defs.iter().zip(&module_ir.functions).enumerate() {
        // TODO(correctness): a native registry entry shadowing a function
        // that has a Move body is invisible here — that is the `(true, true)`
        // case, since lowering resolves natives unconditionally per call site.
        let source = match (fdef.code.is_some(), func_ir.is_some()) {
            (true, false) => PassError::MissingFunctionIR,
            (false, true) => PassError::UnexpectedFunctionIR,
            (true, true) | (false, false) => continue,
        };
        return Err(ValidationError {
            function: qualified_function_name(module_ir, def_idx),
            source,
        });
    }
    Ok(())
}

/// Function name for error attribution, resolved from the verified definition
/// side.
fn qualified_function_name(module_ir: &ModuleIR, def_idx: usize) -> String {
    let module = &module_ir.module;
    match module.function_defs.get(def_idx) {
        Some(fdef) => format!(
            "{}::{}",
            module.self_name(),
            module.identifier_at(module.function_handle_at(fdef.function).name)
        ),
        None => format!("{}::<function #{def_idx}>", module.self_name()),
    }
}

/// The per-function pass list, in execution order.
fn validate_function(
    module_ir: &ModuleIR,
    def_idx: usize,
    func_ir: &FunctionIR,
) -> Result<(), PassError> {
    let ctx = FuncCtx::new(module_ir, def_idx, func_ir)?;
    let blocks = cfg_equivalence::verify(&ctx)?;
    origins::verify(&ctx, &blocks)?;
    transfer::verify(&ctx)?;
    Ok(())
}
