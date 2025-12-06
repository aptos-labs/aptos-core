// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Implements the "reaching definition analysis" (RDA) pass
//! Prerequisites:
//! - Variable liveness information is available
//! - Lifetime information is available
//!
//! Description:
//! This analysis provides reaching definition information for temporaries and global resources used in a function.
//! For each bytecode at `code_offset`, it provides a mapping from each local temporary or global resource
//! to the set of code offsets where they may be defined that can reach `code_offset`:
//! - ReachingDefState(code_offset) := Map<Object, Set<CodeOffset>>
//! - Object := Local(temp_index) | Global(struct_id)
//!
//! The analysis is over-approximating, producing may-definitions and may-reaches.
//! A result `ReachingDefState(offset_1) := Map<obj1, Set<offset_2, offset_3>>` means
//! that `obj1` is possibly defined at `offset_2` and `offset_3` and the definitions may reach `offset_1`.
//!
//! ================= for temporaries =================
//! Definitions of temporaries are created at `Assign`, `Load` instructions, and `Call` instructions with destinations.
//! Definitions are killed when the temporary is re-defined, or when it is moved.
//! Definitions via dereferences are handled conservatively
//! - `WriteRef`: all temporaries potentially pointed to by the reference are killed and re-defined
//! - `mut_ref` passed to callees: all temporaries potentially pointed to by the reference are killed and re-defined
//! - `mut_ref` passed to invoked closures: all temporaries potentially pointed to by the reference are killed and re-defined
//!
//! ================ for global resources =================
//! Definitions of global resources are killed and re-defined at the following instructions:
//! - `MoveFrom`
//! - `MoveTo`
//! - Mutation via dereferencing `mut_ref`, conservatively handled like temporaries
//!
//! Callees and invoked closures are handled conservatively:
//! - On `Call` to a child function, the child function and its transitively called/used functions are analyzed to collect the accessed resource structs
//! - On `Invoke` of a closure, the functions used transitively by the current function are analyzed to collect the accessed resource structs
//!
//! The analysis only considers global resources that are declared in the same module of the target function

use crate::{
    bytecode_generator::generate_bytecode,
    pipeline::{
        livevar_analysis_processor::LiveVarAnnotation,
        reference_safety::{LifetimeAnnotation, Object},
    },
};
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    model::{FunId, FunctionEnv, QualifiedId},
    ty::{ReferenceKind, Type},
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AbortAction, AssignKind, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::{BTreeMap, BTreeSet};

type DefMap = BTreeMap<Object, BTreeSet<CodeOffset>>;

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Default)]
pub struct ReachingDefState {
    pub map: DefMap,
}

/// The annotation for reaching definitions. For each code position, we map it
/// to the set of definitions reaching the code position.
#[derive(Clone, Default)]
pub struct ReachingDefAnnotation(BTreeMap<CodeOffset, ReachingDefState>);

impl ReachingDefAnnotation {
    /// Returns information for code offset.
    pub fn get_info_at(&self, code_offset: CodeOffset) -> &ReachingDefState {
        self.0.get(&code_offset).expect("reaching def info")
    }
}

pub struct ReachingDefProcessor {}

impl FunctionTargetProcessor for ReachingDefProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }

        let target = FunctionTarget::new(func_env, &data);
        // reaching definition depends on reference analysis and usage analysis!!!
        let Some(ref_annotation) = target.get_annotations().get::<LifetimeAnnotation>() else {
            return data;
        };
        let Some(livevar_annotation) = target.get_annotations().get::<LiveVarAnnotation>() else {
            return data;
        };
        // init and run the analyzer
        let analyzer = ReachingDefAnalysis {
            _target: target,
            livevar_annotation,
            ref_annotation,
        };
        let cfg = StacklessControlFlowGraph::new_forward(&data.code);
        let block_state_map = analyzer.analyze_function(
            ReachingDefState {
                map: BTreeMap::new(),
            },
            &data.code,
            &cfg,
        );
        let per_bytecode_state =
            analyzer.state_per_instruction(block_state_map, &data.code, &cfg, |before, _| {
                before.clone()
            });

        let annotations = ReachingDefAnnotation(per_bytecode_state);
        data.annotations.set(annotations, true);

        data
    }

    fn name(&self) -> String {
        "ReachingDefProcessor".to_string()
    }
}

