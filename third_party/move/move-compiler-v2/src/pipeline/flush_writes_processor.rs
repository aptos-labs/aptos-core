// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub struct FlushWritesAnnotation(pub BTreeMap<CodeOffset, BTreeSet<TempIndex>>);

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
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let mut unused: BTreeMap<CodeOffset, BTreeSet<TempIndex>> = BTreeMap::new();
        for block_id in cfg.blocks() {
            if let Some((lower, upper)) = cfg.instr_offset_bounds(block_id) {
                extract_unused_writes_in_block(lower, upper, code, live_vars, &mut unused);
            }
        }
        data.annotations.set(FlushWritesAnnotation(unused), true);
        data
    }

    fn name(&self) -> String {
        "FlushWritesProcessor".to_string()
    }
}

fn extract_unused_writes_in_block(
    lower: u16,
    upper: u16,
    code: &[Bytecode],
    live_vars: &LiveVarAnnotation,
    unused: &mut BTreeMap<u16, BTreeSet<usize>>,
) {
    for offset in lower..=upper {
        let instr = &code[offset as usize];
        // Only `Load` and `Call` instructions push results to the stack.
        if matches!(instr, Bytecode::Load(..) | Bytecode::Call(..)) {
            if let Some(live_info) = live_vars.get_live_var_info_at(offset) {
                for dest in instr.dests() {
                    if let Some(info) = live_info.after.get(&dest) {
                        // `dest` is alive after `offset`.
                        let all_usages_are_outside_block = info
                            .usage_offsets()
                            .iter()
                            .all(|usage| *usage <= offset || *usage > upper);
                        if all_usages_are_outside_block {
                            unused.entry(offset).or_default().insert(dest);
                        }
                    } else {
                        // `dest` is not alive after `offset`, so it is not used.
                        unused.entry(offset).or_default().insert(dest);
                    }
                }
            }
        }
    }
}

impl FlushWritesProcessor {
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
