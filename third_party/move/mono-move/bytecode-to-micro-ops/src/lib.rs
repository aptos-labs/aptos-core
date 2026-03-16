// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Direct lowering from Move bytecode to micro-ops.
//!
//! This is a completely independent alternative to the
//! bytecode -> stackless-exec-ir -> micro-ops pipeline. It translates
//! Move bytecode directly to micro-ops with no intermediate IR.
//!
//! The frame layout materializes the operand stack in the frame itself:
//! params and locals get fixed frame slots, and operand stack entries
//! occupy subsequent slots. All values are treated as 8 bytes (u64-only
//! prototype).

pub mod display;

use anyhow::{bail, Result};
use mono_move_micro_ops::instruction::{CodeOffset, FrameOffset, MicroOp, FRAME_METADATA_SIZE};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{Bytecode, FunctionDefinition, FunctionHandleIndex},
    CompiledModule,
};

/// A compile-time operand stack entry tracking the frame byte offset
/// where this value lives.
#[derive(Clone, Copy, Debug)]
struct StackEntry(u32);

/// Frame layout for a function.
struct FrameLayout {
    num_params: u16,
    num_locals: u16,
    stack_base: u32,
    frame_data_size: u32,
    callee_base: u32,
}

/// Result of lowering a single function.
pub struct LoweredFunction {
    pub name: String,
    pub frame_data_size: u32,
    pub ops: Vec<MicroOp>,
}

/// Build a func_id_map from the module: FunctionHandleIndex -> Option<u32>.
/// For same-module definitions, maps to the definition index.
fn build_func_id_map(module: &CompiledModule) -> Vec<Option<u32>> {
    let self_module_handle_idx = module.self_module_handle_idx;
    let num_handles = module.function_handles.len();
    let mut map: Vec<Option<u32>> = vec![None; num_handles];
    for (def_idx, func_def) in module.function_defs.iter().enumerate() {
        let handle = module.function_handle_at(func_def.function);
        if handle.module == self_module_handle_idx {
            map[func_def.function.0 as usize] = Some(def_idx as u32);
        }
    }
    map
}

/// Compute the stack depth change for a bytecode instruction.
/// Returns (pops, pushes).
fn stack_effect(bytecode: &Bytecode, module: &CompiledModule) -> (u16, u16) {
    match bytecode {
        Bytecode::CopyLoc(_) | Bytecode::MoveLoc(_) | Bytecode::LdU64(_) => (0, 1),
        Bytecode::StLoc(_) => (1, 0),
        Bytecode::Add | Bytecode::Sub | Bytecode::Le => (2, 1),
        Bytecode::BrFalse(_) | Bytecode::BrTrue(_) => (1, 0),
        Bytecode::Branch(_) => (0, 0),
        Bytecode::Ret => {
            // Ret pops the return values; for our purposes it drains the stack
            (0, 0)
        },
        Bytecode::Call(handle_idx) => {
            let handle = module.function_handle_at(*handle_idx);
            let num_params = module.signature_at(handle.parameters).0.len() as u16;
            let num_returns = module.signature_at(handle.return_).0.len() as u16;
            (num_params, num_returns)
        },
        _ => (0, 0),
    }
}

/// Compute the frame layout for a function.
fn compute_frame_layout(
    module: &CompiledModule,
    func_def: &FunctionDefinition,
) -> Result<FrameLayout> {
    let handle = module.function_handle_at(func_def.function);
    let num_params = module.signature_at(handle.parameters).0.len() as u16;

    let code = func_def
        .code
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("function has no code"))?;
    let num_locals = module.signature_at(code.locals).0.len() as u16;
    let bytecodes = &code.code;

    // All values are 8 bytes in this u64-only prototype
    let stack_base = (num_params as u32 + num_locals as u32) * 8;

    // Walk bytecodes to find max stack depth
    let mut depth: i32 = 0;
    let mut max_depth: i32 = 0;
    for bc in bytecodes {
        let (pops, pushes) = stack_effect(bc, module);
        depth -= pops as i32;
        depth += pushes as i32;
        if depth > max_depth {
            max_depth = depth;
        }
    }

    let frame_data_size = stack_base + (max_depth as u32) * 8;
    let callee_base = frame_data_size + FRAME_METADATA_SIZE as u32;

    Ok(FrameLayout {
        num_params,
        num_locals,
        stack_base,
        frame_data_size,
        callee_base,
    })
}

