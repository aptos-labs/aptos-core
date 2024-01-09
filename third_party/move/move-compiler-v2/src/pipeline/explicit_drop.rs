// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// ! Adds explicit destroy instructions for non-primitive types.

use super::{
    livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfo, LiveVarInfoAtCodeOffset},
    reference_safety_processor::{LifetimeAnnotation, LifetimeInfoAtCodeOffset, LifetimeState},
};
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv, ty::Type};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AttrId, Bytecode, Label, Operation},
    stackless_control_flow_graph::{BlockContent, BlockId, StacklessControlFlowGraph},
};
use std::collections::{BTreeMap, BTreeSet};

pub struct ExplicitDrop {}

impl FunctionTargetProcessor for ExplicitDrop {
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
        let mut transformer = ExplicitDropTransformer::new(target);
        transformer.transform();
        data.code = transformer.transformed;
        data.annotations.remove::<LiveVarAnnotation>();
        data.annotations.remove::<LifetimeAnnotation>();
        data
    }

    fn name(&self) -> String {
        "ExplicitDrop".to_owned()
    }
}

struct ExplicitDropTransformer<'a> {
    target: FunctionTarget<'a>,
    // result of the transformation
    transformed: Vec<Bytecode>,
    live_var_annot: &'a LiveVarAnnotation,
    lifetime_annot: &'a LifetimeAnnotation,
    // backward control flow graph
    cfg: StacklessControlFlowGraph,
    // maps code offset of first instruction in block to block ids
    offset_to_block_id: BTreeMap<CodeOffset, BlockId>,
    label_offsets: BTreeMap<Label, CodeOffset>,
    // labels used in the original codes and in the generated codes
    labels: BTreeSet<Label>,
}

impl<'a> ExplicitDropTransformer<'a> {
    pub fn new(target: FunctionTarget<'a>) -> Self {
        let live_var_annot = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("livevar annotation");
        let lifetime_annot = target
            .get_annotations()
            .get::<LifetimeAnnotation>()
            .expect("lifetime annotation");
        let cfg = StacklessControlFlowGraph::new_backward(target.get_bytecode(), true);
        let offset_to_block_id = get_offset_to_block_id(&cfg);
        let label_offsets = Bytecode::label_offsets(target.get_bytecode());
        let labels = label_offsets
            .keys()
            .cloned()
            .collect();
        ExplicitDropTransformer {
            target,
            transformed: Vec::new(),
            live_var_annot,
            lifetime_annot,
            cfg,
            offset_to_block_id,
            label_offsets,
            labels,
        }
    }

    /// Add explicit drop instructions
    /// note that this will invalidate existing analyses
    pub fn transform(&mut self) {
        self.drop_unused_args();
        for (code_offset, bytecode) in self.target.get_bytecode().to_vec().iter().enumerate() {
            self.transform_instr_at(code_offset as CodeOffset, bytecode)
        }
    }

    /// Transforms the instruction `bytecode` at `code_offset`
    pub fn transform_instr_at(&mut self, code_offset: CodeOffset, bytecode: &Bytecode) {
        match bytecode {
            Bytecode::Branch(attr_id, l1, l2, cond) => {
                self.transform_branch_at(code_offset, *attr_id, *l1, *l2, *cond)
            },
            Bytecode::Jump(attr_id, label) => {
                self.transform_jump_at(code_offset, *attr_id, *label)
            },
            Bytecode::Ret(..) | Bytecode::Abort(..) => self.emit_bytecode(bytecode.clone()),
            _ => {
                self.emit_bytecode(bytecode.clone());
                let released_temps = self.released_temps_at(code_offset);
                self.drop_temps(&released_temps, bytecode.get_attr_id())
            },
        }
    }

    fn transform_jump_at(&mut self, code_offset: CodeOffset, attr_id: AttrId, l: Label) {
        if let Some((new_l, codes)) = self.process_jump(code_offset, attr_id, l) {
           self.emit_bytecode(Bytecode::Jump(attr_id, new_l));
           self.emit_bytecodes(codes.into_iter())
        } else {
            self.emit_bytecode(Bytecode::Jump(attr_id, l))
        }
    }

