use super::{
    livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfoAtCodeOffset, LiveVarAnalysisProcessor},
    reference_safety_processor::{LifetimeAnnotation, LifetimeInfoAtCodeOffset},
};
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AttrId, Bytecode, Label, Operation, AssignKind},
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
        let target = FunctionTarget::new(fun_env, &data);
        let mut transformer = ExplicateDropTransformer::new(target);
        transformer.transform();
        data.code = transformer.transformed;
        data.annotations.remove::<LiveVarAnnotation>();
        data.annotations.remove::<LifetimeAnnotation>();
        data
    }

    fn name(&self) -> String {
        "ExplicateDrop".to_owned()
    }
}

struct ExplicateDropTransformer<'a> {
    target: FunctionTarget<'a>,
    // result of the transformation
    transformed: Vec<Bytecode>,
    live_var_annot: &'a LiveVarAnnotation,
    lifetime_annot: &'a LifetimeAnnotation,
    label_offsets: BTreeMap<Label, CodeOffset>,
    cfg: StacklessControlFlowGraph,
}

impl<'a> ExplicateDropTransformer<'a> {
    pub fn new(target: FunctionTarget<'a>) -> Self {
        let live_var_annot = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("livevar annotation");
        let lifetime_annot = target
            .get_annotations()
            .get::<LifetimeAnnotation>()
            .expect("lifetime annotation");
        let label_offsets = Bytecode::label_offsets(target.get_bytecode());
        let cfg = StacklessControlFlowGraph::new_backward(target.get_bytecode(), true);
        ExplicateDropTransformer {
            target,
            transformed: Vec::new(),
            live_var_annot,
            lifetime_annot,
            label_offsets,
            cfg,
        }
    }

    /// Add explicit drop instructions
    /// note that this will invalidate existing analyses
    pub fn transform(&mut self) {
        self.drop_unused_args();
        for (code_offset, bytecode) in self.target.get_bytecode().to_vec().iter().enumerate() {
            self.emit_bytecode(bytecode.clone());
            self.explicate_drops_at(code_offset as CodeOffset, bytecode);
        }
    }

    // add drops at given code offset
    fn explicate_drops_at(&mut self, code_offset: CodeOffset, bytecode: &Bytecode) {
        match bytecode {
            Bytecode::Ret(..) | Bytecode::Jump(..) | Bytecode::Abort(..) | Bytecode::Branch(..) => {
                ()
            },
            Bytecode::Label(_, label) => {
                // all locals released by (immediate) preceding instructions
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

    fn drop_unused_args(&mut self) {
        let code_offset = 0;
        let live_var_info = self.get_live_var_info(code_offset);
        let lifetime_info = self.get_lifetime_info(code_offset);
        for arg in self.target.get_parameters() {
            if !live_var_info.before.contains_key(&arg) && !lifetime_info.before.is_borrowed(arg) {
                // todo
                let attr_id = self.target.get_bytecode()[0].get_attr_id();
                self.drop_temp(arg, attr_id)
            }
        }
    }

    // Returns a set of locals that can be dropped at given code offset
    fn released_temps_at(&self, code_offset: CodeOffset) -> BTreeSet<TempIndex> {
        let live_var_info = self.get_live_var_info(code_offset);
        let lifetime_info = self.get_lifetime_info(code_offset);
        let bytecode = &self.target.get_bytecode()[code_offset as usize];
        released_temps(live_var_info, lifetime_info, bytecode)
    }

    fn get_live_var_info(&self, code_offset: CodeOffset) -> &'a LiveVarInfoAtCodeOffset {
        self
            .live_var_annot
            .get_live_var_info_at(code_offset)
            .expect("live var info")
    }

    fn get_lifetime_info(&self, code_offset: CodeOffset) -> &'a LifetimeInfoAtCodeOffset {
        self.lifetime_annot.get_info_at(code_offset)
    }

    fn drop_temp(&mut self, tmp: TempIndex, attr_id: AttrId) {
        let drop_t = Bytecode::Call(attr_id, Vec::new(), Operation::Destroy, vec![tmp], None);
        self.emit_bytecode(drop_t)
    }

    fn drop_temps(&mut self, temps_to_drop: &BTreeSet<TempIndex>, attr_id: AttrId) {
        for t in temps_to_drop {
            self.drop_temp(*t, attr_id)
        }
    }

    fn emit_bytecode(&mut self, bytecode: Bytecode) {
        self.transformed.push(bytecode)
    }
}

// Returns a set of locals that can be dropped
// these are the ones no longer alive or borrowed
fn released_temps(
    live_var_info: &LiveVarInfoAtCodeOffset,
    life_time_info: &LifetimeInfoAtCodeOffset,
    bytecode: &Bytecode,
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
    // if a temp is moved, then no need to drop
    // this should come before the calculation
    // of unused vars; because of, for instance,
    // x = move(x)
    for moved_src in moved_srcs(bytecode) {
        released_temps.remove(&moved_src);
    }
    // this is needed because unused vars are not released by live var info
    for dst in bytecode.dests() {
        if !live_var_info.after.contains_key(&dst) {
            debug_assert!(
                !life_time_info.after.is_borrowed(dst),
                "dead assignment borrowed later"
            );
            released_temps.insert(dst);
        }
    }
    released_temps
}

fn moved_srcs(bytecode: &Bytecode) -> Vec<TempIndex> {
    match bytecode {
        Bytecode::Assign(_, _, src, AssignKind::Move) => vec![*src],
        Bytecode::Call(_, _, _, srcs, _) => srcs.clone(),
        Bytecode::Branch(_, _, _, src) => vec![*src],
        Bytecode::Ret(_, srcs) => srcs.clone(),
        Bytecode::Abort(_, src) => vec![*src],
        _ => Vec::new(),
    }
}