/// Byte offset of a local (param or local variable) in the frame.
fn local_offset(layout: &FrameLayout, local_idx: u16) -> u32 {
    // Params come first, then locals — all 8 bytes each
    debug_assert!(
        (local_idx as u32) < (layout.num_params as u32 + layout.num_locals as u32),
        "local index out of range"
    );
    local_idx as u32 * 8
}

/// Lower a single module to micro-ops.
pub fn lower_module(module: &CompiledModule) -> Result<Vec<LoweredFunction>> {
    let func_id_map = build_func_id_map(module);
    let mut result = Vec::new();
    for func_def in &module.function_defs {
        if func_def.code.is_none() {
            continue;
        }
        let handle = module.function_handle_at(func_def.function);
        let name = module.identifier_at(handle.name).to_string();
        let lowered = lower_function(module, func_def, &func_id_map)?;
        result.push(LoweredFunction {
            name,
            frame_data_size: lowered.0,
            ops: lowered.1,
        });
    }
    Ok(result)
}

/// Lower a single function definition to micro-ops.
/// Returns (frame_data_size, micro_ops).
fn lower_function(
    module: &CompiledModule,
    func_def: &FunctionDefinition,
    func_id_map: &[Option<u32>],
) -> Result<(u32, Vec<MicroOp>)> {
    let layout = compute_frame_layout(module, func_def)?;
    let code = func_def.code.as_ref().unwrap();
    let bytecodes = &code.code;

    let handle = module.function_handle_at(func_def.function);
    let num_returns = module.signature_at(handle.return_).0.len();

    // Pass 2: emit micro-ops
    let mut ops: Vec<MicroOp> = Vec::new();
    let mut stack: Vec<StackEntry> = Vec::new();

    // Mapping from bytecode index to micro-op index for branch fixup
    let mut bc_to_micro: Vec<u32> = Vec::with_capacity(bytecodes.len());

    for bc in bytecodes {
        bc_to_micro.push(ops.len() as u32);

        match bc {
            Bytecode::CopyLoc(l) | Bytecode::MoveLoc(l) => {
                let src_off = local_offset(&layout, *l as u16);
                let dst_off = layout.stack_base + (stack.len() as u32) * 8;
                ops.push(MicroOp::Move8 {
                    dst: FrameOffset(dst_off),
                    src: FrameOffset(src_off),
                });
                stack.push(StackEntry(dst_off));
            },
            Bytecode::StLoc(l) => {
                let entry = stack.pop().expect("stack underflow on StLoc");
                let dst_off = local_offset(&layout, *l as u16);
                ops.push(MicroOp::Move8 {
                    dst: FrameOffset(dst_off),
                    src: FrameOffset(entry.0),
                });
            },
            Bytecode::LdU64(v) => {
                let dst_off = layout.stack_base + (stack.len() as u32) * 8;
                ops.push(MicroOp::StoreImm8 {
                    dst: FrameOffset(dst_off),
                    imm: *v,
                });
                stack.push(StackEntry(dst_off));
            },
            Bytecode::Le => {
                let rhs = stack.pop().expect("stack underflow on Le");
                let lhs = stack.pop().expect("stack underflow on Le");
                // Result goes to the lower of the two freed slots
                let result_off = lhs.0.min(rhs.0);
                ops.push(MicroOp::LeU64 {
                    dst: FrameOffset(result_off),
                    lhs: FrameOffset(lhs.0),
                    rhs: FrameOffset(rhs.0),
                });
                stack.push(StackEntry(result_off));
            },
            Bytecode::BrFalse(target) => {
                let entry = stack.pop().expect("stack underflow on BrFalse");
                // Placeholder — target will be fixed up in pass 3
                ops.push(MicroOp::JumpZeroU64 {
                    target: CodeOffset(*target as u32),
                    src: FrameOffset(entry.0),
                });
            },
            Bytecode::BrTrue(target) => {
                let entry = stack.pop().expect("stack underflow on BrTrue");
                ops.push(MicroOp::JumpNotZeroU64 {
                    target: CodeOffset(*target as u32),
                    src: FrameOffset(entry.0),
                });
            },
            Bytecode::Branch(target) => {
                ops.push(MicroOp::Jump {
                    target: CodeOffset(*target as u32),
                });
            },
            Bytecode::Sub => {
                let rhs = stack.pop().expect("stack underflow on Sub");
                let lhs = stack.pop().expect("stack underflow on Sub");
                let result_off = lhs.0.min(rhs.0);
                ops.push(MicroOp::SubU64 {
                    dst: FrameOffset(result_off),
                    lhs: FrameOffset(lhs.0),
                    rhs: FrameOffset(rhs.0),
                });
                stack.push(StackEntry(result_off));
            },
            Bytecode::Add => {
                let rhs = stack.pop().expect("stack underflow on Add");
                let lhs = stack.pop().expect("stack underflow on Add");
                let result_off = lhs.0.min(rhs.0);
                ops.push(MicroOp::AddU64 {
                    dst: FrameOffset(result_off),
                    lhs: FrameOffset(lhs.0),
                    rhs: FrameOffset(rhs.0),
                });
                stack.push(StackEntry(result_off));
            },
            Bytecode::Call(handle_idx) => {
                emit_call(
                    module,
                    *handle_idx,
                    &layout,
                    func_id_map,
                    &mut stack,
                    &mut ops,
                )?;
            },
            Bytecode::Ret => {
                emit_return(num_returns, &mut stack, &mut ops);
            },
            other => {
                bail!("unsupported bytecode: {:?}", other);
            },
        }
    }

    // Pass 3: branch fixup — resolve bytecode CodeOffset targets to micro-op indices
    for op in &mut ops {
        match op {
            MicroOp::JumpZeroU64 { target, .. }
            | MicroOp::JumpNotZeroU64 { target, .. }
            | MicroOp::Jump { target } => {
                let bc_target = target.0;
                *target = CodeOffset(bc_to_micro[bc_target as usize]);
            },
            _ => {},
        }
    }

    Ok((layout.frame_data_size, ops))
}