impl ReachingDefProcessor {
    /// Registers annotation formatter at the given function target. This is for debugging and
    /// testing.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_reaching_def_annotation))
    }
}

struct ReachingDefAnalysis<'env> {
    _target: FunctionTarget<'env>,
    livevar_annotation: &'env LiveVarAnnotation,
    ref_annotation: &'env LifetimeAnnotation,
}

impl TransferFunctions for ReachingDefAnalysis<'_> {
    type State = ReachingDefState;

    const BACKWARD: bool = false;

    fn execute(&self, state: &mut ReachingDefState, instr: &Bytecode, offset: CodeOffset) {
        use Bytecode::*;
        use Operation::*;

        // helper to collect potential global accesses by the given function
        // - it collects all the resource structs declared in the current module and accessed by the given function (if `include_self` is true)
        // and its transitively used functions (including callees and functions whose values are taken)
        let collect_potential_global_access =
            |qualified_fid: QualifiedId<FunId>, include_self: bool| {
                let global_env = self._target.global_env();
                let module_env = global_env.get_module(qualified_fid.module_id);
                let func_env = module_env.get_function(qualified_fid.id);

                let mut transitive_used_funcs = func_env.get_transitive_closure_of_used_functions();

                if include_self {
                    transitive_used_funcs.insert(qualified_fid);
                }

                // why requiring `child_fid.module_id == self._target.module_env().get_id()`
                // - the analysis focuses on global resources declared in the current module
                // why functions from other modules cannot access global resources in the current module
                // - other modules cannot directly access global resources declared in this module
                // - other modules calling into this module will create cyclic dependencies
                transitive_used_funcs
                    .into_iter()
                    .filter(|child_fid| child_fid.module_id == self._target.module_env().get_id())
                    .flat_map(|child_fid| {
                        generate_bytecode(global_env, child_fid)
                            .code
                            .into_iter()
                            .filter_map(|bytecode| match bytecode {
                                Bytecode::Call(
                                    _,
                                    _,
                                    Operation::BorrowGlobal(mid, sid, _)
                                    | Operation::MoveTo(mid, sid, _)
                                    | Operation::MoveFrom(mid, sid, _),
                                    _,
                                    _,
                                ) => Some(Object::Global(mid.qualified(sid))),
                                _ => None,
                            })
                    })
            };

        match instr {
            Assign(_, dest, src, kind) => {
                state.kill(Object::Local(*dest));
                state.def(Object::Local(*dest), offset);
                // Move semantics invalidate the source, essentially killing it
                match kind {
                    AssignKind::Move => {
                        state.kill(Object::Local(*src));
                    },
                    AssignKind::Inferred => {
                        let lifetime = self.ref_annotation.get_info_at(offset);
                        let live_info = self.livevar_annotation.get_info_at(offset);
                        // If the source temp is not used after this instruction and is not borrowed,
                        // it will be moved, and we need to consider it killed.
                        if !live_info.is_temp_used_after(src, instr) && !lifetime.is_borrowed(*src)
                        {
                            state.kill(Object::Local(*src));
                        }
                    },
                    _ => {},
                }
            },
            Load(_, dest, ..) => {
                state.kill(Object::Local(*dest));
                state.def(Object::Local(*dest), offset);
            },
            Call(_, dests, oper, srcs, on_abort) => {
                // generic kills
                for dest in dests {
                    state.kill(Object::Local(*dest));
                    state.def(Object::Local(*dest), offset);
                }
                if let Some(AbortAction(_, dest)) = on_abort {
                    state.kill(Object::Local(*dest));
                    state.def(Object::Local(*dest), offset);
                }
                // op-specific actions
                match oper {
                    Drop => {
                        state.kill(Object::Local(srcs[0]));
                    },
                    WriteRef => {
                        let ref_info = self.ref_annotation.get_info_at(offset);
                        for obj in ref_info.referenced_objects_before(srcs[0]) {
                            state.kill(obj);
                            state.def(obj, offset);
                        }
                    },
                    // remove an object from global memory
                    MoveFrom(mid, sid, _) => {
                        let obj = Object::Global(mid.qualified(*sid));
                        state.kill(obj);
                        state.def(obj, offset);
                    },
                    // store an object to global memory
                    MoveTo(mid, sid, _) => {
                        let obj = Object::Global(mid.qualified(*sid));
                        state.kill(obj);
                        state.def(obj, offset);
                    },
                    // If a mut ref is passed to a callee, we consider all the object it points to is redefined
                    // Also collect all the global resources transitively accessed by the callee
                    Function(mid, fid, _) => {
                        for src in srcs.iter() {
                            if self._target.get_local_type(*src).is_mutable_reference() {
                                for obj in self
                                    .ref_annotation
                                    .get_info_at(offset)
                                    .referenced_objects_before(*src)
                                {
                                    state.kill(obj);
                                    state.def(obj, offset);
                                }
                            }
                        }
                        collect_potential_global_access(mid.qualified(*fid), true).for_each(
                            |obj| {
                                state.kill(obj);
                                state.def(obj, offset);
                            },
                        );
                    },
                    // If a mut ref is passed to an invoked closure, we consider all the object it points to is redefined
                    // Also collect all the global resources accessed transitively by the invoked closures
                    Invoke => {
                        let fun_type = self
                            ._target
                            .get_local_type(*srcs.last().expect("closure expected"));
                        if let Type::Fun(args_ty, _, _) = fun_type {
                            let ref_info = self.ref_annotation.get_info_at(offset);
                            match args_ty.as_ref() {
                                // A single arg closure
                                Type::Reference(ReferenceKind::Mutable, _) => {
                                    for obj in ref_info.referenced_objects_before(srcs[0]) {
                                        state.kill(obj);
                                        state.def(obj, offset);
                                    }
                                },
                                Type::Tuple(x) => {
                                    assert!(
                                        srcs.len() == x.len() + 1,
                                        "{} arguments expected for invoke",
                                        x.len()
                                    );
                                    for (i, ty) in x.iter().enumerate() {
                                        if ty.is_mutable_reference() {
                                            for obj in ref_info.referenced_objects_before(srcs[i]) {
                                                state.kill(obj);
                                                state.def(obj, offset);
                                            }
                                        }
                                    }
                                },
                                _ => {},
                            }
                        }
                        let qualified_fid = self
                            ._target
                            .module_env()
                            .get_id()
                            .qualified(self._target.get_id());
                        collect_potential_global_access(qualified_fid, false).for_each(|obj| {
                            state.kill(obj);
                            state.def(obj, offset);
                        });
                    },
                    _ => (),
                }
            },
            _ => {},
        }
    }
}

