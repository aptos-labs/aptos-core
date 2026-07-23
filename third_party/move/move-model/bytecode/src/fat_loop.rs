// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Analysis to determine the 'fat loops' in a function, optionally collecting loop
//! invariants and information for loop unrolling if run in specification mode.
//!
//! A fat loop captures the information of one or more natural loops that share the same loop
//! header. Conceptually, every back edge in the fat loop defines a unique natural loop and
//! different back edges may point to the same loop header (e.g., when there are two
//! "continue" statements in the loop body).
//!
//! Since these natural loops share the same loop header, they share the same loop
//! invariants too and the fat-loop targets (i.e., variables that may be changed in any sub-loop)
//! is the union of loop targets per each natural loop that share the header.

use crate::{
    function_target::FunctionTarget,
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant},
    graph::{Graph, NaturalLoop},
    stackless_bytecode::{AttrId, BorrowNode, Bytecode, Label, Operation, PropKind},
    stackless_control_flow_graph::{BlockContent, BlockId, StacklessControlFlowGraph},
    usage_analysis,
};
use anyhow::bail;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast,
    ast::TempIndex,
    model::{Parameter, QualifiedInstId, StructId},
    pragmas::UNROLL_PRAGMA,
    ty::Type,
};
use std::collections::{BTreeMap, BTreeSet};

/// Representation of a fat loop.
#[derive(Debug, Clone)]
pub struct FatLoop {
    /// The code offsets from which back edges point to this loop.
    pub back_edges: BTreeSet<CodeOffset>,

    /// If fat loops are computed in spec mode, additional info for specs.
    pub spec_info: Option<FatLoopSpecInfo>,
}

/// Specification related information for a fat loop
#[derive(Debug, Clone)]
pub struct FatLoopSpecInfo {
    /// The loop invariants associated with a code offset. The range is the related
    /// `Prop(attr_id, _, exp)` statement.
    pub invariants: BTreeMap<CodeOffset, (AttrId, ast::Exp)>,

    /// The temporaries which are modified in the loop, and which are immutable
    /// references or values. See also function `Bytecode::modifies`.
    pub val_targets: BTreeSet<TempIndex>,

    /// The temporaries which are modified in the loop, and which are mutable
    /// references. The boolean indicates whether the reference itself is modified, and is
    /// false if only the value it points to is. See also function `Bytecode::modifies`.
    pub mut_targets: BTreeMap<TempIndex, bool>,

    /// The global memories which may be modified in the loop, directly (resource
    /// operations, write-backs to global roots) or transitively through calls.
    /// Loop-to-DAG transformation havocs these at the loop header, so the
    /// loop-exit path does not retain pre-loop memory.
    pub mem_targets: BTreeSet<QualifiedInstId<StructId>>,
}

/// Information about fat loops in a function.
#[derive(Debug, Clone)]
pub struct FatLoopFunctionInfo {
    /// If at the label is a header of a fat loop, it will be in the below map.
    pub fat_loops: BTreeMap<Label, FatLoop>,
}

/// Marker for loop unrolling, in specification mode.
#[derive(Debug, Clone)]
pub struct LoopUnrollingMark {
    pub marker: Option<AttrId>,
    pub loop_body: Vec<Vec<Bytecode>>,
    pub back_edges: BTreeSet<CodeOffset>,
    pub iter_count: usize,
}

/// Information about loop unrolling, in specification mode.
#[derive(Debug, Clone)]
pub struct LoopUnrollingFunctionInfo {
    /// If a label is a header of an unrolled loop, it will be in this map.
    pub fat_loops: BTreeMap<Label, LoopUnrollingMark>,
}

impl FatLoop {
    /// Assert spec info is available for the fat loop and return it.
    pub fn spec_info(&self) -> &FatLoopSpecInfo {
        self.spec_info.as_ref().expect("spec info available")
    }
}

impl FatLoopFunctionInfo {
    /// Get all code offsets which have back edges.
    pub fn back_edges_locations(&self) -> BTreeSet<CodeOffset> {
        self.fat_loops
            .values()
            .flat_map(|l| l.back_edges.iter())
            .copied()
            .collect()
    }

