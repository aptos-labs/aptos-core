// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Context for lowering stackless exec IR to micro-ops.
//!
//! Builds frame layout information (slot offsets/sizes) needed by the lowerer.
//! All lookups are O(1) via indexed Vecs — no maps.

use crate::stackless_exec_ir::{FunctionIR, Instr};
use anyhow::Result;
use mono_move_core::FRAME_METADATA_SIZE;
use move_binary_format::{access::ModuleAccess, file_format::SignatureToken, CompiledModule};
use move_vm_types::loaded_data::runtime_types::Type;

/// Returns the byte size of a concrete type, or None if the type is
/// not concrete (e.g., contains type parameters) or not yet handled.
pub fn type_size(ty: &Type) -> Option<usize> {
    match ty {
        Type::Bool
        | Type::U8
        | Type::I8
        | Type::U16
        | Type::I16
        | Type::U32
        | Type::I32
        | Type::U64
        | Type::I64
        | Type::Address
        | Type::Signer => Some(8),
        Type::U128 | Type::I128 => Some(16),
        Type::U256 | Type::I256 => Some(32),
        Type::Vector(_) => Some(8),
        Type::Reference(_) | Type::MutableReference(_) => Some(16),
        Type::TyParam(_) => None,
        Type::Struct { .. } => None,
        Type::StructInstantiation { .. } => None,
        Type::Function { .. } => None,
    }
}

fn sig_token_size(tok: &SignatureToken) -> Option<usize> {
    match tok {
        SignatureToken::Bool
        | SignatureToken::U8
        | SignatureToken::I8
        | SignatureToken::U16
        | SignatureToken::I16
        | SignatureToken::U32
        | SignatureToken::I32
        | SignatureToken::U64
        | SignatureToken::I64
        | SignatureToken::Address
        | SignatureToken::Signer => Some(8),
        SignatureToken::U128 | SignatureToken::I128 => Some(16),
        SignatureToken::U256 | SignatureToken::I256 => Some(32),
        SignatureToken::Vector(_) => Some(8),
        SignatureToken::Reference(_) | SignatureToken::MutableReference(_) => Some(16),
        SignatureToken::TypeParameter(_) => None,
        SignatureToken::Struct(_) => None,
        SignatureToken::StructInstantiation(_, _) => None,
        SignatureToken::Function(_, _, _) => None,
    }
}

