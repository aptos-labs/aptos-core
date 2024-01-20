use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Label},
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
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
		let mut transformer = EliminateEmptyBlocksTransformation::new(data);
		transformer.transform();
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
			original_code
		}
	}

	fn transform(&mut self) {
		let subst = self.collect_blocks_to_remove();
		let blocks_to_remove: BTreeSet<BlockId> = subst.keys().cloned().collect();
		let subst = subst.into_iter().map(|(b0, b1)| (self.get_block_label(b0), self.get_block_label(b1))).collect();
		self.remove_blocks(&blocks_to_remove);
		self.gen_code_from_cfg(&subst);
	}

	/// Generate code from a cfg where empty blocks have been removed
	fn gen_code_from_cfg(&mut self, subst: &BTreeMap<Label, Label>) {
        let mut visited = BTreeSet::new();
        let mut to_visit = vec![self.cfg.entry_block()];
        while let Some(block) = to_visit.pop() {
            for bytecode in self.cfg.content(block).to_bytecodes(&self.original_code) {
                self.data.code.push(self.transform_bytecode(bytecode.clone(), subst));
            }
            debug_assert!(visited.insert(block));
            for suc_block in self.cfg.successors(block) {
                if !visited.contains(suc_block) {
                    to_visit.push(*suc_block);
                }
            }
        }
	}

	/// Transforms the bytecode by substituting the labels in `subst` in branch and jumps
    fn transform_bytecode(&self, bytecode: Bytecode, subst: &BTreeMap<Label, Label>) -> Bytecode {
        match bytecode {
            Bytecode::Branch(attr_id, l0, l1, t) => Bytecode::Branch(
                attr_id,
                Self::subst_label(subst, l0),
                Self::subst_label(subst, l1),
                t,
            ),
            Bytecode::Jump(attr_id, label) => {
                Bytecode::Jump(attr_id, Self::subst_label(subst, label))
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

    /// Returns label substituted if it's in `subst`; else just returns `label`
    fn subst_label(subst: &BTreeMap<Label, Label>, label: Label) -> Label {
		subst.get(&label).cloned().unwrap_or_else(|| label)
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