    fn transform_branch_at(&mut self, code_offset: CodeOffset, attr_id: AttrId, l1: Label, l2: Label, cond: TempIndex) {
        match (self.process_jump(code_offset, attr_id, l1), self.process_jump(code_offset, attr_id, l2)) {
            (None, Some((new_l2, codes2))) => {
                self.emit_bytecode(Bytecode::Branch(attr_id, l1, new_l2, cond));
                self.emit_bytecodes(codes2.into_iter())
            },
            (Some((new_l1, codes1)), None) => {
                self.emit_bytecode(Bytecode::Branch(attr_id, new_l1, l2, cond));
                self.emit_bytecodes(codes1.into_iter())
            },
            (Some((new_l1, codes1)), Some((new_l2, codes2))) => {
                self.emit_bytecode(Bytecode::Branch(attr_id, new_l1, new_l2, cond));
                self.emit_bytecodes(codes1.into_iter());
                self.emit_bytecodes(codes2.into_iter())
            },
            (None, None) => {
                self.emit_bytecode(Bytecode::Branch(attr_id, l1, l2, cond))
            },
        }
        // TODO: if we need to drop primitives one day, then we need to drop the locals released by the branch instruction as well
    }

    /// `pred_offset`: refers to an instruction which may jumps to `label`
    /// Returns `None` if no drops are inferred between (after state of) `pred_offset` and (before state of) `label`
    /// Otherwise returns
    /// - a fresh label
    /// - a new code block that
    ///     - starts with the fresh label
    ///     - drops inferred locals
    ///     - jumps to `label`
    pub fn process_jump(&mut self, pred_offset: CodeOffset, attr_id: AttrId, label: Label) -> Option<(Label, Vec<Bytecode>)> {
        let suc_offset = self.label_offsets.get(&label).expect("label offset");
        let temps_to_drop = &self.released_by_lifetime_between(pred_offset, *suc_offset);
        if temps_to_drop.is_empty() {
            None
        } else {
            Some(self.explicit_drops_before_label(label, temps_to_drop, attr_id))
        }
    }

    /// Add explicit drops at the given code offset.
    fn explicit_drops_at(&mut self, code_offset: CodeOffset, bytecode: &Bytecode) {
        let released_temps = self.released_temps_at(code_offset);
        self.drop_temps(&released_temps, bytecode.get_attr_id())
    }

    /// Returns
    /// - a fresh label
    /// - a new code block that
    ///     - starts with the fresh label
    ///     - drops locals in `temps_to_drop`
    ///     - jumps to `label`
    fn explicit_drops_before_label(&mut self, label: Label, temps_to_drop: &BTreeSet<TempIndex>, attr_id: AttrId) -> (Label, Vec<Bytecode>) {
        let new_label = self.gen_fresh_label();
        let mut instrs = Vec::new();
        instrs.push(Bytecode::Label(attr_id, new_label));
        for drop_instr in Self::gen_drops(temps_to_drop, attr_id) {
            instrs.push(drop_instr);
        }
        instrs.push(Bytecode::Jump(attr_id, label));
        (new_label, instrs)
    }

    /// Generates a fresh label
    fn gen_fresh_label(&mut self) -> Label {
        let new_label = Label::new(
            if self.labels.is_empty() {
                0
            } else {
                self.labels.iter().next_back().expect("label").as_usize() + 1
            },
        );
        self.labels.insert(new_label);
        new_label
    }

    /// Checks if the given local is of primitive type
    fn is_primitive(&self, t: TempIndex) -> bool {
        matches!(self.target.get_local_type(t), Type::Primitive(_))
    }

