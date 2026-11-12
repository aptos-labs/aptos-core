// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Which spec variables keep their value while code a function value runs executes with a
//! given module on the call stack (`ghost_kept_in`).

use crate::{
    function_target::FunctionData,
    stackless_bytecode::{Bytecode, Operation},
    stackless_bytecode_generator::StacklessBytecodeGenerator,
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use move_model::model::{FunId, GlobalEnv, ModuleId, QualifiedId, StructId};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

/// Whether spec variable `ghost` keeps its value across code unknown code runs while a function
/// of `module` is on the call stack. That code may call any function it can reach, also one of
/// `module`: re-entry blocks no call (`reentrancy_checker.rs`, `enter_function`, outside module
/// lock mode), only access to the re-entered module's resources (`check_resource_access`, for
/// `borrow_global`, `exists`, `move_from`, `move_to`). An updater of `module` that must access
/// one of them on every path to its return aborts before its update takes effect, so the spec
/// variable keeps its value when every updater unknown code can reach is such a function.
pub fn ghost_kept_in(env: &GlobalEnv, ghost: &QualifiedId<StructId>, module: ModuleId) -> bool {
    env.ghost_updaters_reachable_by_unknown_code(ghost)
        .is_some_and(|updaters| {
            updaters
                .iter()
                .all(|f| f.module_id == module && must_access_own_resource(env, *f))
        })
}

/// Per module, its functions every path to a return of which accesses one of its resources,
/// directly or through such a function of it.
#[derive(Default)]
struct MustAccessOwn(RefCell<BTreeMap<ModuleId, BTreeSet<FunId>>>);

fn must_access_own_resource(env: &GlobalEnv, fun: QualifiedId<FunId>) -> bool {
    if !env.has_extension::<MustAccessOwn>() {
        env.set_extension(MustAccessOwn::default());
    }
    let cache = env
        .get_extension::<MustAccessOwn>()
        .expect("extension set above");
    if let Some(funs) = cache.0.borrow().get(&fun.module_id) {
        return funs.contains(&fun.id);
    }
    let module = env.get_module(fun.module_id);
    let code: Vec<(FunId, FunctionData)> = module
        .get_functions()
        .filter(|f| !f.is_native() && !f.is_inline() && !f.is_lemma() && f.get_bytecode().is_some())
        .map(|f| {
            (
                f.get_id(),
                StacklessBytecodeGenerator::new(&f).generate_function(),
            )
        })
        .collect();
    // A greatest fixpoint: start from every function and drop those with a path that does not.
    let mut must: BTreeSet<FunId> = code.iter().map(|(id, _)| *id).collect();
    loop {
        let next: BTreeSet<FunId> = code
            .iter()
            .filter(|(_, data)| every_path_accesses(&data.code, fun.module_id, &must))
            .map(|(id, _)| *id)
            .collect();
        if next == must {
            break;
        }
        must = next;
    }
    let result = must.contains(&fun.id);
    cache.0.borrow_mut().insert(fun.module_id, must);
    result
}

/// Whether every path through `code` to a return accesses a resource of `module` or calls one
/// of its functions in `must`.
fn every_path_accesses(code: &[Bytecode], module: ModuleId, must: &BTreeSet<FunId>) -> bool {
    let accesses = |bc: &Bytecode| match bc {
        Bytecode::Call(_, _, op, _, _) => match op {
            Operation::MoveTo(mid, ..)
            | Operation::MoveFrom(mid, ..)
            | Operation::Exists(mid, ..)
            | Operation::BorrowGlobal(mid, ..)
            | Operation::GetGlobal(mid, ..) => *mid == module,
            Operation::Function(mid, fid, _) => *mid == module && must.contains(fid),
            _ => false,
        },
        _ => false,
    };
    let cfg = StacklessControlFlowGraph::new_forward(code);
    // Per block, whether every path to its start has accessed; unvisited blocks are `true`.
    let mut before: BTreeMap<BlockId, bool> = BTreeMap::from([(cfg.entry_block(), false)]);
    let mut todo = vec![cfg.entry_block()];
    while let Some(block) = todo.pop() {
        let mut accessed = before[&block];
        for offset in cfg.instr_indexes(block).into_iter().flatten() {
            let bc = &code[offset as usize];
            accessed |= accesses(bc);
            if matches!(bc, Bytecode::Ret(..)) && !accessed {
                return false;
            }
        }
        for succ in cfg.successors(block) {
            let joined = before.get(succ).map_or(accessed, |b| *b && accessed);
            if before.get(succ) != Some(&joined) {
                before.insert(*succ, joined);
                todo.push(*succ);
            }
        }
    }
    true
}
