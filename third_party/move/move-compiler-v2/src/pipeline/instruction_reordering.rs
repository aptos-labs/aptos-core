// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use move_binary_format::file_format::CodeOffset;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct OrderingAnnotation(BTreeMap<CodeOffset, Vec<u16>>);

/// Mapping from every touch instruction to the "use" instruction they correspond to.
pub struct TouchUseAnnotation(pub BTreeMap<CodeOffset, CodeOffset>);

struct UseDefGraph {
    edges: BTreeMap<CodeOffset, Vec<Option<CodeOffset>>>,
}

impl UseDefGraph {
    pub fn compute_annotation(target: &FunctionTarget) -> OrderingAnnotation {
        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let live_vars = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live variable annotation is a prerequisite");
        UseDefGraph::initialize_from(code, &cfg, live_vars).annotate(code, &cfg)
    }

    fn initialize_from(
        code: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
        live_vars: &LiveVarAnnotation,
    ) -> Self {
        let mut edges = BTreeMap::new();
        // Create all intra-block use-def edges.
        for block_id in cfg.blocks() {
            if let Some((lower, upper)) = cfg.instr_offset_bounds(block_id) {
                for def_offset in lower..=upper {
                    for (temp, info) in live_vars.get_info_at(def_offset).after.iter() {
                        for usage_offset in info.usage_offsets() {
                            if usage_offset < lower || usage_offset > upper {
                                // Usage is outside of the block, ignore.
                                continue;
                            }
                            let usage_instr = &code[usage_offset as usize];
                            if usage_instr.is_spec_only() {
                                continue;
                            }
                            let sources = usage_instr.sources();
                            if !sources.is_empty() {
                                let pos = sources
                                    .iter()
                                    .position(|t| t == temp)
                                    .expect("def should be in sources");
                                let use_def_edges = edges
                                    .entry(usage_offset)
                                    .or_insert(vec![None; sources.len()]);
                                use_def_edges[pos] = Some(def_offset);
                            }
                        }
                    }
                }
            }
        }
        Self { edges }
    }

    fn dfs(
        &self,
        node: CodeOffset,
        visited: &mut [bool],
        annot: &mut OrderingAnnotation,
        num: &mut u16,
    ) {
        if visited[node as usize] {
            return;
        }
        visited[node as usize] = true;
        if let Some(edges) = self.edges.get(&node) {
            for edge in edges.iter().flatten() {
                self.dfs(*edge, visited, annot, num);
            }
        }
        annot
            .0
            .entry(node)
            .and_modify(|v| v.push(*num))
            .or_insert(vec![*num]);
        *num += 1;
    }

    fn annotate(&self, code: &[Bytecode], cfg: &StacklessControlFlowGraph) -> OrderingAnnotation {
        let mut annot = OrderingAnnotation(BTreeMap::new());
        let mut visited = vec![false; code.len()];
        for block_id in cfg.blocks() {
            if let Some((lower, upper)) = cfg.instr_offset_bounds(block_id) {
                for offset in lower..=upper {
                    let instr = &code[offset as usize];
                    if instr.is_spec_only() {
                        continue;
                    }
                    use Bytecode::*;
                    match instr {
                        Call(_, _, op, ..) if op.can_abort() => {
                            self.dfs(offset, &mut visited, &mut annot, &mut 0)
                        },
                        Ret(..) | Branch(..) | Abort(..) => {
                            self.dfs(offset, &mut visited, &mut annot, &mut 0)
                        },
                        _ => (),
                    }
                }
            }
        }
        annot
    }
}

pub struct InstructionReorderingProcessor {}

impl FunctionTargetProcessor for InstructionReorderingProcessor {
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
        let annot = UseDefGraph::compute_annotation(&target);
        data.annotations.set(annot, true);
        data.annotations.remove::<LiveVarAnnotation>();
        data
    }

    fn name(&self) -> String {
        "InstructionReorderingProcessor".to_string()
    }
}

impl InstructionReorderingProcessor {
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_instruction_reordering_annotation));
    }
}

pub fn format_instruction_reordering_annotation(
    target: &FunctionTarget,
    code_offset: CodeOffset,
) -> Option<String> {
    let annot = target.get_annotations().get::<OrderingAnnotation>()?;
    let annot = annot.0.get(&code_offset)?;
    let nums = annot
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("nums: {nums}"))
}