    /// Get all code offsets which have invariants.
    pub fn invariants_locations(&self) -> BTreeSet<CodeOffset> {
        self.fat_loops
            .values()
            .flat_map(|l| l.spec_info().invariants.keys())
            .copied()
            .collect()
    }
}

/// Find all fat loops in the function.
pub fn build_loop_info(func_target: &FunctionTarget) -> anyhow::Result<FatLoopFunctionInfo> {
    FatLoopBuilder {
        for_spec: false,
        targets: None,
    }
    .build_loop_info(func_target)
    .map(|(info, _)| info)
}

/// Find all fat loops in the function and collect information needed for invariant instrumentation
/// (i.e., loop-to-DAG transformation) and loop unrolling (if requested by user). The targets
/// holder is used to look up the (transitively) modified memory of functions called in loop
/// bodies.
pub fn build_loop_info_for_spec(
    func_target: &FunctionTarget,
    targets: &FunctionTargetsHolder,
) -> anyhow::Result<(FatLoopFunctionInfo, LoopUnrollingFunctionInfo)> {
    FatLoopBuilder {
        for_spec: true,
        targets: Some(targets),
    }
    .build_loop_info(func_target)
}

struct FatLoopBuilder<'a> {
    for_spec: bool,
    targets: Option<&'a FunctionTargetsHolder>,
}