impl DataflowAnalysis for ReachingDefAnalysis<'_> {}

impl AbstractDomain for ReachingDefState {
    fn join(&mut self, other: &Self) -> JoinResult {
        let mut result = JoinResult::Unchanged;
        // why we merge them:
        // - we want to collect a sound set of may-reach definitions
        for (idx, other_defs) in &other.map {
            let defs = self.map.entry(*idx).or_default();
            for d in other_defs {
                if defs.insert(*d) {
                    result = JoinResult::Changed;
                }
            }
        }
        result
    }
}

impl ReachingDefState {
    fn def(&mut self, dest: Object, loc: CodeOffset) {
        // ensure that the previous def is killed
        assert!(!self.map.contains_key(&dest));

        // update the new defs
        self.map.entry(dest).or_default().insert(loc);
    }

    fn kill(&mut self, dest: Object) {
        self.map.remove(&dest);
    }
}

// =================================================================================================
// Formatting

/// Format a reaching definition annotation.
pub fn format_reaching_def_annotation(
    target: &FunctionTarget<'_>,
    code_offset: CodeOffset,
) -> Option<String> {
    if let Some(ReachingDefAnnotation(map)) =
        target.get_annotations().get::<ReachingDefAnnotation>()
    {
        let mut str = String::new();
        str.insert_str(0, &format!("reaching instruction #{}: ", code_offset));
        if let Some(map_at) = map.get(&code_offset) {
            let res = map_at
                .map
                .iter()
                .map(|(idx, defs)| {
                    format!(
                        "`{}` @ {{{}}}",
                        match idx {
                            Object::Local(t) => format!("t{}", t),
                            Object::Global(sid) => format!("Struct {:?}", sid),
                        },
                        defs.iter().map(|def| { format!("{}", def) }).join(", ")
                    )
                })
                .join(", ");
            return Some(str + &res);
        }
    }
    None
}
