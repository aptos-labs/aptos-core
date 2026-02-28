// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Reaching Definition Analysis (RDA) pass for Move stackless bytecode.
//!
//! # Prerequisites
//!
//! - `LiveVarAnnotation` (from livevar analysis)
//! - `LifetimeAnnotation` (from reference safety analysis)
//!
//! # Overview
//!
//! This analysis computes reaching definition information for temporaries and global
//! resources within a function. For each bytecode at a given `code_offset`, it provides
//! a mapping from each local temporary or global resource to the set of code offsets
//! where they may be defined and can reach `code_offset`:
//!
//! ```text
//! ReachingDefState(code_offset) := Map<Object, Set<CodeOffset>>
//! Object := Local(TempIndex) | Global(QualifiedId<StructId>)
//! ```
//!
//! The analysis is a may-analysis (over-approximating). A result like
//! `ReachingDefState(offset_1) := {obj1 -> {offset_2, offset_3}}` means that `obj1`
//! is possibly defined at `offset_2` and `offset_3`, and those definitions may reach `offset_1`.
//!
//! # Temporaries
//!
//! Definitions of temporaries are created at:
//! - `Assign` and `Load` instructions
//! - `Call` instructions (for destination operands)
//!
//! Definitions are killed when the temporary is re-defined or moved.
//!
//! Definitions via dereferences are handled conservatively:
//! - `WriteRef`: all temporaries potentially pointed to by the reference are killed and re-defined
//! - Mutable reference passed to callees: all temporaries it may point to are killed and re-defined
//! - Mutable reference passed to invoked closures: all temporaries it may point to are killed and re-defined
//!
//! # Global Resources
//!
//! Definitions of global resources are killed and re-defined at:
//! - `MoveFrom`
//! - `MoveTo`
//! - Mutation via mutable reference dereference (handled conservatively, same as temporaries)
//!
//! Callees and invoked closures are handled conservatively:
//! - On `Call`: the callee and its transitively used functions are analyzed to collect accessed resources
//! - On `Invoke`: all resources in the current module are assumed to be potentially modified,
//!   since the closure target is unknown at compile time (it could come from function parameters,
//!   global storage, or other indirect sources)
//!
//! **Note**: Only global resources declared in the same module as the target function are considered.

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
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

type DefMap = BTreeMap<Object, BTreeSet<CodeOffset>>;

/// The reaching definition state at a particular code offset.
///
/// Maps each object (local temporary or global resource) to the set of code offsets
/// where that object may have been defined and those definitions can reach this point.
///
/// This is a may-analysis: the result over-approximates the actual reaching definitions.
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Default)]
pub struct ReachingDefState {
    pub map: DefMap,
}

/// The annotation for reaching definitions produced by `ReachingDefProcessor`.
///
/// For each code position, maps it to the `ReachingDefState` representing which
/// definitions may reach that position.
///
/// # Limitations
///
/// **Global resources**: Only global resources declared in the same module as the
/// analyzed function are tracked. Global resources declared in other modules are
/// not included in the analysis.
///
/// # Example
///
/// For code like:
/// ```move
/// let x = 1;      // offset 0: defines x
/// let y = 2;      // offset 1: defines y
/// if (cond) {
///     x = 3;      // offset 3: defines x
/// }
/// use(x);         // offset 5: x has reaching defs from {0, 3}
/// ```
///
/// At offset 5, `get_info_at(5).map[Local(x)]` would be `{0, 3}`.
#[derive(Clone, Default)]
pub struct ReachingDefAnnotation(BTreeMap<CodeOffset, ReachingDefState>);

impl ReachingDefAnnotation {
    /// Returns the reaching definition information at the given code offset.
    ///
    /// # Panics
    ///
    /// Panics if no information exists for the given offset (indicates the
    /// analysis was not run or the offset is invalid).
    pub fn get_info_at(&self, code_offset: CodeOffset) -> &ReachingDefState {
        self.0.get(&code_offset).expect("reaching def info")
    }
}