    /// Drops unused function arguments
    fn drop_unused_args(&mut self) {
        let start_code_offset = 0;
        let live_var_info = self.get_live_var_info(start_code_offset);
        let lifetime_info = self.get_lifetime_info(start_code_offset);
        for arg in self.target.get_parameters() {
            if !self.is_primitive(arg)
                && !live_var_info.before.contains_key(&arg)
                && !lifetime_info.before.is_borrowed(arg)
            {
                // a non-native function has at least one instruction; a single return or abort at minimum
                let attr_id = self.target.get_bytecode()[start_code_offset as usize].get_attr_id();
                self.drop_temp(arg, attr_id)
            }
        }
    }

    /// Returns the set of locals released by the control flow edge from `pred_offset` to `suc_offset`
    /// I.e., alive after `pred_offset` but dead before `suc_offset`, and not borrowed before `suc_offset`
    fn released_temps_between(&self, pred_offset: CodeOffset, suc_offset: CodeOffset) -> BTreeSet<TempIndex> {
        let mut released = BTreeSet::new();
        let lifetime_after = &self.get_lifetime_info(suc_offset).before;
        for t in dead_and_unborrowed(
            self.released_by_live_var_between(pred_offset, suc_offset)
                .into_iter(),
            lifetime_after,
        ) {
            released.insert(t);
        }
        let live_var_after = &self.get_live_var_info(suc_offset).before;
        for t in unborrowed_and_dead(
            self.released_by_lifetime_between(pred_offset, suc_offset)
                .into_iter(),
            live_var_after,
        ) {
            released.insert(t);
        }
        released
    }

    // Returns a set of locals that can be dropped at given code offset
    // Primitives are filtered out
    fn released_temps_at(&self, code_offset: CodeOffset) -> BTreeSet<TempIndex> {
        let live_var_info = self.get_live_var_info(code_offset);
        let lifetime_info = self.get_lifetime_info(code_offset);
        let bytecode = &self.target.get_bytecode()[code_offset as usize];
        released_temps(live_var_info, lifetime_info, bytecode)
            .into_iter()
            .filter(|t| !self.is_primitive(*t))
            .collect()
    }

    /// Returns the locals alive after `pred_offset` and not before `suc_offset`
    fn released_by_live_var_between(
        &self,
        pred_offset: CodeOffset,
        suc_offset: CodeOffset,
    ) -> BTreeSet<TempIndex> {
        let live_before = &self.get_live_var_info(pred_offset).after;
        let live_after = &self.get_live_var_info(suc_offset).before;
        LiveVarInfoAtCodeOffset::live_var_diff(live_before, live_after).collect()
    }

    /// Returns the locals borrowed after `pred_offset` and not before `suc_offset`
    fn released_by_lifetime_between(
        &self,
        pred_offset: CodeOffset,
        suc_offset: CodeOffset,
    ) -> BTreeSet<TempIndex> {
        let lifetime_before = &self.get_lifetime_info(pred_offset).after;
        let lifetime_after = &self.get_lifetime_info(suc_offset).before;
        lifetime_before.lifetime_diff(lifetime_after).collect()
    }

    fn get_live_var_info(&self, code_offset: CodeOffset) -> &'a LiveVarInfoAtCodeOffset {
        self.live_var_annot
            .get_live_var_info_at(code_offset)
            .expect("live var info")
    }

    fn get_lifetime_info(&self, code_offset: CodeOffset) -> &'a LifetimeInfoAtCodeOffset {
        self.lifetime_annot.get_info_at(code_offset)
    }

    /// Returns the codeoffsets of the predecessors of the given instruction.
    /// The given instruction should be at the beginning of a block.
    fn get_pred_instr_offsets(&self, code_offset: CodeOffset) -> Vec<CodeOffset> {
        let block_id = self.offset_to_block_id.get(&code_offset).expect("block id");
        self.cfg
            .successors(*block_id)
            .iter()
            .filter_map(|block_id| {
                if let BlockContent::Basic { upper, .. } = self.cfg.content(*block_id) {
                    Some(*upper)
                } else {
                    None
                }
            })
            .collect()
    }

    fn gen_drop(tmp: TempIndex, attr_id: AttrId) -> Bytecode {
        Bytecode::Call(attr_id, Vec::new(), Operation::Destroy, vec![tmp], None)
    }

    fn drop_temp(&mut self, tmp: TempIndex, attr_id: AttrId) {
        let drop_t = Self::gen_drop(tmp, attr_id);
        self.emit_bytecode(drop_t)
    }

    /// Iterators over instructions dropping all locals in `temps_to_drop`
    fn gen_drops(temps_to_drop: & BTreeSet<TempIndex>, attr_id: AttrId) -> impl Iterator<Item = Bytecode> + '_ {
        temps_to_drop
            .iter()
            .map(move |t| Self::gen_drop(*t, attr_id))
    }

    fn drop_temps(&mut self, temps_to_drop: &BTreeSet<TempIndex>, attr_id: AttrId) {
        for t in temps_to_drop {
            self.drop_temp(*t, attr_id)
        }
    }

    fn emit_bytecode(&mut self, bytecode: Bytecode) {
        self.transformed.push(bytecode)
    }

    fn emit_bytecodes(&mut self, bytecodes: impl Iterator<Item = Bytecode>) {
        for bytecode in bytecodes {
            self.emit_bytecode(bytecode)
        }
    }
}

