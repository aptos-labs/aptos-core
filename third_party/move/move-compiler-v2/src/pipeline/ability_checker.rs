//! Checks for ability violations.

use move_binary_format::file_format::{Ability, AbilitySet};
use move_model::{model::{FunctionEnv, Loc, QualifiedId, ModuleId, StructId, FunId}, ty::Type, ast::TempIndex};
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
fn check_copy_for_temp_with_msg(func_target: &FunctionTarget, t: TempIndex, loc: &Loc, err_msg: &str) {
	let ty = func_target.get_local_type(t);
	check_copy(func_target, ty, loc, err_msg)
}

fn check_read_ref(target: &FunctionTarget, t: TempIndex, loc: &Loc) {
    if let Type::Reference(_, ty) = target.get_local_type(t) {
        check_copy(target, ty, loc, "cannot copy")
    } else {
        panic!("ICE from ability checker: read_ref has non-reference argument")
    }
}

/// Determines if the given type has the drop constraint.
fn has_drop(func_target: &FunctionTarget, ty: &Type) -> bool {
    type_abilities(func_target, ty).has_ability(Ability::Drop)
}

// Checks if the given type has constraint drop, and add diagnostics if not.
fn check_drop(func_target: &FunctionTarget, ty: &Type, loc: &Loc, err_msg: &str) {
	if !has_drop(func_target, ty) {
		func_target.global_env().error(loc, err_msg)
	}
}

// Checks if the given temporary variable has constraint drop, and add diagnostics if not.
fn check_drop_for_temp_with_msg(func_target: &FunctionTarget, t: TempIndex, loc: &Loc, err_msg: &str) {
	let ty = func_target.get_local_type(t);
	check_drop(func_target, ty, loc, err_msg)
}

// `t` is the local containing the reference being written to
fn check_write_ref(target: &FunctionTarget, t: TempIndex, loc: &Loc) {
    if let Type::Reference(_, ty) = target.get_local_type(t) {
        // todo: check key, store
        check_drop(target, ty, loc, "cannot drop")
    } else {
        panic!("ICE typing error")
    }
}

/// Determines if the given type has the store constraint.
fn has_store(func_target: &FunctionTarget, ty: &Type) -> bool {
    type_abilities(func_target, ty).has_ability(Ability::Store)
}

// Checks if the given type has constraint store, and add diagnostics if not.
fn check_store(func_target: &FunctionTarget, ty: &Type, loc: &Loc, err_msg: &str) {
	if !has_store(func_target, ty) {
		func_target.global_env().error(loc, err_msg)
	}
}

// Checks if the given temporary variable has constraint store, and add diagnostics if not.
fn check_store_for_temp_with_msg(func_target: &FunctionTarget, t: TempIndex, loc: &Loc, err_msg: &str) {
	let ty = func_target.get_local_type(t);
	check_store(func_target, ty, loc, err_msg)
}

/// Determines if the given type has the key constraint.
fn has_key(func_target: &FunctionTarget, ty: &Type) -> bool {
    type_abilities(func_target, ty).has_ability(Ability::Key)
}

// Checks if the given type has constraint key, and add diagnostics if not.
fn check_key(func_target: &FunctionTarget, ty: &Type, loc: &Loc, err_msg: &str) {
	if !has_key(func_target, ty) {
		func_target.global_env().error(loc, err_msg)
	}
}

// Checks if the given temporary variable has constraint key, and add diagnostics if not.
fn check_key_for_temp_with_msg(func_target: &FunctionTarget, t: TempIndex, loc: &Loc, err_msg: &str) {
	let ty = func_target.get_local_type(t);
	check_key(func_target, ty, loc, err_msg)
}

// checks that the given type is instantiated with types satisfying their ability constraints
// on the type parameter
fn check_struct_inst(target: &FunctionTarget, mid: ModuleId, sid: StructId, inst: &[Type], loc: &Loc) {
    let qid = QualifiedId{ module_id: mid, id: sid };
    let struct_env = target.global_env().get_struct(qid);
    for (param, ty) in struct_env.get_type_parameters().iter().zip(inst.iter()) {
        let required_abilities = param.1.abilities;
        let given_abilities = type_abilities(target, ty);
        // todo: which field, why
        if !required_abilities.is_subset(given_abilities) {
            target.global_env().error(loc, "invalid instantiation")
        }
    }
}