impl FatLoopBuilder<'_> {
    fn build_loop_info(
        &self,
        func_target: &FunctionTarget,
    ) -> anyhow::Result<(FatLoopFunctionInfo, LoopUnrollingFunctionInfo)> {
        // build for natural loops
        let env = func_target.global_env();
        let code = func_target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let entry = cfg.entry_block();
        let nodes = cfg.blocks();
        let edges: Vec<(BlockId, BlockId)> = nodes
            .iter()
            .flat_map(|x| {
                cfg.successors(*x)
                    .iter()
                    .map(|y| (*x, *y))
                    .collect::<Vec<(BlockId, BlockId)>>()
            })
            .collect();
        let graph = Graph::new(entry, nodes, edges);
        let Some(natural_loops) = graph.compute_reducible() else {
            bail!("well-formed Move function expected to have a reducible control-flow graph")
        };
        let unroll_pragma = func_target.func_env.get_num_pragma(UNROLL_PRAGMA);

        // collect shared headers from loops
        let mut fat_headers = BTreeMap::new();
        for single_loop in natural_loops {
            fat_headers
                .entry(single_loop.loop_header)
                .or_insert_with(Vec::new)
                .push(single_loop);
        }

        // build fat loops by label
        let mut fat_loops_for_unrolling = BTreeMap::new();
        let mut fat_loops = BTreeMap::new();
        for (fat_root, sub_loops) in fat_headers {
            // get the label of the scc root
            let label = match cfg.content(fat_root) {
                BlockContent::Dummy => panic!("A loop header should never be a dummy block"),
                BlockContent::Basic { lower, upper: _ } => match code[*lower as usize] {
                    Bytecode::Label(_, label) => label,
                    _ => panic!("A loop header block is expected to start with a Label bytecode"),
                },
            };
            let (invariants, unrolling_mark) = if self.for_spec {
                (
                    self.collect_loop_invariants(&cfg, func_target, fat_root),
                    self.probe_loop_unrolling_mark(&cfg, func_target, fat_root)
                        .map(|(marker, count)| (Some(marker), count))
                        .or_else(|| unroll_pragma.map(|count| (None, count))),
                )
            } else {
                (BTreeMap::default(), None)
            };
            let back_edges = self.collect_loop_back_edges(code, &cfg, label, &sub_loops);

            // loop invariants and unrolling should be mutual exclusive
            match unrolling_mark {
                None => {
                    // no spec mode, or loop invariant instrumentation route
                    let spec_info = if self.for_spec {
                        let (val_targets, mut_targets) =
                            self.collect_loop_targets(&cfg, func_target, &sub_loops);
                        let mem_targets = self.collect_loop_memory(&cfg, func_target, &sub_loops);
                        Some(FatLoopSpecInfo {
                            invariants,
                            val_targets,
                            mut_targets,
                            mem_targets,
                        })
                    } else {
                        None
                    };
                    fat_loops.insert(label, FatLoop {
                        back_edges,
                        spec_info,
                    });
                },
                Some((attr_id, count)) => {
                    if !invariants.is_empty() {
                        let error_loc = attr_id.map_or_else(
                            || env.unknown_loc(),
                            |attr_id| func_target.get_bytecode_loc(attr_id),
                        );
                        env.error(
                            &error_loc,
                            "loop invariants and loop unrolling is mutual exclusive",
                        );
                    }
                    // loop unrolling route
                    let loop_body = self.collect_loop_body_bytecode(code, &cfg, &sub_loops);
                    fat_loops_for_unrolling.insert(label, LoopUnrollingMark {
                        marker: attr_id,
                        loop_body,
                        back_edges,
                        iter_count: count,
                    });
                },
            }
        }

        if self.for_spec {
            // check for redundant loop invariant declarations in the spec
            let all_invariants: BTreeSet<_> = fat_loops
                .values()
                .flat_map(|l| {
                    l.spec_info()
                        .invariants
                        .values()
                        .map(|(attr_id, _)| *attr_id)
                })
                .collect();
            for attr_id in func_target.data.loop_invariants.difference(&all_invariants) {
                env.error(
                    &func_target.get_bytecode_loc(*attr_id),
                    "Loop invariants must be declared at the beginning of the loop header in a \
                consecutive sequence",
                );
            }

            // check for redundant loop unrolling marks in the spec
            let all_unrolling_marks: BTreeSet<_> = fat_loops_for_unrolling
                .values()
                .filter_map(|l| l.marker)
                .collect();
            let declared_unrolling_marks: BTreeSet<_> =
                func_target.data.loop_unrolling.keys().copied().collect();
            for attr_id in declared_unrolling_marks.difference(&all_unrolling_marks) {
                env.error(
                    &func_target.get_bytecode_loc(*attr_id),
                    "Loop unrolling mark must be declared at the beginning of the loop header",
                );
            }
        }

        // done with information collection
        Ok((
            FatLoopFunctionInfo { fat_loops },
            LoopUnrollingFunctionInfo {
                fat_loops: fat_loops_for_unrolling,
            },
        ))
    }

    /// Collect invariants in the given loop header block
    ///
    /// Loop invariants are defined as
    /// 1) the longest sequence of consecutive
    /// 2) `PropKind::Assert` propositions
    /// 3) in the loop header block, immediately after the `Label` statement,
    /// 4) which are also marked in the `loop_invariants` field in the `FunctionData`.
    /// All above conditions must be met to be qualified as a loop invariant.
    ///
    /// The reason we piggyback on `PropKind::Assert` instead of introducing a new
    /// `PropKind::Invariant` is that we don't want to introduce a`PropKind::Invariant` type which
    /// only exists to be eliminated. The same logic applies for other invariants in the system
    /// (e.g., data invariants, global invariants, etc).
    ///
    /// In other words, for the loop header block:
    /// - the first statement must be a `label`,
    /// - followed by N `assert` statements, N >= 0
    /// - all these N `assert` statements are marked as loop invariants,
    /// - statement N + 1 is either not an `assert` or is not marked in `loop_invariants`.
    fn collect_loop_invariants(
        &self,
        cfg: &StacklessControlFlowGraph,
        func_target: &FunctionTarget<'_>,
        loop_header: BlockId,
    ) -> BTreeMap<CodeOffset, (AttrId, ast::Exp)> {
        let code = func_target.get_bytecode();
        let asserts_as_invariants = &func_target.data.loop_invariants;

        let mut invariants = BTreeMap::new();
        for (index, code_offset) in cfg.instr_indexes(loop_header).unwrap().enumerate() {
            let bytecode = &code[code_offset as usize];
            if index == 0 {
                assert!(matches!(bytecode, Bytecode::Label(_, _)));
            } else {
                match bytecode {
                    Bytecode::Prop(attr_id, PropKind::Assert, exp)
                        if asserts_as_invariants.contains(attr_id) =>
                    {
                        invariants.insert(code_offset, (*attr_id, exp.clone()));
                    },
                    _ => break,
                }
            }
        }
        invariants
    }

    /// Collect loop unrolling instruction in the given loop header block
    ///
    /// A loop unrolling instruction defined as
    /// - an `assume true;`
    /// - in the loop header block, immediately after the `Label` statement,
    /// - with its `attr_id` marked in the `loop_unrolling` field in the `FunctionData`
    fn probe_loop_unrolling_mark(
        &self,
        cfg: &StacklessControlFlowGraph,
        func_target: &FunctionTarget<'_>,
        loop_header: BlockId,
    ) -> Option<(AttrId, usize)> {
        let code = func_target.get_bytecode();
        let assumes_as_unrolling_marks = &func_target.data.loop_unrolling;

        let mut marks = BTreeMap::new();
        for (index, code_offset) in cfg.instr_indexes(loop_header).unwrap().enumerate() {
            let bytecode = &code[code_offset as usize];
            if index == 0 {
                assert!(matches!(bytecode, Bytecode::Label(_, _)));
            } else {
                match bytecode {
                    Bytecode::Prop(attr_id, PropKind::Assume, _) => {
                        match assumes_as_unrolling_marks.get(attr_id) {
                            None => {
                                break;
                            },
                            Some(count) => {
                                marks.insert(code_offset, (*attr_id, *count));
                            },
                        }
                    },

                    _ => break,
                }
            }
        }

        // check that there is at most one unrolling mark
        let env = func_target.global_env();
        if marks.len() > 1 {
            for (attr_id, _) in marks.values() {
                env.error(
                    &func_target.get_bytecode_loc(*attr_id),
                    "Loop unrolling mark can only be specified once per loop",
                );
            }
        }
        marks
            .into_iter()
            .next()
            .map(|(_, (attr_id, count))| (attr_id, count))
    }

    /// Collect variables that may be changed during the loop execution.
    ///
    /// The input to this function should include all the sub loops that constitute a fat-loop.
    /// This function will return two sets of variables that represents, respectively,
    /// - the set of values to be havoc-ed, and
    /// - the set of mutations to be havoc-ed and how they should be havoc-ed.
    fn collect_loop_targets(
        &self,
        cfg: &StacklessControlFlowGraph,
        func_target: &FunctionTarget<'_>,
        sub_loops: &[NaturalLoop<BlockId>],
    ) -> (BTreeSet<TempIndex>, BTreeMap<TempIndex, bool>) {
        let code = func_target.get_bytecode();
        let mut val_targets = BTreeSet::new();
        let mut mut_targets = BTreeMap::new();
        let fat_loop_body: BTreeSet<_> = sub_loops
            .iter()
            .flat_map(|l| l.loop_body.iter())
            .copied()
            .collect();
        for block_id in fat_loop_body {
            for code_offset in cfg
                .instr_indexes(block_id)
                .expect("A loop body should never contain a dummy block")
            {
                let bytecode = &code[code_offset as usize];
                let (bc_val_targets, bc_mut_targets) = bytecode.modifies(func_target);
                val_targets.extend(bc_val_targets);
                for (idx, is_full_havoc) in bc_mut_targets {
                    mut_targets
                        .entry(idx)
                        .and_modify(|v| {
                            *v = *v || is_full_havoc;
                        })
                        .or_insert(is_full_havoc);
                }
            }
        }
        (val_targets, mut_targets)
    }

    /// Collect global memories that may be modified during loop execution:
    /// directly through resource operations and write-backs to global roots,
    /// or transitively through called functions (per their usage analysis).
    /// Intrinsic memory is skipped — it is not modeled as regular global
    /// memory. Requires the targets holder; returns empty without one.
    fn collect_loop_memory(
        &self,
        cfg: &StacklessControlFlowGraph,
        func_target: &FunctionTarget<'_>,
        sub_loops: &[NaturalLoop<BlockId>],
    ) -> BTreeSet<QualifiedInstId<StructId>> {
        let mut mems = BTreeSet::new();
        let Some(targets) = self.targets else {
            return mems;
        };
        let env = func_target.global_env();
        let code = func_target.get_bytecode();
        let mut add = |mem: QualifiedInstId<StructId>| {
            if !env.get_struct_qid(mem.to_qualified_id()).is_intrinsic() {
                mems.insert(mem);
            }
        };
        let fat_loop_body: BTreeSet<_> = sub_loops
            .iter()
            .flat_map(|l| l.loop_body.iter())
            .copied()
            .collect();
        for block_id in fat_loop_body {
            for code_offset in cfg
                .instr_indexes(block_id)
                .expect("A loop body should never contain a dummy block")
            {
                // Inline spec blocks that update ghost memory are lowered to
                // `Prop(Assume, exp)` with `exp.node_id()` recorded in
                // `spec.update_map` (see `stackless_bytecode_generator.rs`
                // for the emission, and `spec_instrumentation.rs`'s use of
                // the same lookup). This is offset-space-correct at loop
                // analysis time; `spec.on_impl` is keyed by file-format
                // offsets and does not align with the stackless code offsets
                // used here.
                if let Bytecode::Prop(_, PropKind::Assume, exp) = &code[code_offset as usize] {
                    if let Some(cond) = func_target.get_spec().update_map.get(&exp.node_id()) {
                        if !cond.additional_exps.is_empty() {
                            if let Some((mem, _, _)) =
                                cond.additional_exps[0].extract_ghost_mem_access(env)
                            {
                                add(mem);
                            }
                        }
                    }
                }
                let Bytecode::Call(_, _, oper, srcs, _) = &code[code_offset as usize] else {
                    continue;
                };
                match oper {
                    Operation::MoveTo(mid, sid, inst) | Operation::MoveFrom(mid, sid, inst) => {
                        add(mid.qualified_inst(*sid, inst.clone()));
                    },
                    Operation::WriteBack(BorrowNode::GlobalRoot(mem), _) => {
                        add(mem.clone());
                    },
                    Operation::Function(mid, fid, inst) => {
                        let callee = env.get_function(mid.qualified(*fid));
                        if callee.is_native() || callee.is_intrinsic() || callee.is_struct_api() {
                            continue;
                        }
                        // The callee's `invoke_frame` carries `modifies_of`
                        // frame memory transitively reachable through
                        // function-value invocations in the call subtree —
                        // effects that are not in the callee's direct or
                        // through-call `modified` summary.
                        let usage = if callee.get_qualified_id()
                            == func_target.func_env.get_qualified_id()
                        {
                            // Recursive call: the function under processing is
                            // held out of the targets holder; its own usage
                            // summary is fixpointed and includes recursive
                            // effects.
                            usage_analysis::get_memory_usage(func_target)
                        } else {
                            if !targets.has_target(&callee, &FunctionVariant::Baseline) {
                                continue;
                            }
                            let callee_target =
                                targets.get_target(&callee, &FunctionVariant::Baseline);
                            usage_analysis::get_memory_usage(&callee_target)
                        };
                        for mem in usage.modified.get_all_inst(inst) {
                            add(mem);
                        }
                        for mem in usage.invoke_frame.get_all_inst(inst) {
                            add(mem);
                        }
                        // A wildcard `modifies_of<f> *` propagated up from
                        // the callee is not resolved in `invoke_frame` (the
                        // callee's own accessed footprint is often empty
                        // for pure forwarders — a caller inheriting the
                        // summary would havoc nothing). Resolve here
                        // against the enclosing function's own accessed
                        // footprint, which includes anything mentioned by
                        // its specs (`requires`/`ensures`/etc.).
                        if usage.invoke_frame_wildcard {
                            let self_usage = usage_analysis::get_memory_usage(func_target);
                            for mem in self_usage.accessed.all.iter() {
                                add(mem.clone());
                            }
                        }
                    },
                    Operation::Invoke => {
                        // Direct invocation in the current function body:
                        // the operand's function-value type is known here,
                        // so declared frames can be filtered by it. Union
                        // callee `invoke_frame`s inherited via
                        // `subsume_callee` (from static calls in the body,
                        // which have no per-Invoke type to filter against
                        // — they've already been resolved in the callee's
                        // analysis).
                        let self_usage = usage_analysis::get_memory_usage(func_target);
                        let invoked_ty = func_target
                            .get_local_type(*srcs.last().expect("invoke has a function operand"))
                            .skip_reference()
                            .clone();
                        let invoked_norm = matches!(invoked_ty, Type::Fun(..))
                            .then(|| invoked_ty.clone().normalize_fun());
                        let type_may_match = |ty: &Type| {
                            let ty = ty.skip_reference();
                            ty.is_open()
                                || invoked_ty.is_open()
                                || match (&invoked_norm, ty) {
                                    (Some(inv), Type::Fun(..)) => {
                                        &ty.clone().normalize_fun() == inv
                                    },
                                    _ => ty == &invoked_ty,
                                }
                        };
                        let mem_is_closed = |mem: &QualifiedInstId<StructId>| {
                            mem.inst.iter().all(|ty| !ty.is_open())
                        };
                        // Parameter-declared frames of the enclosing
                        // function, filtered by the invoked operand type.
                        let mut wildcard_here = false;
                        for access in func_target.func_env.get_fun_param_access_of() {
                            let param_ty = func_target
                                .func_env
                                .get_parameters()
                                .into_iter()
                                .find(|Parameter(sym, _, _)| *sym == access.fun_param)
                                .map(|Parameter(_, ty, _)| ty);
                            let matches = param_ty.as_ref().map(&type_may_match).unwrap_or(true);
                            if !matches {
                                continue;
                            }
                            if access.frame_spec.modifies_all {
                                wildcard_here = true;
                            }
                            for mem in &access.old_memory {
                                if mem_is_closed(mem) {
                                    add(mem.clone());
                                } else {
                                    wildcard_here = true;
                                }
                            }
                        }
                        // Struct-field-declared frames from anywhere in the
                        // program, filtered by field type.
                        for module_env in env.get_modules() {
                            for struct_env in module_env.get_structs() {
                                for access in struct_env.get_field_access_of() {
                                    let field_may_match = struct_env
                                        .find_field(access.fun_param)
                                        .map(|f| type_may_match(&f.get_type()))
                                        .unwrap_or(true);
                                    if !field_may_match {
                                        continue;
                                    }
                                    if access.frame_spec.modifies_all {
                                        wildcard_here = true;
                                    }
                                    for mem in &access.old_memory {
                                        if mem_is_closed(mem) {
                                            add(mem.clone());
                                        } else {
                                            wildcard_here = true;
                                        }
                                    }
                                }
                            }
                        }
                        if wildcard_here {
                            for mem in self_usage.accessed.all.iter() {
                                add(mem.clone());
                            }
                        }
                        // The function under processing is held out of the
                        // targets holder, so its own Closure ops must be
                        // scanned explicitly.
                        let current_id = func_target.func_env.get_qualified_id();
                        let mut closure_mems = vec![];
                        let mut closure_widen = false;
                        let mut scan_closures =
                            |creator_target: &FunctionTarget<'_>, creator_is_current: bool| {
                                for bc in creator_target.get_bytecode() {
                                    let Bytecode::Call(
                                        _,
                                        dests,
                                        Operation::Closure(mid, fid, inst, _),
                                        _,
                                        _,
                                    ) = bc
                                    else {
                                        continue;
                                    };
                                    if !type_may_match(creator_target.get_local_type(dests[0])) {
                                        continue;
                                    }
                                    let callee = env.get_function(mid.qualified(*fid));
                                    if callee.is_native()
                                        || callee.is_intrinsic()
                                        || callee.is_struct_api()
                                    {
                                        continue;
                                    }
                                    let callee_id = callee.get_qualified_id();
                                    let callee_usage = if callee_id == current_id {
                                        usage_analysis::get_memory_usage(func_target)
                                    } else {
                                        if !targets.has_target(&callee, &FunctionVariant::Baseline)
                                        {
                                            continue;
                                        }
                                        usage_analysis::get_memory_usage(
                                            &targets
                                                .get_target(&callee, &FunctionVariant::Baseline),
                                        )
                                    };
                                    // The closed-over callee may itself
                                    // invoke a function value transitively
                                    // — its `invoke_frame` carries those
                                    // effects and must be havocked here too.
                                    let mems = callee_usage
                                        .modified
                                        .get_all_inst(inst)
                                        .into_iter()
                                        .chain(callee_usage.invoke_frame.get_all_inst(inst));
                                    for mem in mems {
                                        if creator_is_current || mem_is_closed(&mem) {
                                            closure_mems.push(mem);
                                        } else {
                                            closure_widen = true;
                                        }
                                    }
                                }
                            };
                        scan_closures(func_target, true);
                        for fun_id in targets.get_funs() {
                            if fun_id == current_id {
                                continue;
                            }
                            let creator = env.get_function(fun_id);
                            if !targets.has_target(&creator, &FunctionVariant::Baseline) {
                                continue;
                            }
                            scan_closures(
                                &targets.get_target(&creator, &FunctionVariant::Baseline),
                                false,
                            );
                        }
                        for mem in closure_mems {
                            add(mem);
                        }
                        // Foreign-context closure memory (open types the
                        // creator's context can't express here) widens to
                        // the current function's accessed footprint —
                        // same fallback the wildcard case above uses.
                        if closure_widen {
                            for mem in self_usage.accessed.all.iter() {
                                add(mem.clone());
                            }
                        }
                    },
                    _ => {},
                }
            }
        }
        mems
    }

    /// Collect code offsets that are branch instructions forming loop back-edges
    ///
    /// The input to this function should include all the sub loops that constitute a fat-loop.
    /// This function will return one back-edge location for each sub loop.
    fn collect_loop_back_edges(
        &self,
        code: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
        header_label: Label,
        sub_loops: &[NaturalLoop<BlockId>],
    ) -> BTreeSet<CodeOffset> {
        sub_loops
            .iter()
            .map(|l| {
                let code_offset = match cfg.content(l.loop_latch) {
                    BlockContent::Dummy => {
                        panic!("A loop body should never contain a dummy block")
                    },
                    BlockContent::Basic { upper, .. } => *upper,
                };
                match &code[code_offset as usize] {
                    Bytecode::Jump(_, goto_label) if *goto_label == header_label => {},
                    Bytecode::Branch(_, if_label, else_label, _)
                        if *if_label == header_label || *else_label == header_label => {},
                    _ => panic!("The latch bytecode of a loop does not branch into the header"),
                };
                code_offset
            })
            .collect()
    }

    /// Collect bytecodes that constitute the loop
    ///
    /// The input to this function should include all the sub loops that constitute a fat-loop.
    /// This function will return a vector of basic blocks, where each basic block is a vector
    /// of bytecode.
    fn collect_loop_body_bytecode(
        &self,
        code: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
        sub_loops: &[NaturalLoop<BlockId>],
    ) -> Vec<Vec<Bytecode>> {
        let mut id_label_map = BTreeMap::new();
        let blocks: Vec<(BlockId, Vec<Bytecode>)> = sub_loops
            .iter()
            .flat_map(|l| l.loop_body.iter())
            .map(|block_id| match cfg.content(*block_id) {
                BlockContent::Dummy => {
                    panic!("A loop body should never contain a dummy block")
                },
                BlockContent::Basic { lower, upper } => {
                    if let Some(lbl) = code[*lower as usize].get_label_inner_opt() {
                        id_label_map.insert(*block_id, lbl);
                    }
                    let block: Vec<_> = (*lower..=*upper)
                        .map(|i| code.get(i as usize).unwrap().clone())
                        .collect();
                    (*block_id, block)
                },
            })
            .collect();
        let mut results = vec![];
        for (i, (block_id, block)) in blocks.iter().enumerate() {
            // if this block is a fallthough block, we need to remove it by inserting a jump to the correct block
            // because the fatloop algorithm doesn't support fallthrough
            if cfg.successors(*block_id).len() == 1 && !block[block.len() - 1].is_branching() {
                let successor_id = cfg.successors(*block_id).first().unwrap();
                let successor_label_opt = id_label_map.get(successor_id).copied();
                if let Some(successor_label) = successor_label_opt
                    && i != blocks.len() - 1
                    && !blocks.get(i + 1).unwrap().1.is_empty()
                {
                    if let Some(lbl) = blocks.get(i + 1).unwrap().1[0].get_label_inner_opt() {
                        if lbl != successor_label {
                            let mut new_block = block.clone();
                            // Inserted bc is used for jumping to its successor so
                            // we just use the attr_id of its previous bc
                            new_block.push(Bytecode::Jump(
                                block[block.len() - 1].get_attr_id(),
                                Label::new(successor_label as usize),
                            ));
                            results.push(new_block);
                            continue;
                        }
                    }
                }
                results.push(block.clone());
            } else {
                results.push(block.clone());
            }
        }
        results
    }
}