/// A processor that computes reaching definition analysis for Move stackless bytecode.
///
/// This processor analyzes which definitions of variables and global resources may
/// reach each program point. It produces a `ReachingDefAnnotation` that can be
/// queried by subsequent pipeline stages.
///
/// # Prerequisites
///
/// This processor requires the following annotations to be present:
/// - `LiveVarAnnotation` from `LiveVarAnalysisProcessor`
/// - `LifetimeAnnotation` from reference safety analysis
///
/// If these annotations are missing, the processor returns the function data unchanged.
///
/// # See Also
///
/// - `ReachingDefAnnotation` for the output format and limitations
/// - `ReachingDefState` for the per-offset state
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
        // Reaching definition analysis requires lifetime and livevar annotations
        let Some(ref_annotation) = target.get_annotations().get::<LifetimeAnnotation>() else {
            return data;
        };
        let Some(livevar_annotation) = target.get_annotations().get::<LiveVarAnnotation>() else {
            return data;
        };
        // Collect all resource structs (structs with `key` ability) in the module
        let module_env = target.module_env();
        let all_module_resources: BTreeSet<Object> = module_env
            .get_structs()
            .filter(|s| s.has_memory()) // has_memory() checks for `key` ability
            .map(|s| Object::Global(module_env.get_id().qualified(s.get_id())))
            .collect();

        // init and run the analyzer
        let analyzer = ReachingDefAnalysis {
            target,
            livevar_annotation,
            ref_annotation,
            transitive_global_access: RefCell::new(BTreeMap::new()),
            direct_global_access: RefCell::new(BTreeMap::new()),
            all_module_resources,
        };
        let cfg = StacklessControlFlowGraph::new_forward(&data.code);
        let block_state_map = analyzer.analyze_function(
            ReachingDefState {
                map: BTreeMap::new(),
            },
            &data.code,
            &cfg,
        );
        let per_bytecode_state = analyzer.state_per_instruction_with_default(
            block_state_map,
            &data.code,
            &cfg,
            |before, _| before.clone(),
        );

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
    target: FunctionTarget<'env>,
    livevar_annotation: &'env LiveVarAnnotation,
    ref_annotation: &'env LifetimeAnnotation,
    /// Cache: function ID -> global resources accessed by the function and its transitive callees.
    transitive_global_access: RefCell<BTreeMap<QualifiedId<FunId>, BTreeSet<Object>>>,
    /// Cache: function ID -> global resources directly accessed by the function (not including callees).
    direct_global_access: RefCell<BTreeMap<QualifiedId<FunId>, BTreeSet<Object>>>,
    /// All resource structs (structs with `key` ability) in the current module.
    /// Computed once at initialization and used for conservative analysis.
    all_module_resources: BTreeSet<Object>,
}

impl ReachingDefAnalysis<'_> {
    /// Gets global resources accessed by the given function and its transitive callees.
    /// Results are cached to avoid recomputation.
    fn get_transitive_global_access(&self, qualified_fid: QualifiedId<FunId>) -> BTreeSet<Object> {
        if let Some(cached) = self.transitive_global_access.borrow().get(&qualified_fid) {
            return cached.clone();
        }

        let global_env = self.target.global_env();
        let module_env = global_env.get_module(qualified_fid.module_id);
        let func_env = module_env.get_function(qualified_fid.id);

        let mut transitive_used_funcs = func_env.get_transitive_closure_of_used_functions();
        transitive_used_funcs.insert(qualified_fid);

        let target_mid = self.target.module_env().get_id();
        let mut global_resources = BTreeSet::new();

        for child_fid in transitive_used_funcs.iter() {
            // Skip functions from external modules because they cannot access
            // global resources declared in the current module:
            // - External modules cannot directly access this module's resources
            // - If they called back into this module, it would create a cyclic dependency
            if child_fid.module_id != target_mid {
                continue;
            }
            global_resources.extend(self.get_direct_global_access(*child_fid));
        }

        self.transitive_global_access
            .borrow_mut()
            .insert(qualified_fid, global_resources.clone());
        global_resources
    }

    /// Gets the global resources directly accessed by a single function (not including callees).
    /// Results are cached to avoid re-analyzing the same function's bytecode.
    fn get_direct_global_access(&self, qualified_fid: QualifiedId<FunId>) -> BTreeSet<Object> {
        if let Some(cached) = self.direct_global_access.borrow().get(&qualified_fid) {
            return cached.clone();
        }

        let global_env = self.target.global_env();
        let func_env = global_env.get_function(qualified_fid);

        // Native functions do not access global resources
        // Inline functions are expanded at call sites
        if func_env.is_native() || func_env.is_inline() {
            return BTreeSet::new();
        }

        let func_data = generate_bytecode(global_env, qualified_fid);
        let mut resources = BTreeSet::new();

        for bytecode in func_data.code.iter() {
            match bytecode {
                Bytecode::Call(
                    _,
                    _,
                    Operation::MoveTo(mid, sid, _) | Operation::MoveFrom(mid, sid, _),
                    _,
                    _,
                ) => {
                    resources.insert(Object::Global(mid.qualified(*sid)));
                },
                // Mutable borrow gives the function the capability to modify the global.
                // Since we don't analyze the code for actual WriteRef usage,
                // we conservatively assume any mutable borrow may modify the resource.
                Bytecode::Call(_, dests, Operation::BorrowGlobal(mid, sid, _), _, _)
                    if !dests.is_empty()
                        && func_data.local_types[dests[0]].is_mutable_reference() =>
                {
                    resources.insert(Object::Global(mid.qualified(*sid)));
                },
                // If the function invokes a closure, conservatively assume all module
                // resources could be modified (closure target is unknown).
                Bytecode::Call(_, _, Operation::Invoke, _, _) => {
                    resources.extend(self.all_module_resources.clone());
                },
                _ => {},
            }
        }

        self.direct_global_access
            .borrow_mut()
            .insert(qualified_fid, resources.clone());
        resources
    }
}