fn check_fun_inst(target: &FunctionTarget, mid: ModuleId, fid: FunId, inst: &[Type], loc: &Loc) {
    let qid = QualifiedId{ module_id: mid, id: fid };
    let fun_env = target.global_env().get_function(qid);
    for (param, ty) in fun_env.get_type_parameters().iter().zip(inst.iter()) {
        let required_abilities = param.1.abilities;
        let given_abilities = type_abilities(target, ty);
        // todo: which field, why
        if !required_abilities.is_subset(given_abilities) {
            target.global_env().error(loc, "invalid instantiation")
        }
    }
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
        if fun_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(fun_env, &data);
        for bytecode in target.get_bytecode() {
            check_bytecode(&target, bytecode)
        }
        data
    }

    fn name(&self) -> String {
        "AbilityChecker".to_owned()
    }
}

fn check_bytecode(target: &FunctionTarget, bytecode: &Bytecode) {
    let loc = target.get_bytecode_loc(bytecode.get_attr_id());
    match bytecode {
        // drop of dst during the assignment has been made explicit
        // so we don't check it here, plus this could be an initialization
        Bytecode::Assign(_, src, _, kind) => {
            if matches!(kind, AssignKind::Copy | AssignKind::Store) {
                check_copy_for_temp_with_msg(target, *src, &loc, "cannot copy");
            }
        },
        Bytecode::Call(attr_id, dsts, op, srcs, _) => {
            use Operation::*;
			let loc = target.get_bytecode_loc(*attr_id);
            match op {
                Function(mod_id, fun_id, type_params) => {
                    check_fun_inst(target, *mod_id, *fun_id, type_params, &loc);
                }
                Pack(mod_id, struct_id, type_params) => {
                    check_struct_inst(target, *mod_id, *struct_id, type_params, &loc);
                }
                Unpack(mod_id, struct_id, type_params) => {
                    check_struct_inst(target, *mod_id, *struct_id, type_params, &loc);
                }
                MoveTo(mod_id, struct_id, type_params) => {
                    check_struct_inst(target, *mod_id, *struct_id, type_params, &loc);
                    let ty = Type::Struct(*mod_id, *struct_id, type_params.clone());
                    check_key(target, &ty, &loc, "cannot be used as key");
                }
                MoveFrom(mod_id, struct_id, type_params) => {
                    check_struct_inst(target, *mod_id, *struct_id, type_params, &loc);
                    let ty = Type::Struct(*mod_id, *struct_id, type_params.clone());
                    check_key(target, &ty, &loc, "cannot be used as key")
                }
                Exists(mod_id, struct_id, type_params) => {
                    check_struct_inst(target, *mod_id, *struct_id, type_params, &loc);
                    let ty = Type::Struct(*mod_id, *struct_id, type_params.clone());
                    check_key(target, &ty, &loc, "cannot be used as key")
                }
                BorrowField(mod_id, struct_id, type_params, _) => {
                    check_struct_inst(target, *mod_id, *struct_id, type_params, &loc);
                }
                BorrowGlobal(mod_id, struct_id, type_params) => {
                    check_struct_inst(target, *mod_id, *struct_id, type_params, &loc);
                    let ty = Type::Struct(*mod_id, *struct_id, type_params.clone());
                    check_key(target, &ty, &loc, "cannot be used as key")
                },
                Destroy => {
                    check_drop_for_temp_with_msg(target, srcs[0], &loc, "cannot drop")
                }
                ReadRef => {
                    check_read_ref(target, srcs[0], &loc)
				}
                WriteRef => {
                    check_write_ref(target, srcs[0], &loc)
                },
                _ => (),
            }
        },
        _ => (),
    }
}
