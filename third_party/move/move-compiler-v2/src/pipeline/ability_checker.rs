//! Checks for ability violations.

use move_binary_format::file_format::{Ability, AbilitySet};
use move_model::{model::{FunctionEnv, Loc}, ty::Type, ast::TempIndex};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AssignKind, AttrId, Bytecode, Operation},
};

// Returns the abilities of the given type.
fn type_abilities(func_target: &FunctionTarget, ty: &Type) -> AbilitySet {
    let ty_params = func_target.get_type_parameters();
    let global_env = func_target.global_env();
    global_env.type_abilities(ty, &ty_params)
}

// Determines if the given type has constraint copy.
fn has_copy(func_target: &FunctionTarget, ty: &Type) -> bool {
    type_abilities(func_target, ty).has_ability(Ability::Copy)
}

// Checks if the given type has constraint copy, and add diagnostics if not.
fn check_copy(func_target: &FunctionTarget, ty: &Type, loc: &Loc, err_msg: &str) {
	if !has_copy(func_target, ty) {
		func_target.global_env().error(loc, err_msg)
	}
}

// Checks if the given temporary variable has constraint copy, and add diagnostics if not.
fn check_copy_for_temp(func_target: &FunctionTarget, t: TempIndex, loc: &Loc, err_msg: &str) {
	let ty = func_target.get_local_type(t);
	check_copy(func_target, ty, loc, err_msg)
}

/// Determines if the given type has the drop constraint.
fn has_drop(func_target: &FunctionTarget, ty: &Type) -> bool {
    type_abilities(func_target, ty).has_ability(Ability::Drop)
}

pub struct AbilityChecker();

impl FunctionTargetProcessor for AbilityChecker {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        let fun_target = FunctionTarget::new(fun_env, &data);
        todo!()
    }

    fn name(&self) -> String {
        "Ability Checker".to_owned()
    }
}

fn check_bytecode(target: &FunctionTarget, bytecode: &Bytecode) {
    match bytecode {
        Bytecode::Assign(attr_id, src, dst, kind) => {
            // let src_ty = target.get_local_type(*src);
            // if !has_drop(target, src_ty) {
            //     let loc = target.get_bytecode_loc(*attr_id);
            //     target.global_env().error(&loc, &format!("cannot drop"))
            // }

            if matches!(kind, AssignKind::Copy | AssignKind::Store) {
                let src_ty = target.get_local_type(*src);
                if !has_copy(target, src_ty) {
                    let loc = target.get_bytecode_loc(*attr_id);
                    target.global_env().error(&loc, &format!("cannot copy"))
                }
            }
        },
        Bytecode::Call(attr_id, dsts, op, srcs, _) => {
            use Operation::*;
			let loc = target.get_bytecode_loc(*attr_id);
            match op {
                Function(_, _, _) => todo!(),
                Pack(_, _, _) => todo!(),
                Unpack(_, _, _) => todo!(),
                MoveTo(_, _, _) => todo!(),
                MoveFrom(_, _, _) => todo!(),
                Exists(_, _, _) => todo!(),
                BorrowLoc => todo!(),
                BorrowField(_, _, _, _) => todo!(),
                BorrowGlobal(_, _, _) => todo!(),
                Destroy => todo!(),
                ReadRef => {
					// todo: may need to check drop for dst
					let src = srcs[0];
					check_copy_for_temp(
						target,
						src, &loc,
						"cannot copy"
					)
				}
                WriteRef => todo!(),
                FreezeRef => todo!(),
                Vector => todo!(),
                CastU8 => todo!(),
                CastU16 => todo!(),
                CastU32 => todo!(),
                CastU64 => todo!(),
                CastU128 => todo!(),
                Not => todo!(),
                Add => todo!(),
                Sub => todo!(),
                Mul => todo!(),
                Div => todo!(),
                Mod => todo!(),
                BitOr => todo!(),
                BitAnd => todo!(),
                Xor => todo!(),
                Shl => todo!(),
                Shr => todo!(),
                Lt => todo!(),
                Gt => todo!(),
                Le => todo!(),
                Ge => todo!(),
                Or => todo!(),
                And => todo!(),
                Eq => todo!(),
                Neq => todo!(),
                CastU256 => todo!(),
                OpaqueCallBegin(_, _, _) => todo!(),
                OpaqueCallEnd(_, _, _) => todo!(),
                IsParent(_, _) => todo!(),
                WriteBack(_, _) => todo!(),
                UnpackRef => todo!(),
                PackRef => todo!(),
                UnpackRefDeep => todo!(),
                PackRefDeep => todo!(),
                GetField(_, _, _, _) => todo!(),
                GetGlobal(_, _, _) => todo!(),
                Uninit => todo!(),
                Havoc(_) => todo!(),
                Stop => todo!(),
                TraceLocal(_) => todo!(),
                TraceReturn(_) => todo!(),
                TraceAbort => todo!(),
                TraceExp(_, _) => todo!(),
                TraceGlobalMem(_) => todo!(),
                EmitEvent => todo!(),
                EventStoreDiverge => todo!(),
            }
        },
        _ => (),
    }
}