impl TransferFunctions for ReachingDefAnalysis<'_> {
    type State = ReachingDefState;

    const BACKWARD: bool = false;

    fn execute(&self, state: &mut ReachingDefState, instr: &Bytecode, offset: CodeOffset) {
        use Bytecode::*;
        use Operation::*;

        match instr {
            Assign(_, dest, src, kind) => {
                // Move semantics invalidate the source, essentially killing it
                match kind {
                    AssignKind::Copy => {
                        state.kill(Object::Local(*dest));
                        state.def(Object::Local(*dest), offset);
                    },
                    AssignKind::Move => {
                        state.kill(Object::Local(*dest));
                        state.def(Object::Local(*dest), offset);
                        state.kill(Object::Local(*src));
                    },
                    AssignKind::Inferred => {
                        state.kill(Object::Local(*dest));
                        state.def(Object::Local(*dest), offset);

                        let lifetime = self.ref_annotation.get_info_at(offset);
                        let live_info = self.livevar_annotation.get_info_at(offset);
                        // If the source temp is not used after this instruction and is not borrowed,
                        // it will be moved, and we need to consider it killed.
                        if !live_info.is_temp_used_after(src, instr) && !lifetime.is_borrowed(*src)
                        {
                            state.kill(Object::Local(*src));
                        }
                    },
                    AssignKind::Store => {
                        state.kill(Object::Local(*dest));
                        state.def(Object::Local(*dest), offset);
                    },
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
                    // MoveFrom removes the resource from global storage.
                    // This is a redefinition point: the resource state changes from
                    // "exists with value" to "does not exist".
                    MoveFrom(mid, sid, _) => {
                        let obj = Object::Global(mid.qualified(*sid));
                        state.kill(obj);
                        state.def(obj, offset);
                    },
                    // MoveTo stores a new value to global storage.
                    // This is a redefinition point: the resource state changes to
                    // "exists with new value".
                    MoveTo(mid, sid, _) => {
                        let obj = Object::Global(mid.qualified(*sid));
                        state.kill(obj);
                        state.def(obj, offset);
                    },
                    // If a mut ref is passed to a callee, we consider all the objects it points to is redefined
                    // Also collect all the global resources transitively accessed by the callee
                    Function(mid, fid, _) => {
                        for src in srcs.iter() {
                            if self.target.get_local_type(*src).is_mutable_reference() {
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
                        for obj in self.get_transitive_global_access(mid.qualified(*fid)) {
                            state.kill(obj);
                            state.def(obj, offset);
                        }
                    },
                    // Handle closure invocation conservatively:
                    // 1. Mutable refs passed as arguments: redefine all objects they may point to
                    // 2. Global resources: we conservatively assume the invoked closure could
                    //    modify ANY resource in the current module. This is necessary because
                    //    the closure's target function is generally unknown at compile time -
                    //    it could come from function parameters, global storage, or other
                    //    indirect sources. Without full alias/pointer analysis, we cannot
                    //    determine which specific function will be invoked.
                    Invoke => {
                        let fun_type = self
                            .target
                            .get_local_type(*srcs.last().expect("closure expected"));
                        if let Type::Fun(args_ty, _, _) = fun_type {
                            let ref_info = self.ref_annotation.get_info_at(offset);
                            match args_ty.as_ref() {
                                // A single arg closure
                                Type::Reference(ReferenceKind::Mutable, _) => {
                                    assert!(srcs.len() == 2, "one argument expected for invoke");
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
                        // Conservatively assume any module resource could be modified
                        for obj in self.all_module_resources.clone() {
                            state.kill(obj);
                            state.def(obj, offset);
                        }
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
                        defs.iter().map(|def| format!("{}", def)).join(", ")
                    )
                })
                .join(", ");
            return Some(format!("reaching instruction #{}: {}", code_offset, res));
        }
    }
    None
}
