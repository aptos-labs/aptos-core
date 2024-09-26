// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

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
    graph::{Graph, NaturalLoop},
    stackless_bytecode::{AttrId, Bytecode, Label, PropKind},
    stackless_control_flow_graph::{BlockContent, BlockId, StacklessControlFlowGraph},
};
use anyhow::bail;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast, ast::TempIndex, pragmas::UNROLL_PRAGMA};
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
    FatLoopBuilder { for_spec: false }
        .build_loop_info(func_target)
        .map(|(info, _)| info)
}

/// Find all fat loops in the function and collect information needed for invariant instrumentation
/// (i.e., loop-to-DAG transformation) and loop unrolling (if requested by user).
pub fn build_loop_info_for_spec(
    func_target: &FunctionTarget,
) -> anyhow::Result<(FatLoopFunctionInfo, LoopUnrollingFunctionInfo)> {
    FatLoopBuilder { for_spec: true }.build_loop_info(func_target)
}

struct FatLoopBuilder {
    for_spec: bool,
}

impl FatLoopBuilder {
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
                        Some(FatLoopSpecInfo {
                            invariants,
                            val_targets,
                            mut_targets,
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
        sub_loops
            .iter()
            .flat_map(|l| l.loop_body.iter())
            .map(|block_id| match cfg.content(*block_id) {
                BlockContent::Dummy => {
                    panic!("A loop body should never contain a dummy block")
                },
                BlockContent::Basic { lower, upper } => {
                    let block: Vec<_> = (*lower..=*upper)
                        .map(|i| code.get(i as usize).unwrap().clone())
                        .collect();
                    block
                },
            })
            .collect()
    }
}