// Returns a set of locals that can be dropped
// these are the ones no longer alive or borrowed
// including locals of primitives
fn released_temps(
    live_var_info: &LiveVarInfoAtCodeOffset,
    life_time_info: &LifetimeInfoAtCodeOffset,
    bytecode: &Bytecode,
) -> BTreeSet<TempIndex> {
    // use set to avoid duplicate dropping
    let mut released_temps = BTreeSet::new();
    for t in dead_and_unborrowed(live_var_info.released_temps(), &life_time_info.after) {
        released_temps.insert(t);
    }
    for t in unborrowed_and_dead(life_time_info.released_temps(), &live_var_info.after) {
        released_temps.insert(t);
    }
    // if a temp is moved, then no need to drop
    // this should come before the calculation
    // of unused vars; because of, for instance,
    // x = move(x)
    released_temps.retain(|t| !life_time_info.is_moved(*t));

    // this is needed because unused vars are not released by live var info
    for dst in bytecode.dests() {
        if !live_var_info.before.contains_key(&dst)
            && !live_var_info.after.contains_key(&dst)
            && !life_time_info.before.is_borrowed(dst)
            && !life_time_info.after.is_borrowed(dst)
        {
            // TODO: triggered in ability-checker/ability_violation.move
            // debug_assert!(
            //     !life_time_info.after.is_borrowed(dst),
            //     "dead assignment borrowed later"
            // );
            released_temps.insert(dst);
        }
    }
    released_temps
}

// Return a map mapping code offsets of the first instruction in blocks to their block id
fn get_offset_to_block_id(cfg: &StacklessControlFlowGraph) -> BTreeMap<CodeOffset, BlockId> {
    let mut code_offset_to_block_id = BTreeMap::new();
    for block_id in cfg.blocks() {
        if let BlockContent::Basic { lower, .. } = cfg.content(block_id) {
            assert!(code_offset_to_block_id.insert(*lower, block_id).is_none())
        }
    }
    code_offset_to_block_id
}

/// Iterates over the locals released by live var analysis, and not borrowed in `lifetime_after`
fn dead_and_unborrowed<'a>(
    released_by_live_var: impl Iterator<Item = TempIndex> + 'a,
    lifetime_after: &'a LifetimeState,
) -> impl Iterator<Item = TempIndex> + 'a {
    released_by_live_var
        .into_iter()
        .filter(|t| !lifetime_after.is_borrowed(*t))
}

/// Iterates over the locals released by live var analysis, and not borrowed in `lifetime_after`
fn unborrowed_and_dead<'a>(
    released_by_lifetime: impl Iterator<Item = TempIndex> + 'a,
    live_var_after: &'a BTreeMap<TempIndex, LiveVarInfo>,
) -> impl Iterator<Item = TempIndex> + 'a {
    released_by_lifetime
        .into_iter()
        .filter(|t| !live_var_after.contains_key(t))
}
