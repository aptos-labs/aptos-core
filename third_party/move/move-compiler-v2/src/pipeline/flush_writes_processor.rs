// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a processor that determines which writes to temporaries
//! are better "flushed" right away by the file format code generator (as long
//! it does not lead to potentially extra flushes: this condition is assumed to be
//! implicit in the rest of this file, to avoid repetition). As such, these are
//! suggestions, and can be safely ignored by the file format generator.
//! Read on for more information on what "flushing" means.
//!
//! For this pass to be effective, it should be run after all the stackless-bytecode
//! transformations are done, because the annotations produced by it are used
//! (when available) by the file-format generator. Code transformations render
//! previously computed annotations invalid.
//!
//! Prerequisite: the `LiveVarAnnotation` should already be computed by running the
//! `LiveVarAnalysisProcessor` in the `track_all_usages` mode.
//!
//! The file format generator can keep some writes to temporaries only on the stack,
//! not writing it back to local memory (as a potential optimization).
//! However, this is not always good, and this pass helps determine when a write to
//! a temporary is better flushed right away.
//! In the context of file format code generator, "flushed" means either store the
//! value to a local (if used later) or pop if from the stack (if not used later).
//!
//! Currently, we suggest to flush those temps right away that are:
//! 1. Not used within the same basic block, because these will be flushed without
//!    getting consumed anyway at the end of the block.
//! 2. Used multiple times. Before getting consumed, these have to be flushed to local
//!    memory anyway.
//! 3. Used in the wrong order in an instruction, than they are put on the stack.
//!    In such a case, they would be flushed before getting consumed anyway.
//!    For example, in the code below:
//!    ```move
//!    let a = foo(); // stack: [`a`]
//!    let b = foo(); // stack: [`a`, `b`]
//!    consume(b, a); // we need the stack to be [`b`, `a`], so the entire stack has
//!                   // to be flushed and reloaded in the right order.
//!    ```
//!    Instead, by flushing `a` eagerly when it is written, we can avoid flushing and
//!    reloading `b`.
//! In all these cases, the file format generator can avoid extra stack operations due
//! to eager flushing.

use crate::pipeline::livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfoAtCodeOffset};
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::RangeInclusive,
};

/// For a given code offset, tracks which temporaries written at the code offset
/// should be flushed right away (popped from stack or saving to a local if used
/// elsewhere) by the file format generator.
#[derive(Clone)]
pub struct FlushWritesAnnotation(pub BTreeMap<CodeOffset, BTreeSet<TempIndex>>);

/// A point in the code where a temporary is defined or used.
/// Note: the order of the fields is important for comparisons.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct DefOrUsePoint {
    offset: CodeOffset, // code offset of the instruction
    index: usize,       // source (for use) or destination (for def) index in the instruction
}

/// Collection of links between definitions and uses of temporaries in a function.
/// Note that this includes only the uses of explicit definitions in the function,
/// in particular, the uses of function parameters are not included.
struct UseDefLinks {
    /// Maps a use point to the set of definition points that define the temporary used.
    use_to_def: BTreeMap<DefOrUsePoint, BTreeSet<DefOrUsePoint>>,
    /// Maps a definition point to the set of use points that use the temporary defined.
    def_to_use: BTreeMap<DefOrUsePoint, BTreeSet<DefOrUsePoint>>,
}

impl UseDefLinks {
    /// Create a new `UseDefLinks` instance for a function with `code` and `live_vars`.
    pub fn new(code: &[Bytecode], live_vars: &LiveVarAnnotation) -> Self {
        let mut use_to_def: BTreeMap<DefOrUsePoint, BTreeSet<DefOrUsePoint>> = BTreeMap::new();
        let mut def_to_use: BTreeMap<DefOrUsePoint, BTreeSet<DefOrUsePoint>> = BTreeMap::new();
        for (def_offset, def_instr) in code.iter().enumerate() {
            let live_info = live_vars.get_info_at(def_offset as CodeOffset);
            for (dest_index, dest) in def_instr.dests().into_iter().enumerate() {
                let mut use_points = Self::compute_use_points(dest, code, live_info).peekable();
                if use_points.peek().is_none() {
                    // If there are no uses, there are no links to create.
                    continue;
                }
                let def_point = DefOrUsePoint {
                    offset: def_offset as CodeOffset,
                    index: dest_index,
                };
                let use_set = def_to_use.entry(def_point.clone()).or_default();
                for use_point in use_points {
                    use_to_def
                        .entry(use_point.clone())
                        .or_default()
                        .insert(def_point.clone());
                    use_set.insert(use_point);
                }
            }
        }
        Self {
            use_to_def,
            def_to_use,
        }
    }