fn sig_token_to_type(tok: &SignatureToken) -> Option<Type> {
    match tok {
        SignatureToken::Bool => Some(Type::Bool),
        SignatureToken::U8 => Some(Type::U8),
        SignatureToken::I8 => Some(Type::I8),
        SignatureToken::U16 => Some(Type::U16),
        SignatureToken::I16 => Some(Type::I16),
        SignatureToken::U32 => Some(Type::U32),
        SignatureToken::I32 => Some(Type::I32),
        SignatureToken::U64 => Some(Type::U64),
        SignatureToken::I64 => Some(Type::I64),
        SignatureToken::U128 => Some(Type::U128),
        SignatureToken::I128 => Some(Type::I128),
        SignatureToken::U256 => Some(Type::U256),
        SignatureToken::I256 => Some(Type::I256),
        SignatureToken::Address => Some(Type::Address),
        SignatureToken::Signer => Some(Type::Signer),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SlotInfo {
    pub offset: u32,
    pub size: u32,
}

pub struct CallSiteInfo {
    pub callee_func_id: u32,
    pub arg_write_slots: Vec<SlotInfo>,
    pub ret_read_slots: Vec<SlotInfo>,
    pub param_types: Vec<Type>,
    pub ret_types: Vec<Type>,
}

pub struct LoweringContext {
    pub home_slots: Vec<SlotInfo>,
    pub frame_data_size: u32,
    pub call_sites: Vec<CallSiteInfo>,
    pub return_slots: Vec<SlotInfo>,
    /// Maximum number of Xfer slots needed across all call sites in this function.
    pub num_xfer_slots: u16,
}

/// Try to build a LoweringContext for a monomorphic function.
/// Returns `Ok(None)` if any type is not concrete (e.g. type parameters, structs).
/// Returns `Err` for unexpected failures.
pub fn try_build_context(
    module: &CompiledModule,
    func_ir: &FunctionIR,
) -> Result<Option<LoweringContext>> {
    // Use an inner function that returns Option to keep `?` ergonomic for
    // non-concrete type checks, then wrap the result.
    let inner = try_build_context_inner(module, func_ir);
    match inner {
        Some(result) => result.map(Some),
        None => Ok(None),
    }
}

/// Returns `None` if any type is not concrete.
/// Returns `Some(Ok(ctx))` on success, `Some(Err(..))` on unexpected failure.
fn try_build_context_inner(
    module: &CompiledModule,
    func_ir: &FunctionIR,
) -> Option<Result<LoweringContext>> {
    // 1. Compute home slot layout
    let mut home_slots = Vec::with_capacity(func_ir.num_home_slots as usize);
    let mut offset: u32 = 0;
    for slot_ty in &func_ir.home_slot_types {
        let size = type_size(slot_ty)? as u32;
        home_slots.push(SlotInfo { offset, size });
        offset += size;
    }
    let frame_data_size = offset;

    // 2. Build return_slots from this function's return signature
    let handle = module.function_handle_at(func_ir.handle_idx);
    let ret_sig = &module.signature_at(handle.return_).0;
    let mut return_slots = Vec::with_capacity(ret_sig.len());
    let mut ret_offset: u32 = 0;
    for tok in ret_sig {
        let size = sig_token_size(tok)? as u32;
        return_slots.push(SlotInfo {
            offset: ret_offset,
            size,
        });
        ret_offset += size;
    }

    // 3. Scan instructions for Call/CallGeneric to build call_sites
    let callee_base = frame_data_size + FRAME_METADATA_SIZE as u32;
    let mut call_sites = Vec::new();

    for instr in func_ir.instrs() {
        let handle_idx = match instr {
            Instr::Call(_, idx, _) => *idx,
            Instr::CallGeneric(_, idx, _) => {
                let inst = &module.function_instantiations[idx.0 as usize];
                inst.handle
            },
            _ => continue,
        };

        let callee_handle = module.function_handle_at(handle_idx);
        let callee_func_id = handle_idx.0 as u32;

        // Param layout
        let param_sig = &module.signature_at(callee_handle.parameters).0;
        let mut arg_write_slots = Vec::with_capacity(param_sig.len());
        let mut param_types = Vec::with_capacity(param_sig.len());
        let mut arg_offset = callee_base;
        for tok in param_sig {
            let size = sig_token_size(tok)? as u32;
            arg_write_slots.push(SlotInfo {
                offset: arg_offset,
                size,
            });
            param_types.push(sig_token_to_type(tok)?);
            arg_offset += size;
        }

        // Return layout
        let callee_ret_sig = &module.signature_at(callee_handle.return_).0;
        let mut ret_read_slots = Vec::with_capacity(callee_ret_sig.len());
        let mut ret_types = Vec::with_capacity(callee_ret_sig.len());
        let mut callee_ret_offset = callee_base;
        for tok in callee_ret_sig {
            let size = sig_token_size(tok)? as u32;
            ret_read_slots.push(SlotInfo {
                offset: callee_ret_offset,
                size,
            });
            ret_types.push(sig_token_to_type(tok)?);
            callee_ret_offset += size;
        }

        call_sites.push(CallSiteInfo {
            callee_func_id,
            arg_write_slots,
            ret_read_slots,
            param_types,
            ret_types,
        });
    }

    Some(Ok(LoweringContext {
        home_slots,
        frame_data_size,
        call_sites,
        return_slots,
        num_xfer_slots: func_ir.num_xfer_slots,
    }))
}
