use super::{
    livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfoAtCodeOffset},
    reference_safety_processor::{LifetimeAnnotation, LifetimeInfoAtCodeOffset},
};
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AttrId, Bytecode, Label, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct ExplicateDrop {}

impl FunctionTargetProcessor for ExplicateDrop {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() {
            return data;
        }
        let mut transformer = ExplicateDropTransformer::new(std::mem::take(&mut data.code), &data);
        transformer.transform();
        data.code = transformer.codes;
        data
    }

    fn name(&self) -> String {
        "ExplicateDrop".to_owned()
    }
}

struct ExplicateDropTransformer<'a> {
    codes: Vec<Bytecode>,
    live_var_annot: &'a LiveVarAnnotation,
    lifetime_annot: &'a LifetimeAnnotation,
    label_offsets: BTreeMap<Label, CodeOffset>,
    cfg: StacklessControlFlowGraph,
}

impl<'a> ExplicateDropTransformer<'a> {
    pub fn new(codes: Vec<Bytecode>, fun_data: &'a FunctionData) -> Self {
        let live_var_annot = fun_data
            .annotations
            .get::<LiveVarAnnotation>()
            .expect("livevar annotation");
        let lifetime_annot = fun_data
            .annotations
            .get::<LifetimeAnnotation>()
            .expect("lifetime annotation");
        let label_offsets = Bytecode::label_offsets(&codes);
        let cfg = StacklessControlFlowGraph::new_backward(&codes, true);
        ExplicateDropTransformer {
            codes,
            live_var_annot,
            lifetime_annot,
            label_offsets,
            cfg,
        }
    }

    /// Add explicit drop instructions
    /// note that this will invalidate existing analyses
    pub fn transform(&mut self) {
        let bytecodes = std::mem::take(&mut self.codes);
        for (code_offset, bytecode) in bytecodes.into_iter().enumerate() {
            self.emit_bytecode(bytecode.clone());
            self.explicate_drops_at(code_offset as CodeOffset, &bytecode);
        }
    }

    // add drops at given code offset
    fn explicate_drops_at(&mut self, code_offset: CodeOffset, bytecode: &Bytecode) {
        match bytecode {
            Bytecode::Ret(..) | Bytecode::Jump(..) | Bytecode::Abort(..) | Bytecode::Branch(..) => {
                ()
            },
            Bytecode::Label(_, label) => {
                // all locals released by (immediate) predec instructions
                let released_temps_join: BTreeSet<_> = self
                    .pred_instr_offsets(label)
                    .flat_map(|pred_instr_offset| self.released_temps_at(pred_instr_offset))
                    .collect();
                self.drop_temps(&released_temps_join, bytecode.get_attr_id())
            },
            _ => {
                let released_temps = self.released_temps_at(code_offset);
                self.drop_temps(&released_temps, bytecode.get_attr_id())
            },
        }
    }

    // return the offsets of the instructions which jump to the given label
    fn pred_instr_offsets(&self, label: &Label) -> impl Iterator<Item = CodeOffset> + '_ {
        let offset = self.label_offsets.get(label).expect("label offset");
        let block_id = self.cfg.offset_to_key().get(offset).expect("block id");
        self.cfg.successors(*block_id).iter().map(|pred_block| {
            // last instr of the pred block
            self.cfg
                .instr_indexes(*pred_block)
                .expect("basic block")
                .last()
                .expect("code offset")
        })
    }

    // Returns a set of locals that can be dropped at given code offset
    fn released_temps_at(&self, code_offset: CodeOffset) -> BTreeSet<TempIndex> {
        let live_var_info = self
            .live_var_annot
            .get_live_var_info_at(code_offset)
            .expect("live var info");
        let lifetime_info = self.lifetime_annot.get_info_at(code_offset);
        released_temps(live_var_info, lifetime_info)
    }

    fn drop_temps(&mut self, temps_to_drop: &BTreeSet<TempIndex>, attr_id: AttrId) {
        for t in temps_to_drop {
            let drop_t = Bytecode::Call(
                attr_id,
                Vec::new(),
                Operation::Destroy,
                vec![*t],
                None,
            );
            self.emit_bytecode(drop_t)
        }
    }

    fn emit_bytecode(&mut self, bytecode: Bytecode) {
        self.codes.push(bytecode)
    }
}

// Returns a set of locals that can be dropped
// these are the ones no longer alive or borrowed
fn released_temps(
    live_var_info: &LiveVarInfoAtCodeOffset,
    life_time_info: &LifetimeInfoAtCodeOffset,
) -> BTreeSet<TempIndex> {
    // use set to avoid duplicate dropping
    let mut released_temps = BTreeSet::new();
    for t in live_var_info.released_temps() {
        if !life_time_info.after.is_borrowed(t) {
            released_temps.insert(t);
        }
    }
    for t in life_time_info.released_temps() {
        if !live_var_info.after.contains_key(&t) {
            released_temps.insert(t);
        }
    }
    released_temps
}