    /// Compute the use points of `dest` defined at an instruction which has `live_info`.
    fn compute_use_points<'a>(
        dest: TempIndex,
        code: &'a [Bytecode],
        live_info: &LiveVarInfoAtCodeOffset,
    ) -> Box<dyn Iterator<Item = DefOrUsePoint> + 'a> {
        if let Some(info) = live_info.after.get(&dest) {
            Box::new(
                info.usage_offsets()
                    .into_iter()
                    .flat_map(move |use_offset| {
                        let use_instr = &code[use_offset as usize];
                        let mut sources = use_instr.sources();
                        // We need to handle `WriteRef` instructions specially, because the order
                        // of operands in stackless bytecode and stack-based bytecode is reversed.
                        if let Bytecode::Call(_, _, Operation::WriteRef, _, _) = use_instr {
                            sources.reverse();
                        }
                        sources
                            .into_iter()
                            .enumerate()
                            .filter_map(move |(src_index, src)| {
                                if src == dest {
                                    Some(DefOrUsePoint {
                                        offset: use_offset,
                                        index: src_index,
                                    })
                                } else {
                                    None
                                }
                            })
                    }),
            )
        } else {
            Box::new(std::iter::empty())
        }
    }
}

/// A processor for computing the `FlushWritesAnnotation`.
pub struct FlushWritesProcessor {}

impl FunctionTargetProcessor for FlushWritesProcessor {
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
        let live_vars = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live variable annotation is a prerequisite");
        let code = target.get_bytecode();
        let use_def_links = UseDefLinks::new(code, live_vars);
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let mut flush_writes: BTreeMap<CodeOffset, BTreeSet<TempIndex>> = BTreeMap::new();
        for block_id in cfg.blocks() {
            if let Some((lower, upper)) = cfg.instr_offset_bounds(block_id) {
                Self::extract_flush_writes_in_block(
                    lower..=upper,
                    code,
                    &use_def_links,
                    &mut flush_writes,
                );
            }
        }
        data.annotations
            .set(FlushWritesAnnotation(flush_writes), true);
        data
    }

    fn name(&self) -> String {
        "FlushWritesProcessor".to_string()
    }
}

impl FlushWritesProcessor {
    /// In the basic block given by `block_range` part of `code`, extract the writes
    /// to temporaries that are better flushed right away. At the offset where the
    /// write happens, such temporaries are included, in the out param `flush_writes`.
    fn extract_flush_writes_in_block(
        block_range: RangeInclusive<CodeOffset>,
        code: &[Bytecode],
        use_def_links: &UseDefLinks,
        flush_writes: &mut BTreeMap<CodeOffset, BTreeSet<TempIndex>>,
    ) {
        let upper = *block_range.end();
        for offset in block_range {
            let instr = &code[offset as usize];
            // Only `Load` and `Call` instructions push temps to the stack.
            // We need to find if any of these temps are better flushed right away.
            if matches!(instr, Bytecode::Load(..) | Bytecode::Call(..)) {
                for (dest_index, dest) in instr.dests().into_iter().enumerate() {
                    let def = DefOrUsePoint {
                        offset,
                        index: dest_index,
                    };
                    if Self::could_flush_right_away(def, upper, use_def_links) {
                        flush_writes.entry(offset).or_default().insert(dest);
                    }
                }
            }
        }
    }

    /// Is the `def` better flushed right away?
    /// `block_end` is the end of the block that has `def`.
    fn could_flush_right_away(
        def: DefOrUsePoint,
        block_end: CodeOffset,
        use_def_links: &UseDefLinks,
    ) -> bool {
        use_def_links.def_to_use.get(&def).map_or(true, |uses| {
            let exactly_one_use = uses.len() == 1;
            if !exactly_one_use {
                // If there is more than one use, flush right away.
                return true;
            }
            let use_ = uses.first().expect("there is exactly one use");
            let use_outside_block = use_.offset <= def.offset || use_.offset > block_end;
            if use_outside_block {
                // If used outside the basic block, flush right away.
                return true;
            }
            // If has intervening definition, flush right away.
            Self::has_intervening_def(def, use_, use_def_links)
        })
    }

    /// Given the `use_` of `def`, is there a previous use of any temp at the same
    /// instruction as `use_`, which has a definition after `def` and before
    /// the `use_` instruction?
    fn has_intervening_def(
        def: DefOrUsePoint,
        use_: &DefOrUsePoint,
        use_def_links: &UseDefLinks,
    ) -> bool {
        let DefOrUsePoint {
            offset: use_offset,
            index: use_index,
        } = use_;
        (0..*use_index).any(|prev| {
            let prev_use_at_usage_instr = DefOrUsePoint {
                offset: *use_offset,
                index: prev,
            };
            use_def_links
                .use_to_def
                .get(&prev_use_at_usage_instr)
                .map_or(false, |defs| {
                    defs.iter().any(|defs_of_prev_use| {
                        defs_of_prev_use > &def && defs_of_prev_use.offset < *use_offset
                    })
                })
        })
    }

    /// Registers annotation formatter at the given function target.
    /// Helps with testing and debugging.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_flush_writes_annotation));
    }
}

// ====================================================================
// Formatting functionality for flush writes annotation

pub fn format_flush_writes_annotation(
    target: &FunctionTarget,
    code_offset: CodeOffset,
) -> Option<String> {
    let FlushWritesAnnotation(map) = target.get_annotations().get::<FlushWritesAnnotation>()?;
    let temps = map.get(&code_offset)?;
    if temps.is_empty() {
        return None;
    }
    let mut res = "flush: ".to_string();
    res.push_str(
        &temps
            .iter()
            .map(|t| {
                let name = target.get_local_raw_name(*t);
                format!("{}", name.display(target.symbol_pool()))
            })
            .join(", "),
    );
    Some(res)
}
