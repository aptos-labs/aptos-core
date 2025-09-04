// Copyright (c) Velor Foundation
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
//! 3. Used in the wrong order than they are put on the stack.
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
    ops::{Range, RangeInclusive},
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
        for offset in block_range.clone() {
            let instr = &code[offset as usize];
            use Bytecode::{Assign, Call, Load};
            // Only `Assign`, `Call`, and `Load` instructions push temps to the stack.
            // We need to find if any of these temps are better flushed right away.
            if matches!(instr, Assign(..) | Call(..) | Load(..)) {
                for (dest_index, dest) in instr.dests().into_iter().enumerate() {
                    let def = DefOrUsePoint {
                        offset,
                        index: dest_index,
                    };
                    if Self::better_flushed_right_away(def, upper, use_def_links) {
                        flush_writes.entry(offset).or_default().insert(dest);
                    }
                }
            }
        }
        // We have identified some of the temps that are better flushed right away.
        // We identity more based on their usage order below.
        for offset in block_range.clone() {
            Self::process_out_of_order_uses_in_same_instruction(
                code,
                offset,
                *block_range.start(),
                use_def_links,
                flush_writes,
            );
            Self::process_out_of_order_uses_in_different_instructions(
                code,
                offset,
                *block_range.start(),
                use_def_links,
                flush_writes,
            );
        }
    }

    /// Is the `def` better flushed right away?
    /// If there more than one use or the use is outside the block, then it is better flushed right away.
    /// `block_end` is the end of the block that has `def`.
    fn better_flushed_right_away(
        def: DefOrUsePoint,
        block_end: CodeOffset,
        use_def_links: &UseDefLinks,
    ) -> bool {
        use_def_links.def_to_use.get(&def).is_none_or(|uses| {
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
            false
        })
    }

    /// Given an instruction at `offset` of `code`, check if there are uses of temporaries in the
    /// wrong order, and if so, mark the corresponding temporaries for flushing.
    /// `block_start` is the start of the block that has the instruction at `offset`.
    ///
    /// Example of a wrong order use in the same instruction:
    /// ```move
    /// let a = some_integer(); // stack: [`a`]
    /// let b = some_integer(); // stack: [`a`, `b`]
    /// let c = some_integer(); // stack: [`a`, `b`, `c`]
    /// let d = some_integer(); // stack: [`a`, `b`, `c`, `d`]
    /// consume(a, c, b, d);
    /// ```
    /// In the above code, the stack should be [`a`, `c`, `b`, `d`] before the `consume` call.
    /// However, as it stands, the stack is [`a`, `b`, `c`, `d`]. Thus, the top 3 temps on the
    /// stack have to be flushed and re-loaded in the right order.
    /// Instead, because we know the (`c`, `b`) pair is used in the wrong order, we can flush `b`
    /// right away. With this, we only flush and re-load one temp instead of three. This function
    /// accomplishes this behavior by marking `b` for flushing right away.
    fn process_out_of_order_uses_in_same_instruction(
        code: &[Bytecode],
        offset: CodeOffset,
        block_start: CodeOffset,
        use_def_links: &UseDefLinks,
        flush_writes: &mut BTreeMap<CodeOffset, BTreeSet<TempIndex>>,
    ) {
        let instr = &code[offset as usize];
        // We are only interested in `Call` and `Ret` instructions, which have multiple sources,
        // and therefore can have uses in the wrong order compared to their definitions.
        if !matches!(instr, Bytecode::Call(..) | Bytecode::Ret(..)) {
            return;
        }
        let offset_range = block_start..offset;
        let sources = instr.sources();
        for pair in (0..sources.len()).combinations(2) {
            Self::flush_out_of_order_uses_in_same_instruction(
                pair[0],
                pair[1],
                offset,
                code,
                use_def_links,
                &offset_range,
                &sources,
                flush_writes,
            );
        }
    }

    /// Given two use indices `use_index_1` and `use_index_2` in the same instruction, such that
    /// `use_index_1 < use_index_2`, check if the uses are in the wrong order compared to their
    /// definition. If so, mark the corresponding temporary for flushing.
    /// See `process_out_of_order_uses_in_same_instruction` for an example.
    fn flush_out_of_order_uses_in_same_instruction(
        use_index_1: usize,
        use_index_2: usize,
        offset: CodeOffset,
        code: &[Bytecode],
        use_def_links: &UseDefLinks,
        offset_range: &Range<CodeOffset>,
        sources: &[TempIndex],
        flush_writes: &mut BTreeMap<CodeOffset, BTreeSet<TempIndex>>,
    ) {
        let use_1 = DefOrUsePoint {
            offset,
            index: use_index_1,
        };
        let use_2 = DefOrUsePoint {
            offset,
            index: use_index_2,
        };
        let def_set_1 = use_def_links.use_to_def.get(&use_1);
        let def_set_2 = use_def_links.use_to_def.get(&use_2);
        match (def_set_1, def_set_2) {
            (None, None) => {
                // nothing to be done, both definitions are not within this block
            },
            (_, None) => {
                // nothing to be done, the second definition is not within this block
            },
            (None, Some(defs_2)) => {
                // Is the `def_2` within this block (and the only definition)? If so, it must be flushed.
                if defs_2.len() == 1 {
                    let def_2_offset = defs_2
                        .first()
                        .expect("there must be at least one def")
                        .offset;
                    if offset_range.contains(&def_2_offset) {
                        flush_writes
                            .entry(def_2_offset)
                            .or_default()
                            .insert(sources[use_index_2]);
                    }
                }
            },
            (Some(defs_1), Some(defs_2)) => {
                if defs_1.len() == 1 && defs_2.len() == 1 {
                    let def_1 = defs_1.first().expect("there must be at least one def");
                    let def_2 = defs_2.first().expect("there must be at least one def");
                    let def_1_actual = Self::get_def_skipping_self_assigns(
                        def_1,
                        code,
                        offset_range,
                        use_def_links,
                    );
                    let def_2_actual = Self::get_def_skipping_self_assigns(
                        def_2,
                        code,
                        offset_range,
                        use_def_links,
                    );
                    if offset_range.contains(&def_1_actual.offset)
                        && offset_range.contains(&def_2_actual.offset)
                        && def_1_actual > def_2_actual
                    {
                        flush_writes
                            .entry(def_2_actual.offset)
                            .or_default()
                            .insert(sources[use_index_2]);
                    }
                }
            },
        }
    }

    /// Given an instruction at `offset` of `code`, check if there are uses of temporaries in the
    /// wrong order when looking at uses at this instruction and other instructions. If so, mark
    /// the corresponding temporaries for flushing.
    /// `block_start` is the start of the block that has the instruction at `offset`.
    ///
    /// Example of a wrong order use within different instructions:
    /// ```move
    /// let a = some_integer(); // stack: [`a`]
    /// let b = some_integer(); // stack: [`a`, `b`]
    /// let c = some_integer(); // stack: [`a`, `b`, `c`]
    /// consume_2(a, c);
    /// consume_1(b);
    /// ```
    /// In the above code, the stack should be [`a`, `c`] before the `consume_2` call.
    /// However, as it stands, the stack is [`a`, `b`, `c`]. Thus, the top 2 temps on the
    /// stack have to be flushed and re-loaded in the right order.
    /// Instead, because we know the (`c`, `b`) pair is used in the wrong order, we can
    /// flush `b` right away. With this, we only flush and re-load one temp instead of two.
    fn process_out_of_order_uses_in_different_instructions(
        code: &[Bytecode],
        offset: CodeOffset,
        block_start: CodeOffset,
        use_def_links: &UseDefLinks,
        flush_writes: &mut BTreeMap<CodeOffset, BTreeSet<TempIndex>>,
    ) {
        // We are only interested in the uses in `Call` and `Ret` instructions.
        let instr = &code[offset as usize];
        if !matches!(instr, Bytecode::Call(..) | Bytecode::Ret(..)) {
            return;
        }
        if matches!(instr, Bytecode::Call(_, _, Operation::BorrowLoc, ..)) {
            // `BorrowLoc` does not require its operands to be on the stack.
            return;
        }
        let sources = instr.sources();
        for (use_index, use_temp) in sources.into_iter().enumerate() {
            Self::flush_out_of_order_uses_in_different_instructions(
                use_index,
                use_temp,
                offset,
                block_start,
                code,
                use_def_links,
                flush_writes,
            );
        }
    }

    /// Given a `use_index` at instruction `offset` of `code`, check if there are uses of temps
    /// in other previous instructions that are in the wrong order compared to their definition.
    /// If so, mark the corresponding temp for flushing.
    /// See `process_out_of_order_uses_in_different_instructions` for an example.
    fn flush_out_of_order_uses_in_different_instructions(
        use_index: usize,
        use_temp: TempIndex,
        offset: CodeOffset,
        block_start: CodeOffset,
        code: &[Bytecode],
        use_def_links: &UseDefLinks,
        flush_writes: &mut BTreeMap<CodeOffset, BTreeSet<TempIndex>>,
    ) {
        let this_use_point = DefOrUsePoint {
            offset,
            index: use_index,
        };
        if let Some(this_uses_actual_def) = Self::get_singular_actual_def_in_range(
            &this_use_point,
            use_def_links,
            code,
            block_start..offset,
        ) {
            if Self::is_def_flushed_away(&this_uses_actual_def, code, flush_writes) {
                return;
            }
            let other_uses =
                Self::get_uses_in_between(this_uses_actual_def.offset + 1..offset, code);
            // Compare this use with all other uses that may conflict with this use.
            for other_use in other_uses {
                if let Some(other_uses_actual_def) = Self::get_singular_actual_def_in_range(
                    &other_use,
                    use_def_links,
                    code,
                    block_start..other_use.offset,
                ) {
                    if other_uses_actual_def < this_uses_actual_def
                        && !Self::is_def_flushed_away(&other_uses_actual_def, code, flush_writes)
                    {
                        // Flush the use that is the only use, to minimize flushes.
                        // TODO: This heuristic could be improved by considering multiple uses in two (or more)
                        // conflicting instructions at the same time.
                        if Self::is_only_use_in_instr(&this_use_point, code) {
                            flush_writes
                                .entry(this_uses_actual_def.offset)
                                .or_default()
                                .insert(use_temp);
                        } else if Self::is_only_use_in_instr(&other_use, code) {
                            flush_writes
                                .entry(other_uses_actual_def.offset)
                                .or_default()
                                .insert(Self::get_temp_from_def_point(
                                    &other_uses_actual_def,
                                    code,
                                ));
                        }
                    }
                }
            }
        }
    }

    /// Get the temporary corresponding to `def_point`.
    fn get_temp_from_def_point(def_point: &DefOrUsePoint, code: &[Bytecode]) -> TempIndex {
        let instr = &code[def_point.offset as usize];
        instr.dests()[def_point.index]
    }

    /// Is `use_point` the only use in that instruction?
    fn is_only_use_in_instr(use_point: &DefOrUsePoint, code: &[Bytecode]) -> bool {
        let instr = &code[use_point.offset as usize];
        let sources = instr.sources();
        sources.len() == 1 && use_point.index == 0
    }

    /// Get all the use points in the `offset_range`.
    fn get_uses_in_between(
        offset_range: Range<CodeOffset>,
        code: &[Bytecode],
    ) -> Vec<DefOrUsePoint> {
        let mut uses = vec![];
        for offset in offset_range {
            let instr = &code[offset as usize];
            if let Bytecode::Call(_, _, op, sources, _) = instr {
                if *op == Operation::BorrowLoc {
                    continue;
                }
                for use_index in 0..sources.len() {
                    uses.push(DefOrUsePoint {
                        offset,
                        index: use_index,
                    });
                }
            }
        }
        uses
    }

    /// Get the actual definition (the definition obtained by skipping self-assigns) of `use_point`.
    /// Is this the only definition and is it within the `range`?
    fn get_singular_actual_def_in_range(
        use_point: &DefOrUsePoint,
        use_def_links: &UseDefLinks,
        code: &[Bytecode],
        range: Range<CodeOffset>,
    ) -> Option<DefOrUsePoint> {
        let def_set = use_def_links.use_to_def.get(use_point)?;
        if def_set.len() != 1 {
            return None;
        }
        let def = def_set.first().expect("there is at least one def");
        let actual_def = Self::get_def_skipping_self_assigns(def, code, &range, use_def_links);
        if range.contains(&def.offset) {
            Some(actual_def)
        } else {
            None
        }
    }

    /// Has `def` been marked to be flushed right away?
    fn is_def_flushed_away(
        def: &DefOrUsePoint,
        code: &[Bytecode],
        flush_writes: &BTreeMap<CodeOffset, BTreeSet<TempIndex>>,
    ) -> bool {
        if let Some(temps) = flush_writes.get(&def.offset) {
            // Some temps were marked to be flushed right away at `instr`.
            let instr = &code[def.offset as usize];
            // Was it this `def`?
            let def_temp = instr.dests()[def.index];
            return temps.contains(&def_temp);
        }
        false
    }

    /// Trace `def` to the actual definition by skipping self-assigns in the `offset_range`.
    fn get_def_skipping_self_assigns(
        def: &DefOrUsePoint,
        code: &[Bytecode],
        offset_range: &Range<CodeOffset>,
        use_def_links: &UseDefLinks,
    ) -> DefOrUsePoint {
        let mut actual_def = def.clone();
        // Only get defs within the same block.
        while offset_range.contains(&actual_def.offset) {
            let instr = &code[actual_def.offset as usize];
            let mut changed = false;
            if let Bytecode::Assign(_, dest, src, _) = instr {
                if *dest == *src {
                    // This is a self assign, skip it and get the actual def.
                    let self_use = DefOrUsePoint {
                        offset: actual_def.offset,
                        index: 0,
                    };
                    if let Some(new_def) = use_def_links.use_to_def.get(&self_use) {
                        if new_def.len() == 1 {
                            actual_def =
                                new_def.first().expect("there is at least one def").clone();
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        actual_def
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
