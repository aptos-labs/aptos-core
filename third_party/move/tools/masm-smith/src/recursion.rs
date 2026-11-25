// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Makes a function call itself, counting down its first `u64` parameter so the recursion ends.
//!
//! At zero the function returns made-up values; otherwise it calls itself with one less, directly
//! or through a closure of itself, then runs its normal body. Large counts can hit the VM's stack
//! limits, and a call through a closure exercises the VM's reentrancy checks.

use crate::{
    error::Result,
    instructions::{
        imm_instr, is_synthesizable_primitive, label_instr, name_instr, push_synthesized_value,
        simple_op,
    },
    ir::{Block, BlockId, Function, Instr, Operand, QualifiedFunction, Type, Value},
    state::FunctionState,
    types::is_signer_ref,
};
use arbitrary::Unstructured;
use move_core_types::language_storage::ModuleId;

/// Index of the block where `function`'s generated body starts: past the bounded-recursion
/// prologue when it has one, so code added to the body runs once per call level, after the
/// depth check.
pub(crate) fn body_entry_index(function: &Function) -> usize {
    let is_check = |block: &Block| {
        matches!(
            block
                .instrs
                .iter()
                .map(|instr| instr.opcode.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
            ["copy_loc", "ld_u64", "neq", "br_false"]
        )
    };
    let Some(recurse) = function
        .body
        .get(1)
        .filter(|_| function.body.first().is_some_and(is_check))
    else {
        return 0;
    };
    let entry = recurse
        .instrs
        .last()
        .and_then(|instr| match instr.operands.as_slice() {
            [Operand::Label(target)] if instr.opcode == "branch" => Some(*target),
            _ => None,
        });
    entry
        .and_then(|target| function.body.iter().position(|block| block.id == target))
        .unwrap_or(0)
}

/// The depth parameter of `function` if it can be made recursive: its first `u64` parameter,
/// when every parameter can be passed to the recursive call by copy and every return value
/// can be synthesized for the base case.
pub(crate) fn depth_parameter(function: &Function) -> Option<usize> {
    let copyable = |ty: &Type| is_synthesizable_primitive(ty) || is_signer_ref(ty);
    if !function.params.iter().all(|param| copyable(&param.ty))
        || !function.return_types.iter().all(is_synthesizable_primitive)
    {
        return None;
    }
    function
        .params
        .iter()
        .position(|param| matches!(param.ty, Type::U64))
}

/// Most values a recursive step holds on the operand stack across its call. At four per
/// level, a depth above 256 exceeds the 1,024-value operand stack.
const HELD_VALUES_MAX: u8 = 4;

/// How the recursive step calls the function again.
#[derive(Clone, Copy, Debug)]
pub(crate) enum RecursiveCall {
    Direct,
    /// `pack_closure` of the function itself, then `call_closure`.
    Closure,
}

/// Whether `function` supports this generator's closure recursion: it is persistent, has
/// at most four parameters and nonempty returns, and uses unsigned integers, bools, or type
/// parameters in its signature.
pub(crate) fn closure_recursion_ok(function: &Function) -> bool {
    let closure_ty = |ty: &Type| {
        matches!(
            ty,
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256 | Type::Bool
        ) || matches!(ty, Type::TypeParam(index) if usize::from(*index) < function.type_params.len())
    };
    function.attributes.iter().any(|attribute| {
        matches!(
            attribute,
            move_binary_format::file_format::FunctionAttribute::Persistent
        )
    }) && !function.return_types.is_empty()
        && function.params.len() <= 4
        && function.params.iter().all(|param| closure_ty(&param.ty))
        && function.return_types.iter().all(closure_ty)
}

/// Whether and how `function` recurses: with probability [`Config::bounded_recursion_prob`]
/// when it has a depth parameter, through a closure of itself three times in four when its
/// signature allows that.
pub(crate) fn pick_recursion(
    u: &mut Unstructured<'_>,
    cfg: &crate::config::Config,
    function: &Function,
) -> Result<Option<RecursiveCall>> {
    if depth_parameter(function).is_none()
        || !crate::violations::chance(u, cfg.bounded_recursion_prob)?
    {
        return Ok(None);
    }
    Ok(Some(
        if closure_recursion_ok(function) && u.ratio(3u8, 4u8)? {
            RecursiveCall::Closure
        } else {
            RecursiveCall::Direct
        },
    ))
}

/// How `function` recurses, read from its prologue; `None` when it does not.
pub(crate) fn recursive_call(function: &Function) -> Option<RecursiveCall> {
    if body_entry_index(function) == 0 {
        return None;
    }
    let recurse = function.body.get(1)?;
    Some(
        if recurse
            .instrs
            .iter()
            .any(|instr| instr.opcode == "call_closure")
        {
            RecursiveCall::Closure
        } else {
            RecursiveCall::Direct
        },
    )
}

/// Prepends the bounded-recursion prologue to `body`, which is the generated body of
/// `function` in `module`, using `depth` as the depth parameter. Block ids in `body` are kept;
/// the prologue's blocks take fresh ids.
pub(crate) fn prepend_prologue(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    module: &ModuleId,
    function: &Function,
    depth: usize,
    call: RecursiveCall,
    body: &mut Vec<Block>,
) -> Result<()> {
    let Some(entry) = body.first().map(|block| block.id) else {
        return Ok(());
    };
    let next_id = body.iter().map(|block| block.id.0).max().unwrap_or(0) + 1;
    let (check, recurse, base) = (BlockId(next_id), BlockId(next_id + 1), BlockId(next_id + 2));
    let depth_name = function.params[depth].name.clone();

    // Base case: zero depth returns without recursing.
    let check_block = Block {
        id: check,
        instrs: vec![
            name_instr("copy_loc", depth_name.clone()),
            imm_instr("ld_u64", Value::U64(0)),
            simple_op("neq"),
            label_instr("br_false", base),
        ],
    };

    // Recursive call with the depth reduced by one, then the generated body. Values held on
    // the operand stack across the call accumulate over the levels, so deep recursion reaches
    // the operand-stack limit, which is shared by all frames; without them it reaches the
    // call-stack limit instead.
    let held = if u.arbitrary::<bool>()? {
        u.int_in_range(1..=HELD_VALUES_MAX)?
    } else {
        0
    };
    let mut recurse_instrs: Vec<Instr> = (0..held)
        .map(|value| imm_instr("ld_u64", Value::U64(u64::from(value))))
        .collect();
    for (index, param) in function.params.iter().enumerate() {
        recurse_instrs.push(name_instr("copy_loc", param.name.clone()));
        if index == depth {
            recurse_instrs.push(imm_instr("ld_u64", Value::U64(1)));
            recurse_instrs.push(simple_op("sub"));
        }
    }
    let type_args: Vec<Type> = (0..function.type_params.len())
        .map(|index| Type::TypeParam(index as u16))
        .collect();
    let target = QualifiedFunction {
        module: module.clone(),
        name: function.name.clone(),
    };
    match call {
        RecursiveCall::Direct => recurse_instrs.push(Instr {
            opcode: "call".to_string(),
            operands: vec![Operand::Function(target, type_args)],
        }),
        RecursiveCall::Closure => {
            // Nothing captured: the arguments stay on the stack under the closure.
            recurse_instrs.push(Instr {
                opcode: "pack_closure".to_string(),
                operands: vec![
                    Operand::Function(target, type_args),
                    Operand::Imm(Value::U64(0)),
                ],
            });
            recurse_instrs.push(Instr {
                opcode: "call_closure".to_string(),
                operands: vec![Operand::Type(Type::Function {
                    args: function
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect(),
                    results: function.return_types.clone(),
                    abilities: move_core_types::ability::AbilitySet::PUBLIC_FUNCTIONS,
                })],
            });
        },
    }
    recurse_instrs.extend(function.return_types.iter().map(|_| simple_op("pop")));
    recurse_instrs.extend((0..held).map(|_| simple_op("pop")));
    recurse_instrs.push(label_instr("branch", entry));

    // The base case returns values synthesized like any other literal.
    let mut base_instrs = Vec::new();
    for ty in &function.return_types {
        push_synthesized_value(u, state, &mut base_instrs, ty)?;
    }
    for _ in &function.return_types {
        state.pop();
    }
    base_instrs.push(simple_op("ret"));

    body.splice(0..0, [
        check_block,
        Block {
            id: recurse,
            instrs: recurse_instrs,
        },
        Block {
            id: base,
            instrs: base_instrs,
        },
    ]);
    Ok(())
}
