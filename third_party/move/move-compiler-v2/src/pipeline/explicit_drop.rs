// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// ! Adds explicit destroy instructions for non-primitive types.

use super::{
    livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfoAtCodeOffset},
    reference_safety_processor::{LifetimeAnnotation, LifetimeInfoAtCodeOffset},
};
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv, ty::Type};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AttrId, Bytecode, Operation},
};
use std::collections::BTreeSet;

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
        ExplicitDropTransformer {
            target,
            transformed: Vec::new(),
            live_var_annot,
            lifetime_annot,
        }
    }

    /// Add explicit drop instructions
    /// note that this will invalidate existing analyses
    pub fn transform(&mut self) {
        self.drop_unused_args();
        for (code_offset, bytecode) in self.target.get_bytecode().to_vec().iter().enumerate() {
            self.emit_bytecode(bytecode.clone());
            self.explicit_drops_at(code_offset as CodeOffset, bytecode);
        }
    }

    /// Add explicit drops at the given code offset.
    fn explicit_drops_at(&mut self, code_offset: CodeOffset, bytecode: &Bytecode) {
        match bytecode {
            Bytecode::Ret(..) | Bytecode::Jump(..) | Bytecode::Abort(..) | Bytecode::Branch(..) => {
            },
            _ => {
                let released_temps = self.released_temps_at(code_offset);
                self.drop_temps(&released_temps, bytecode.get_attr_id())
            },
        }
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

    fn get_live_var_info(&self, code_offset: CodeOffset) -> &'a LiveVarInfoAtCodeOffset {
        self.live_var_annot
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
// including locals of primitives
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