fn emit_call(
    module: &CompiledModule,
    handle_idx: FunctionHandleIndex,
    layout: &FrameLayout,
    func_id_map: &[Option<u32>],
    stack: &mut Vec<StackEntry>,
    ops: &mut Vec<MicroOp>,
) -> Result<()> {
    let handle = module.function_handle_at(handle_idx);
    let num_params = module.signature_at(handle.parameters).0.len();
    let num_returns = module.signature_at(handle.return_).0.len();
    let func_id = func_id_map[handle_idx.0 as usize]
        .ok_or_else(|| anyhow::anyhow!("cross-module call not supported"))?;

    // Pop args from stack (they were pushed left-to-right, so pop right-to-left)
    let mut arg_entries: Vec<StackEntry> = Vec::with_capacity(num_params);
    for _ in 0..num_params {
        arg_entries.push(stack.pop().expect("stack underflow on Call args"));
    }
    arg_entries.reverse(); // now in parameter order

    // Copy args to callee frame area
    for (i, entry) in arg_entries.iter().enumerate() {
        let callee_arg_off = layout.callee_base + (i as u32) * 8;
        ops.push(MicroOp::Move8 {
            dst: FrameOffset(callee_arg_off),
            src: FrameOffset(entry.0),
        });
    }

    // Emit call
    ops.push(MicroOp::CallFunc { func_id });

    // Copy return values back from callee frame area
    // Return values overwrite the start of the callee's frame (offset 0 from callee base)
    for i in 0..num_returns {
        let callee_ret_off = layout.callee_base + (i as u32) * 8;
        // Place return values starting at the lowest freed stack slot
        let dst_off = if !arg_entries.is_empty() {
            arg_entries[0].0 + (i as u32) * 8
        } else {
            layout.stack_base + (stack.len() as u32 + i as u32) * 8
        };
        ops.push(MicroOp::Move8 {
            dst: FrameOffset(dst_off),
            src: FrameOffset(callee_ret_off),
        });
        stack.push(StackEntry(dst_off));
    }

    Ok(())
}

fn emit_return(num_returns: usize, stack: &mut Vec<StackEntry>, ops: &mut Vec<MicroOp>) {
    // Copy return values to offset 0+ (beginning of frame)
    let entries: Vec<StackEntry> = stack.drain(stack.len() - num_returns..).collect();
    for (i, entry) in entries.iter().enumerate() {
        let dst_off = (i as u32) * 8;
        ops.push(MicroOp::Move8 {
            dst: FrameOffset(dst_off),
            src: FrameOffset(entry.0),
        });
    }
    ops.push(MicroOp::Return);
}
