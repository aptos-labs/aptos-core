// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// ! Adds explicit drop and release instructions for non-primitive types.

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
                let (released_temps, dropped_temps) =
                    self.released_and_dropped_temps_at(code_offset);
                self.release_or_drop_temps(&released_temps, bytecode.get_attr_id(), true);
                self.release_or_drop_temps(&dropped_temps, bytecode.get_attr_id(), false);
            },
        }
    }

    /// Checks if the given local is of primitive type
    fn is_primitive(&self, t: TempIndex) -> bool {
        matches!(self.target.get_local_type(t), Type::Primitive(_))
    }

    /// Checks if the given local is of reference type
    fn is_reference(&self, t: TempIndex) -> bool {
        self.target.get_local_type(t).is_reference()
    }

    /// Drops unused function arguments. We do not need to consider release operations here because they are never
    /// borrowed from.
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
                self.release_or_drop_temp(arg, attr_id, false)
            }
        }
    }

    // Returns a set of locals that should be released or dropped at given code offset
    // Primitives are filtered out
    fn released_and_dropped_temps_at(
        &self,
        code_offset: CodeOffset,
    ) -> (BTreeSet<TempIndex>, BTreeSet<TempIndex>) {
        let live_var_info = self.get_live_var_info(code_offset);
        let lifetime_info = self.get_lifetime_info(code_offset);
        let bytecode = &self.target.get_bytecode()[code_offset as usize];
        self.released_and_dropped_temps(live_var_info, lifetime_info, bytecode)
    }

    fn get_live_var_info(&self, code_offset: CodeOffset) -> &'a LiveVarInfoAtCodeOffset {
        self.live_var_annot
            .get_live_var_info_at(code_offset)
            .expect("live var info")
    }

    fn get_lifetime_info(&self, code_offset: CodeOffset) -> &'a LifetimeInfoAtCodeOffset {
        self.lifetime_annot.get_info_at(code_offset)
    }

    /// Release or drop a temporary if its type is not primitive.
    fn release_or_drop_temp(&mut self, tmp: TempIndex, attr_id: AttrId, release: bool) {
        if !self.is_primitive(tmp) {
            let instr = Bytecode::Call(
                attr_id,
                Vec::new(),
                if release {
                    Operation::Release
                } else {
                    Operation::Drop
                },
                vec![tmp],
                None,
            );
            self.emit_bytecode(instr)
        }
    }

    fn release_or_drop_temps(
        &mut self,
        temps_to_drop: &BTreeSet<TempIndex>,
        attr_id: AttrId,
        release: bool,
    ) {
        for t in temps_to_drop {
            self.release_or_drop_temp(*t, attr_id, release)
        }
    }

    fn emit_bytecode(&mut self, bytecode: Bytecode) {
        self.transformed.push(bytecode)
    }

    /// Returns sets of locals which should be released and dropped at this program point.
    /// See comments in the code.
    fn released_and_dropped_temps(
        &self,
        live_var_info: &LiveVarInfoAtCodeOffset,
        life_time_info: &LifetimeInfoAtCodeOffset,
        bytecode: &Bytecode,
    ) -> (BTreeSet<TempIndex>, BTreeSet<TempIndex>) {
        // use sets to avoid duplicate dropping
        let mut released_temps = BTreeSet::new();
        let mut dropped_temps = BTreeSet::new();
        // Get the temps dropped at this program point, including those which are introduced here but never used.
        // Exclude local values which are borrowed.
        for t in live_var_info.released_and_unused_temps(bytecode) {
            if !life_time_info.after.is_borrowed(t) || self.is_reference(t) {
                // The local gets out of scope and is either not borrowed from or a reference, so drop it.
                dropped_temps.insert(t);
            }
        }
        // Get the temps which are released according to live var info.
        for t in life_time_info.released_temps() {
            if !live_var_info.after.contains_key(&t)
                && !dropped_temps.contains(&t)
                && !self.is_reference(t)
            {
                // The local is not longer alive but borrowed; that borrow can now be released
                released_temps.insert(t);
            }
        }
        // if a temp is moved, then no need to drop
        // this should not remove unused vars; because of, for instance,
        // x = move(x)
        dropped_temps.retain(|t| {
            let is_used =
                live_var_info.before.contains_key(t) || live_var_info.after.contains_key(t);
            !is_used || !life_time_info.is_moved(*t)
        });

        (released_temps, dropped_temps)
    }
}
