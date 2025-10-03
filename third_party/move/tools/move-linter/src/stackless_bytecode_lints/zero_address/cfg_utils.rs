use move_binary_format::file_format::CodeOffset;
use move_model::ast::TempIndex;
use move_stackless_bytecode::{
    stackless_bytecode::{Bytecode, Label},
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use num::ToPrimitive;
use std::collections::{BTreeMap, HashMap};

pub(super) fn collect_label_offsets(code: &[Bytecode]) -> BTreeMap<Label, CodeOffset> {
    let mut map = BTreeMap::new();
    for (offset, instr) in code.iter().enumerate() {
        let Some(code_offset) = offset.to_u16() else {
            continue;
        };
        if let Bytecode::Label(_, label) = instr {
            map.insert(*label, code_offset);
        }
    }
    map
}

pub(super) fn build_label_to_block_map(
    label_offsets: &BTreeMap<Label, CodeOffset>,
    cfg: &StacklessControlFlowGraph,
) -> HashMap<Label, BlockId> {
    let mut map = HashMap::new();
    for (label, offset) in label_offsets {
        let block = cfg.enclosing_block(*offset);
        map.insert(*label, block);
    }
    map
}

pub(super) struct BranchInfo {
    pub(super) last_offset: CodeOffset,
    pub(super) then_block: BlockId,
    pub(super) else_block: BlockId,
    pub(super) cond: TempIndex,
}

pub(super) fn collect_branch_info(
    code: &[Bytecode],
    cfg: &StacklessControlFlowGraph,
    label_to_block: &HashMap<Label, BlockId>,
) -> HashMap<BlockId, BranchInfo> {
    let mut map = HashMap::new();
    for block_id in cfg.blocks() {
        let Some(last_offset) = cfg.instr_offset_bounds(block_id).map(|(_, end)| end) else {
            continue;
        };
        let Some(Bytecode::Branch(_, then_label, else_label, cond)) =
            code.get(last_offset as usize)
        else {
            continue;
        };
        let Some(&then_block) = label_to_block.get(then_label) else {
            continue;
        };
        let Some(&else_block) = label_to_block.get(else_label) else {
            continue;
        };
        map.insert(
            block_id,
            BranchInfo {
                last_offset,
                then_block,
                else_block,
                cond: *cond,
            },
        );
    }
    map
}
