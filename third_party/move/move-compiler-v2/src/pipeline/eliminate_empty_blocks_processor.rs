use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Label},
    stackless_control_flow_graph::{
        generate_cfg_in_dot_format, BlockId, StacklessControlFlowGraph,
    },
};
use std::collections::{BTreeMap, BTreeSet};

pub struct EliminateEmptyBlocksProcessor {}

impl FunctionTargetProcessor for EliminateEmptyBlocksProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(fun_env, &data);
        println!("{}", generate_cfg_in_dot_format(&target));
        let mut transformer = EliminateEmptyBlocksTransformation::new(data);
        transformer.transform();
        let target = FunctionTarget::new(fun_env, &transformer.data);
        println!("{}", generate_cfg_in_dot_format(&target));
        transformer.data
    }

    fn name(&self) -> String {
        "EliminateEmptyBlocksProcessor".to_owned()
    }
}

struct EliminateEmptyBlocksTransformation {
    data: FunctionData,
    cfg: StacklessControlFlowGraph,
    original_code: Vec<Bytecode>,
}

impl EliminateEmptyBlocksTransformation {
    pub fn new(mut data: FunctionData) -> Self {
        let original_code = std::mem::take(&mut data.code);
        let cfg = StacklessControlFlowGraph::new_forward(&original_code);
        Self {
            data,
            cfg,
            original_code,
        }
    }

    fn transform(&mut self) {
        let subst = self.collect_blocks_to_remove();
        self.gen_code_from_cfg(&subst);
    }

    /// Generate code from a cfg where empty blocks have been removed
    fn gen_code_from_cfg(&mut self, subst: &BTreeMap<BlockId, BlockId>) {
        let blocks_to_remove: BTreeSet<BlockId> = subst.keys().cloned().collect();
		let subst_block = subst;
        let subst_label = subst
            .iter()
            .map(|(b0, b1)| (self.get_block_label(*b0), self.get_block_label(*b1)))
            .collect();
        let mut visited = BTreeSet::new();
        let mut to_visit = vec![self.cfg.entry_block()];
        while let Some(block) = to_visit.pop() {
            let mut last_instr_of_block = None;
            if !blocks_to_remove.contains(&block) {
                let codes = self.cfg.content(block).to_bytecodes(&self.original_code);
                debug_assert!(
                    self.cfg.entry_block() == block
                        || self.cfg.exit_block() == block
                        || codes.len() != 0
                );
                for bytecode in self.cfg.content(block).to_bytecodes(&self.original_code) {
                    let transformed = self.transform_bytecode(bytecode.clone(), &subst_label);
                    self.data.code.push(transformed.clone());
                    last_instr_of_block = Some(transformed);
                }
            }
            debug_assert!(visited.insert(block));
            for suc_block in self.cfg.successors(block) {
                if !visited.contains(suc_block) {
                    to_visit.push(*suc_block);
                }
            }
            if let Some(instr) = last_instr_of_block {
                if !matches!(instr, Bytecode::Jump(..) | Bytecode::Branch(..)) {
                    if let Some(&next_to_visit) = to_visit.last() {
                        let suc_block = Self::get_updated_suc(&subst_block, self.cfg.successors(block)[0]);
                        if suc_block != self.cfg.exit_block() && next_to_visit != suc_block {
                            self.data.code.push(Bytecode::Jump(
                                instr.get_attr_id(),
                                self.get_block_label(suc_block),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Transforms the bytecode by substituting the labels in `subst` in branch and jumps
    fn transform_bytecode(&self, bytecode: Bytecode, subst: &BTreeMap<Label, Label>) -> Bytecode {
        match bytecode {
            Bytecode::Branch(attr_id, l0, l1, t) => Bytecode::Branch(
                attr_id,
                Self::get_updated_label(subst, l0),
                Self::get_updated_label(subst, l1),
                t,
            ),
            Bytecode::Jump(attr_id, label) => {
                Bytecode::Jump(attr_id, Self::get_updated_label(subst, label))
            },
            Bytecode::Label(_, label) => {
                if subst.contains_key(&label) {
                    panic!("label not removed")
                } else {
                    bytecode
                }
            },
            _ => bytecode,
        }
    }

    /// Get the updated label of `label` after block removal
    /// so that any jump to `label` should jump to the updated label instead
    fn get_updated_label(subst: &BTreeMap<Label, Label>, label: Label) -> Label {
        apply_inf(subst, &label).clone()
    }

    /// If a block has successor `block`, then it has the returned block
    /// as successor after block removal
    fn get_updated_suc(subst: &BTreeMap<BlockId, BlockId>, block: BlockId) -> BlockId {
        apply_inf(subst, &block).clone()
    }

    /// Returns the instructions of the block
    fn block_instrs(&self, block_id: BlockId) -> &[Bytecode] {
        self.cfg.content(block_id).to_bytecodes(&self.original_code)
    }

    /// If the given block contains only a label and a jump, returns its successor block;
    /// else returns `None`
    fn is_empty_block(&self, block_id: BlockId) -> bool {
        let block_instrs = self.block_instrs(block_id);
        block_instrs.len() == 2
            && matches!(block_instrs[0], Bytecode::Label(..))
            && matches!(
                block_instrs.last().expect("instruction"),
                Bytecode::Jump(..)
            )
    }

    /// Returns the label of the block
    /// Panics if the block is entry/exit, or doesn't start with a label
    fn get_block_label(&self, block_id: BlockId) -> Label {
        if let Bytecode::Label(_, label) = self
            .block_instrs(block_id)
            .get(0)
            .expect("first instruction")
        {
            label.clone()
        } else {
            panic!("using `get_block_label` on block not starting with a label")
        }
    }

    /// Returns a subst map, where b0 -> b1 if a block jumpping to b0 should jump to b1 because b0 is empty
    fn collect_blocks_to_remove(&self) -> BTreeMap<BlockId, BlockId> {
        let mut subst = BTreeMap::new();
        for block_id in self.cfg.blocks() {
            if self.is_empty_block(block_id) {
                let sucs = self.cfg.successors(block_id);
                debug_assert!(sucs.len() == 1);
                subst.insert(block_id, sucs[0]);
            }
        }
        subst
    }

    /// Remove given blocks in the cfg
    fn remove_blocks(&mut self, subst: &BTreeSet<BlockId>) {
        self.cfg.remove_blocks(subst);
    }
}

/// Let `g` be `f` extended with the identity function.
/// Returns the result of applying `g` infinitely many times to `x`
/// Requires: no loop in `f` (say x -> x, or x -> y, y -> x)
fn apply_inf<'a, T: Ord>(f: &'a BTreeMap<T, T>, x: &'a T) -> &'a T {
    match f.get(&x) {
        Some(y) => apply_inf(f, y),
        None => x,
    }
}

struct ControlFlowGraphCodeGenerator {
    cfg: StacklessControlFlowGraph,
    code_blocks: BTreeMap<BlockId, Vec<Bytecode>>,
}

impl ControlFlowGraphCodeGenerator {
    pub fn new(cfg: StacklessControlFlowGraph, codes: &[Bytecode]) -> Self {
        let code_blocks = cfg
            .iter_dfs_left()
            .map(|block| (block, cfg.content(block).to_bytecodes(&codes).to_vec()))
            .collect();
        Self { cfg, code_blocks }
    }

    /// Generates code from the control flow graph
    fn gen_codes(mut self) -> Vec<Bytecode> {
        let mut generated = Vec::new();
        let mut iter_dfs_left = self.cfg.iter_dfs_left().peekable();
        while let Some(block) = iter_dfs_left.next() {
            if block == self.cfg.entry_block() || block == self.cfg.exit_block() {
                continue;
            }
            let mut code_block = self.code_blocks.remove(&block).expect("code block");
            // if we have block 0 followed by block 1 without jump/branch
            // and we don't visit block 1 after block 0, then we have to add an explicit jump
            if self.falls_to_next_block(&code_block) {
                debug_assert!(self.cfg.successors(block).len() == 1);
                let suc_block = *self.cfg.successors(block).get(0).expect("successor block");
                debug_assert!(
                    suc_block != self.cfg.exit_block(),
                    "path ending without return/abort"
                );
                let maybe_next_to_visit = iter_dfs_left.peek();
                if maybe_next_to_visit.is_none() || *maybe_next_to_visit.unwrap() != suc_block {
                    let attr_id = code_block.last().expect("last instr").get_attr_id();
                    // assuming that any block with a non-trivial predecessor block starts with a label
                    code_block.push(Bytecode::Jump(attr_id, self.get_block_label(suc_block)));
                }
            }
            generated.append(&mut code_block);
        }
        generated
    }

    /// Checks whether a block falls to the next block without jump, branch, abort, or return
    fn falls_to_next_block(&self, codes: &[Bytecode]) -> bool {
        let last_instr = codes.last().expect("last instr");
        !matches!(
            last_instr,
            Bytecode::Jump(..) | Bytecode::Branch(..) | Bytecode::Ret(..) | Bytecode::Abort(..)
        )
    }

    /// Returns the instructions of the block
    fn block_instrs(&self, block_id: BlockId) -> &[Bytecode] {
        self.code_blocks.get(&block_id).expect("block instructions")
    }

    /// Returns the label of the block
    /// Panics if the block is entry/exit, or doesn't start with a label
    fn get_block_label(&self, block_id: BlockId) -> Label {
        if let Bytecode::Label(_, label) = self
            .block_instrs(block_id)
            .get(0)
            .expect("first instruction")
        {
            label.clone()
        } else {
            panic!("block doesn't start with a label")
        }
    }
}
