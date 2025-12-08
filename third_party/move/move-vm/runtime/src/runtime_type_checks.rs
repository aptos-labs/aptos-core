// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    frame::Frame, frame_type_cache::FrameTypeCache, interpreter::Stack,
    reentrancy_checker::CallType, Function, LoadedFunction,
};
use move_binary_format::errors::*;
use move_core_types::{
    ability::{Ability, AbilitySet},
    function::ClosureMask,
    vm_status::{sub_status::unknown_invariant_violation::EPARANOID_FAILURE, StatusCode},
};
use move_vm_types::{
    instr::Instruction,
    ty_interner::{InternedTypePool, TypeId},
};

pub(crate) trait RuntimeTypeCheck {
    /// Paranoid type checks to perform before instruction execution.
    fn pre_execution_type_stack_transition(
        frame: &Frame,
        operand_stack: &mut Stack,
        instruction: &Instruction,
        ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()>;

    /// Paranoid type checks to perform after instruction execution.
    fn post_execution_type_stack_transition(
        frame: &Frame,
        operand_stack: &mut Stack,
        instruction: &Instruction,
        ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()>;

    /// For any other checks are performed externally
    fn should_perform_checks(for_fun: &Function) -> bool;

    /// Performs a runtime check of the caller is allowed to call the callee for any type of call,
    /// including native dynamic dispatch or calling a closure.
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn check_call_visibility(
        caller: &LoadedFunction,
        callee: &LoadedFunction,
        call_type: CallType,
    ) -> PartialVMResult<()> {
        match call_type {
            CallType::Regular => {
                // We only need to check cross-contract calls.
                if caller.module_id() == callee.module_id() {
                    return Ok(());
                }
                Self::check_cross_module_regular_call_visibility(caller, callee)
            },
            CallType::ClosureDynamicDispatch => {
                // In difference to regular calls, we skip visibility check. It is possible to call
                // a private function of another module via a closure.
                Ok(())
            },
            CallType::NativeDynamicDispatch => {
                // Dynamic dispatch may fail at runtime and this is ok. Hence, these errors are not
                // invariant violations as they cannot be checked at compile- or load-time.
                //
                // Note: native dispatch cannot call into the same module, otherwise the reentrancy
                // check is broken. For more details, see AIP-73:
                //   https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-73.md
                if callee.is_friend_or_private() || callee.module_id() == caller.module_id() {
                    return Err(PartialVMError::new(StatusCode::RUNTIME_DISPATCH_ERROR)
                        .with_message(
                            "Invoking private or friend function during dispatch".to_string(),
                        ));
                }

                if callee.is_native() {
                    return Err(PartialVMError::new(StatusCode::RUNTIME_DISPATCH_ERROR)
                        .with_message("Invoking native function during dispatch".to_string()));
                }
                Ok(())
            },
        }
    }

    /// Checks if the caller can pack a function as a closure.
    fn check_pack_closure_visibility(
        caller: &LoadedFunction,
        function: &LoadedFunction,
    ) -> PartialVMResult<()> {
        if caller.module_id() == function.module_id() {
            return Ok(());
        }
        // Same visibility rules as for regular cross-contract calls should apply.
        Self::check_cross_module_regular_call_visibility(caller, function)
    }

    /// Performs a runtime check of the caller is allowed to call a cross-module callee. Applies
    /// only on regular static calls (no dynamic dispatch!), with caller and callee being coming
    /// from different modules.
    fn check_cross_module_regular_call_visibility(
        caller: &LoadedFunction,
        callee: &LoadedFunction,
    ) -> PartialVMResult<()>;
}

// note(inline): improves perf a little bit, but increases `post_execution_type_stack_transition` by 20%
#[cfg_attr(feature = "force-inline", inline(always))]
fn verify_pack(
    ty_pool: &InternedTypePool,
    operand_stack: &mut Stack,
    field_count: u16,
    field_tys: impl Iterator<Item = TypeId>,
    output_ty: TypeId,
) -> PartialVMResult<()> {
    let ability = ty_pool.abilities(output_ty);

    // If the struct has a key ability, we expect all of its field to
    // have store ability but not key ability.
    let field_expected_abilities = if ability.has_key() {
        ability
            .remove(Ability::Key)
            .union(AbilitySet::singleton(Ability::Store))
    } else {
        ability
    };
    for (ty, expected_ty) in operand_stack
        .popn_tys(field_count)?
        .into_iter()
        .zip(field_tys)
    {
        // Fields ability should be a subset of the struct ability
        // because abilities can be weakened but not the other
        // direction.
        // For example, it is ok to have a struct that doesn't have a
        // copy capability where its field is a struct that has copy
        // capability but not vice versa.
        ty_pool.paranoid_check_abilities(ty, field_expected_abilities)?;
        // Similar, we use assignability for the value moved in the field
        ty_pool.paranoid_check_assignable(ty, expected_ty)?;
    }

    operand_stack.push_ty(output_ty)
}

pub fn verify_pack_closure(
    ty_pool: &InternedTypePool,
    operand_stack: &mut Stack,
    func: &LoadedFunction,
    mask: ClosureMask,
) -> PartialVMResult<()> {
    // Accumulated abilities
    let mut abilities = if func.function.is_persistent() {
        AbilitySet::PUBLIC_FUNCTIONS
    } else {
        AbilitySet::PRIVATE_FUNCTIONS
    };
    // Verify that captured arguments are assignable against types in the function
    // signature, and that they are no references.
    let expected_capture_tys = mask.extract(func.param_tys(), true);

    let given_capture_tys = operand_stack.popn_tys(expected_capture_tys.len() as u16)?;
    for (expected_ty_type, given) in expected_capture_tys
        .into_iter()
        .zip(given_capture_tys.into_iter())
    {
        let expected = ty_pool.instantiate_and_intern(expected_ty_type, func.ty_args());
        ty_pool.paranoid_check_is_no_ref(expected, "Captured argument type")?;

        // Intersect the captured type with the accumulated abilities
        abilities = abilities.intersect(ty_pool.abilities(given));
        ty_pool.paranoid_check_assignable(given, expected)?;
    }

    let func_ty = if func.ty_args.is_empty() {
        let args = mask
            .extract(func.param_ty_ids(), false)
            .into_iter()
            .copied()
            .collect::<Vec<_>>();
        ty_pool.function_of_vec(args, func.return_ty_ids().to_vec(), abilities)
    } else {
        let args = mask
            .extract(func.param_tys(), false)
            .into_iter()
            .map(|curried| Ok(ty_pool.instantiate_and_intern(curried, &func.ty_args)))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let results = func
            .return_tys()
            .iter()
            .map(|ret| Ok(ty_pool.instantiate_and_intern(ret, &func.ty_args)))
            .collect::<PartialVMResult<Vec<_>>>()?;
        ty_pool.function_of_vec(args, results, abilities)
    };

    operand_stack.push_ty(func_ty)?;

    Ok(())
}

pub(crate) struct NoRuntimeTypeCheck;
pub(crate) struct FullRuntimeTypeCheck;
pub(crate) struct UntrustedOnlyRuntimeTypeCheck;

impl RuntimeTypeCheck for NoRuntimeTypeCheck {
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn pre_execution_type_stack_transition(
        _frame: &Frame,
        _operand_stack: &mut Stack,
        _instruction: &Instruction,
        _ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn post_execution_type_stack_transition(
        _frame: &Frame,
        _operand_stack: &mut Stack,
        _instruction: &Instruction,
        _ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn should_perform_checks(_for_fun: &Function) -> bool {
        false
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn check_cross_module_regular_call_visibility(
        _caller: &LoadedFunction,
        _callee: &LoadedFunction,
    ) -> PartialVMResult<()> {
        Ok(())
    }
}

impl RuntimeTypeCheck for FullRuntimeTypeCheck {
    /// Note that most of the checks should happen after instruction execution, because gas charging will happen during
    /// instruction execution and we want to avoid running code without charging proper gas as much as possible.
    // note(inline): it should not be inlined, function calling overhead
    // is not big enough to justify the increase in function size
    fn pre_execution_type_stack_transition(
        frame: &Frame,
        operand_stack: &mut Stack,
        instruction: &Instruction,
        ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()> {
        match instruction {
            // Call instruction will be checked at execute_main.
            Instruction::Call(_) | Instruction::CallGeneric(_) => (),
            Instruction::BrFalse(_) | Instruction::BrTrue(_) => {
                operand_stack.pop_ty()?;
            },
            Instruction::CallClosure(sig_idx) => {
                // For closure, we need to check the type of the closure on
                // top of the stack. The argument types are checked when the frame
                // is constructed in the interpreter, using the same code as for regular
                // calls.
                let ty_pool = frame.ty_pool();
                let (expected_ty, _) = ty_cache.get_signature_index_type(*sig_idx, frame)?;
                let given_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_assignable(given_ty, expected_ty)?;
            },
            Instruction::Branch(_) => (),
            Instruction::Ret => {
                frame.check_local_tys_have_drop_ability()?;
            },
            Instruction::Abort => {
                operand_stack.pop_ty()?;
            },
            // StLoc needs to check before execution as we need to check the drop ability of values.
            Instruction::StLoc(idx) => {
                let ty_pool = frame.ty_pool();
                let expected_ty = frame.local_ty_at(*idx as usize);
                let val_ty = operand_stack.pop_ty()?;
                // For store, use assignability
                ty_pool.paranoid_check_assignable(val_ty, expected_ty)?;
                if !frame.locals.is_invalid(*idx as usize)? {
                    ty_pool.paranoid_check_has_ability(expected_ty, Ability::Drop)?;
                }
            },
            // We will check the rest of the instructions after execution phase.
            Instruction::Pop
            | Instruction::LdU8(_)
            | Instruction::LdU16(_)
            | Instruction::LdU32(_)
            | Instruction::LdU64(_)
            | Instruction::LdU128(_)
            | Instruction::LdU256(_)
            | Instruction::LdI8(_)
            | Instruction::LdI16(_)
            | Instruction::LdI32(_)
            | Instruction::LdI64(_)
            | Instruction::LdI128(_)
            | Instruction::LdI256(_)
            | Instruction::LdTrue
            | Instruction::LdFalse
            | Instruction::LdConst(_)
            | Instruction::CopyLoc(_)
            | Instruction::MoveLoc(_)
            | Instruction::MutBorrowLoc(_)
            | Instruction::ImmBorrowLoc(_)
            | Instruction::ImmBorrowField(_)
            | Instruction::MutBorrowField(_)
            | Instruction::ImmBorrowFieldGeneric(_)
            | Instruction::MutBorrowFieldGeneric(_)
            | Instruction::PackClosure(..)
            | Instruction::PackClosureGeneric(..)
            | Instruction::Pack(_)
            | Instruction::PackGeneric(_)
            | Instruction::Unpack(_)
            | Instruction::UnpackGeneric(_)
            | Instruction::ReadRef
            | Instruction::WriteRef
            | Instruction::CastU8
            | Instruction::CastU16
            | Instruction::CastU32
            | Instruction::CastU64
            | Instruction::CastU128
            | Instruction::CastU256
            | Instruction::CastI8
            | Instruction::CastI16
            | Instruction::CastI32
            | Instruction::CastI64
            | Instruction::CastI128
            | Instruction::CastI256
            | Instruction::Add
            | Instruction::Sub
            | Instruction::Mul
            | Instruction::Mod
            | Instruction::Div
            | Instruction::Negate
            | Instruction::BitOr
            | Instruction::BitAnd
            | Instruction::Xor
            | Instruction::Or
            | Instruction::And
            | Instruction::Shl
            | Instruction::Shr
            | Instruction::Lt
            | Instruction::Le
            | Instruction::Gt
            | Instruction::Ge
            | Instruction::Eq
            | Instruction::Neq
            | Instruction::MutBorrowGlobal(_)
            | Instruction::ImmBorrowGlobal(_)
            | Instruction::MutBorrowGlobalGeneric(_)
            | Instruction::ImmBorrowGlobalGeneric(_)
            | Instruction::Exists(_)
            | Instruction::ExistsGeneric(_)
            | Instruction::MoveTo(_)
            | Instruction::MoveToGeneric(_)
            | Instruction::MoveFrom(_)
            | Instruction::MoveFromGeneric(_)
            | Instruction::FreezeRef
            | Instruction::Nop
            | Instruction::Not
            | Instruction::VecPack(_, _)
            | Instruction::VecLen(_)
            | Instruction::VecImmBorrow(_)
            | Instruction::VecMutBorrow(_)
            | Instruction::VecPushBack(_)
            | Instruction::VecPopBack(_)
            | Instruction::VecUnpack(_, _)
            | Instruction::VecSwap(_) => (),

            // Since bytecode version 7
            Instruction::PackVariant(_)
            | Instruction::PackVariantGeneric(_)
            | Instruction::UnpackVariant(_)
            | Instruction::UnpackVariantGeneric(_)
            | Instruction::TestVariant(_)
            | Instruction::TestVariantGeneric(_)
            | Instruction::MutBorrowVariantField(_)
            | Instruction::MutBorrowVariantFieldGeneric(_)
            | Instruction::ImmBorrowVariantField(_)
            | Instruction::ImmBorrowVariantFieldGeneric(_) => (),
        };
        Ok(())
    }

    /// Paranoid type checks to perform after instruction execution.
    ///
    /// This function and `pre_execution_type_stack_transition` should
    /// constitute the full type stack transition for the paranoid
    /// mode.
    // note(inline): it should not be inlined, function calling overhead
    // is not big enough to justify the increase in function size
    fn post_execution_type_stack_transition(
        frame: &Frame,
        operand_stack: &mut Stack,
        instruction: &Instruction,
        ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()> {
        let ty_pool = frame.ty_pool();
        match instruction {
            Instruction::BrTrue(_) | Instruction::BrFalse(_) => (),
            Instruction::Branch(_)
            | Instruction::Ret
            | Instruction::Call(_)
            | Instruction::CallGeneric(_)
            | Instruction::CallClosure(_)
            | Instruction::Abort => {
                // Invariants hold because all of the instructions
                // above will force VM to break from the interpreter
                // loop and thus not hit this code path.
                unreachable!("control flow instruction encountered during type check")
            },
            Instruction::Pop => {
                let ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_has_ability(ty, Ability::Drop)?;
            },
            Instruction::LdU8(_) => operand_stack.push_ty(TypeId::U8)?,
            Instruction::LdU16(_) => operand_stack.push_ty(TypeId::U16)?,
            Instruction::LdU32(_) => operand_stack.push_ty(TypeId::U32)?,
            Instruction::LdU64(_) => operand_stack.push_ty(TypeId::U64)?,
            Instruction::LdU128(_) => operand_stack.push_ty(TypeId::U128)?,
            Instruction::LdU256(_) => operand_stack.push_ty(TypeId::U256)?,
            Instruction::LdI8(_) => operand_stack.push_ty(TypeId::I8)?,
            Instruction::LdI16(_) => operand_stack.push_ty(TypeId::I16)?,
            Instruction::LdI32(_) => operand_stack.push_ty(TypeId::I32)?,
            Instruction::LdI64(_) => operand_stack.push_ty(TypeId::I64)?,
            Instruction::LdI128(_) => operand_stack.push_ty(TypeId::I128)?,
            Instruction::LdI256(_) => operand_stack.push_ty(TypeId::I256)?,
            Instruction::LdTrue | Instruction::LdFalse => operand_stack.push_ty(TypeId::BOOL)?,
            Instruction::LdConst(i) => {
                let constant = frame.constant_at(*i);
                // TODO: cache at load-time.
                let ty = ty_pool.create_constant_ty(&constant.type_);
                operand_stack.push_ty(ty)?;
            },
            Instruction::CopyLoc(idx) => {
                let ty = frame.local_ty_at(*idx as usize);
                ty_pool.paranoid_check_has_ability(ty, Ability::Copy)?;
                operand_stack.push_ty(ty)?;
            },
            Instruction::MoveLoc(idx) => {
                let ty = frame.local_ty_at(*idx as usize);
                operand_stack.push_ty(ty)?;
            },
            Instruction::StLoc(_) => (),
            Instruction::MutBorrowLoc(idx) => {
                let ty = frame.local_ty_at(*idx as usize);
                operand_stack.push_ty(TypeId::ref_mut_of(ty))?;
            },
            Instruction::ImmBorrowLoc(idx) => {
                let ty = frame.local_ty_at(*idx as usize);
                operand_stack.push_ty(TypeId::ref_of(ty))?;
            },
            Instruction::ImmBorrowField(fh_idx) => {
                let ty = operand_stack.pop_ty()?;
                let expected_ty_type = frame.field_handle_to_struct(*fh_idx);
                let expected_ty = ty_pool.instantiate_and_intern(&expected_ty_type, &[]);
                ty_pool.paranoid_check_ref_eq(ty, expected_ty, false)?;

                let field_ty = ty_pool.instantiate_and_intern(frame.get_field_ty(*fh_idx)?, &[]);
                operand_stack.push_ty(TypeId::ref_of(field_ty))?;
            },
            Instruction::MutBorrowField(fh_idx) => {
                let ref_ty = operand_stack.pop_ty()?;
                let expected_inner_ty_type = frame.field_handle_to_struct(*fh_idx);
                let expected_inner_ty =
                    ty_pool.instantiate_and_intern(&expected_inner_ty_type, &[]);
                ty_pool.paranoid_check_ref_eq(ref_ty, expected_inner_ty, true)?;

                let field_ty = ty_pool.instantiate_and_intern(frame.get_field_ty(*fh_idx)?, &[]);
                operand_stack.push_ty(TypeId::ref_mut_of(field_ty))?;
            },
            Instruction::ImmBorrowFieldGeneric(idx) => {
                let struct_ty = operand_stack.pop_ty()?;
                let ((field_ty, _), (expected_struct_ty, _)) =
                    ty_cache.get_field_type_and_struct_type(*idx, frame)?;
                ty_pool.paranoid_check_ref_eq(struct_ty, expected_struct_ty, false)?;
                operand_stack.push_ty(TypeId::ref_of(field_ty))?;
            },
            Instruction::MutBorrowFieldGeneric(idx) => {
                let struct_ty = operand_stack.pop_ty()?;
                let ((field_ty, _), (expected_struct_ty, _)) =
                    ty_cache.get_field_type_and_struct_type(*idx, frame)?;
                ty_pool.paranoid_check_ref_eq(struct_ty, expected_struct_ty, true)?;
                operand_stack.push_ty(TypeId::ref_mut_of(field_ty))?;
            },
            Instruction::ImmBorrowVariantField(fh_idx)
            | Instruction::MutBorrowVariantField(fh_idx) => {
                let is_mut = matches!(instruction, Instruction::MutBorrowVariantField(..));
                let field_info = frame.variant_field_info_at(*fh_idx);
                let ty = operand_stack.pop_ty()?;
                let expected_ty_type = frame.create_struct_ty(&field_info.definition_struct_type);
                let expected_ty = ty_pool.instantiate_and_intern(&expected_ty_type, &[]);
                ty_pool.paranoid_check_ref_eq(ty, expected_ty, is_mut)?;
                let field_ty_type = &field_info.uninstantiated_field_ty;
                let field_ty = ty_pool.instantiate_and_intern(field_ty_type, &[]);
                let field_ref_ty = if is_mut {
                    TypeId::ref_mut_of(field_ty)
                } else {
                    TypeId::ref_of(field_ty)
                };
                operand_stack.push_ty(field_ref_ty)?;
            },
            Instruction::ImmBorrowVariantFieldGeneric(idx)
            | Instruction::MutBorrowVariantFieldGeneric(idx) => {
                let is_mut = matches!(instruction, Instruction::MutBorrowVariantFieldGeneric(..));
                let struct_ty = operand_stack.pop_ty()?;
                let ((field_ty, _), (expected_struct_ty, _)) =
                    ty_cache.get_variant_field_type_and_struct_type(*idx, frame)?;
                ty_pool.paranoid_check_ref_eq(struct_ty, expected_struct_ty, is_mut)?;
                let field_ref_ty = if is_mut {
                    TypeId::ref_mut_of(field_ty)
                } else {
                    TypeId::ref_of(field_ty)
                };
                operand_stack.push_ty(field_ref_ty)?;
            },
            Instruction::PackClosure(..) | Instruction::PackClosureGeneric(..) => {
                // Skip: runtime checks are implemented in interpreter loop!
            },

            Instruction::Pack(idx) => {
                let field_count = frame.field_count(*idx);
                let args_ty = frame.get_struct(*idx);
                let field_tys = args_ty
                    .fields(None)?
                    .iter()
                    .map(|(_, ty)| ty_pool.instantiate_and_intern(ty, &[]));
                let output_ty = frame.get_struct_ty(*idx);
                verify_pack(ty_pool, operand_stack, field_count, field_tys, output_ty)?;
            },
            Instruction::PackGeneric(idx) => {
                let field_count = frame.field_instantiation_count(*idx);
                let output_ty = ty_cache.get_struct_type(*idx, frame)?.0;
                let args_ty = ty_cache.get_struct_fields_types(*idx, frame)?;

                if field_count as usize != args_ty.len() {
                    // This is an inconsistency between the cache and the actual
                    // type declaration. We would crash if for some reason this invariant does
                    // not hold. It seems impossible to hit, but we keep it here for safety
                    // reasons, as a previous version of this code had this too.
                    return Err(
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .with_message("Args count mismatch".to_string()),
                    );
                }

                verify_pack(
                    ty_pool,
                    operand_stack,
                    field_count,
                    args_ty.iter().map(|(ty, _)| *ty),
                    output_ty,
                )?;
            },
            Instruction::Unpack(idx) => {
                let struct_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_eq(struct_ty, frame.get_struct_ty(*idx))?;
                let struct_decl = frame.get_struct(*idx);
                for (_name, ty) in struct_decl.fields(None)?.iter() {
                    let ty_id = ty_pool.instantiate_and_intern(ty, &[]);
                    operand_stack.push_ty(ty_id)?;
                }
            },
            Instruction::UnpackGeneric(idx) => {
                let struct_ty = operand_stack.pop_ty()?;

                ty_pool.paranoid_check_eq(struct_ty, ty_cache.get_struct_type(*idx, frame)?.0)?;

                let struct_fields_types = ty_cache.get_struct_fields_types(*idx, frame)?;
                for (ty, _) in struct_fields_types {
                    operand_stack.push_ty(*ty)?;
                }
            },
            Instruction::PackVariant(idx) => {
                let info = frame.get_struct_variant_at(*idx);
                let field_tys = info
                    .definition_struct_type
                    .fields(Some(info.variant))?
                    .iter()
                    .map(|(_, ty)| ty_pool.instantiate_and_intern(ty, &[]));
                let output_ty_type = frame.create_struct_ty(&info.definition_struct_type);
                let output_ty = ty_pool.instantiate_and_intern(&output_ty_type, &[]);
                verify_pack(
                    ty_pool,
                    operand_stack,
                    info.field_count,
                    field_tys,
                    output_ty,
                )?;
            },
            Instruction::PackVariantGeneric(idx) => {
                let info = frame.get_struct_variant_instantiation_at(*idx);
                let output_ty = ty_cache.get_struct_variant_type(*idx, frame)?.0;
                let args_ty = ty_cache.get_struct_variant_fields_types(*idx, frame)?;
                verify_pack(
                    ty_pool,
                    operand_stack,
                    info.field_count,
                    args_ty.iter().map(|(ty, _)| *ty),
                    output_ty,
                )?;
            },
            Instruction::UnpackVariant(idx) => {
                let info = frame.get_struct_variant_at(*idx);
                let expected_struct_ty_type = frame.create_struct_ty(&info.definition_struct_type);
                let expected_struct_ty =
                    ty_pool.instantiate_and_intern(&expected_struct_ty_type, &[]);
                let actual_struct_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_eq(actual_struct_ty, expected_struct_ty)?;
                for (_name, ty) in info
                    .definition_struct_type
                    .fields(Some(info.variant))?
                    .iter()
                {
                    let ty_id = ty_pool.instantiate_and_intern(ty, &[]);
                    operand_stack.push_ty(ty_id)?;
                }
            },
            Instruction::UnpackVariantGeneric(idx) => {
                let expected_struct_type = ty_cache.get_struct_variant_type(*idx, frame)?.0;
                let actual_struct_type = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_eq(actual_struct_type, expected_struct_type)?;
                let struct_fields_types = ty_cache.get_struct_variant_fields_types(*idx, frame)?;
                for (ty, _) in struct_fields_types {
                    operand_stack.push_ty(*ty)?;
                }
            },
            Instruction::TestVariant(idx) => {
                let info = frame.get_struct_variant_at(*idx);
                let expected_struct_ty_type = frame.create_struct_ty(&info.definition_struct_type);
                let expected_struct_ty =
                    ty_pool.instantiate_and_intern(&expected_struct_ty_type, &[]);
                let actual_struct_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_ref_eq(actual_struct_ty, expected_struct_ty, false)?;
                operand_stack.push_ty(TypeId::BOOL)?;
            },
            Instruction::TestVariantGeneric(idx) => {
                let expected_struct_ty = ty_cache.get_struct_variant_type(*idx, frame)?.0;
                let actual_struct_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_ref_eq(actual_struct_ty, expected_struct_ty, false)?;
                operand_stack.push_ty(TypeId::BOOL)?;
            },
            Instruction::ReadRef => {
                let ref_ty = operand_stack.pop_ty()?;
                let inner_ty = ty_pool.paranoid_read_ref(ref_ty)?;
                operand_stack.push_ty(inner_ty)?;
            },
            Instruction::WriteRef => {
                let mut_ref_ty = operand_stack.pop_ty()?;
                let val_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_write_ref(mut_ref_ty, val_ty)?;
            },
            Instruction::CastU8 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::U8)?;
            },
            Instruction::CastU16 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::U16)?;
            },
            Instruction::CastU32 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::U32)?;
            },
            Instruction::CastU64 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::U64)?;
            },
            Instruction::CastU128 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::U128)?;
            },
            Instruction::CastU256 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::U256)?;
            },
            Instruction::CastI8 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::I8)?;
            },
            Instruction::CastI16 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::I16)?;
            },
            Instruction::CastI32 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::I32)?;
            },
            Instruction::CastI64 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::I64)?;
            },
            Instruction::CastI128 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::I128)?;
            },
            Instruction::CastI256 => {
                operand_stack.pop_ty()?;
                operand_stack.push_ty(TypeId::I256)?;
            },
            Instruction::Add
            | Instruction::Sub
            | Instruction::Mul
            | Instruction::Mod
            | Instruction::Div
            | Instruction::BitOr
            | Instruction::BitAnd
            | Instruction::Xor
            | Instruction::Or
            | Instruction::And => {
                let rhs_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_eq(rhs_ty, operand_stack.top_ty()?)?;
                // NO-OP, same as the two lines below when the types are indeed the same:
                // let lhs_ty = operand_stack.pop_ty()?;
                // operand_stack.push_ty(rhs_ty)?;
            },
            Instruction::Negate => {
                ty_pool.paranoid_check_is_sint_ty(operand_stack.top_ty()?)?;
                // NO-OP, leave stack as is
            },
            Instruction::Shl | Instruction::Shr => {
                let _rhs = operand_stack.pop_ty()?;
                // NO-OP, same as the two lines below:
                // let lhs = operand_stack.pop_ty()?;
                // operand_stack.push_ty(lhs)?;
            },
            Instruction::Lt | Instruction::Le | Instruction::Gt | Instruction::Ge => {
                let rhs_ty = operand_stack.pop_ty()?;
                let lhs_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_eq(rhs_ty, lhs_ty)?;
                operand_stack.push_ty(TypeId::BOOL)?;
            },
            Instruction::Eq | Instruction::Neq => {
                let rhs_ty = operand_stack.pop_ty()?;
                let lhs_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_eq(rhs_ty, lhs_ty)?;
                ty_pool.paranoid_check_has_ability(rhs_ty, Ability::Drop)?;
                operand_stack.push_ty(TypeId::BOOL)?;
            },
            Instruction::MutBorrowGlobal(idx) => {
                ty_pool.paranoid_check_is_address_ty(operand_stack.pop_ty()?)?;
                let struct_ty = frame.get_struct_ty(*idx);
                ty_pool.paranoid_check_has_ability(struct_ty, Ability::Key)?;

                let struct_mut_ref_ty = TypeId::ref_mut_of(struct_ty);
                operand_stack.push_ty(struct_mut_ref_ty)?;
            },
            Instruction::ImmBorrowGlobal(idx) => {
                ty_pool.paranoid_check_is_address_ty(operand_stack.pop_ty()?)?;
                let struct_ty = frame.get_struct_ty(*idx);
                ty_pool.paranoid_check_has_ability(struct_ty, Ability::Key)?;

                let struct_ref_ty = TypeId::ref_of(struct_ty);
                operand_stack.push_ty(struct_ref_ty)?;
            },
            Instruction::MutBorrowGlobalGeneric(idx) => {
                ty_pool.paranoid_check_is_address_ty(operand_stack.pop_ty()?)?;
                let struct_ty = ty_cache.get_struct_type(*idx, frame)?.0;
                ty_pool.paranoid_check_has_ability(struct_ty, Ability::Key)?;

                let struct_mut_ref_ty = TypeId::ref_mut_of(struct_ty);
                operand_stack.push_ty(struct_mut_ref_ty)?;
            },
            Instruction::ImmBorrowGlobalGeneric(idx) => {
                ty_pool.paranoid_check_is_address_ty(operand_stack.pop_ty()?)?;
                let struct_ty = ty_cache.get_struct_type(*idx, frame)?.0;
                ty_pool.paranoid_check_has_ability(struct_ty, Ability::Key)?;

                let struct_ref_ty = TypeId::ref_of(struct_ty);
                operand_stack.push_ty(struct_ref_ty)?;
            },
            Instruction::Exists(_) | Instruction::ExistsGeneric(_) => {
                ty_pool.paranoid_check_is_address_ty(operand_stack.pop_ty()?)?;
                operand_stack.push_ty(TypeId::BOOL)?;
            },
            Instruction::MoveTo(idx) => {
                let ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_is_signer_ref_ty(operand_stack.pop_ty()?)?;
                ty_pool.paranoid_check_eq(ty, frame.get_struct_ty(*idx))?;
                ty_pool.paranoid_check_has_ability(ty, Ability::Key)?;
            },
            Instruction::MoveToGeneric(idx) => {
                let ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_is_signer_ref_ty(operand_stack.pop_ty()?)?;
                ty_pool.paranoid_check_eq(ty, ty_cache.get_struct_type(*idx, frame)?.0)?;
                ty_pool.paranoid_check_has_ability(ty, Ability::Key)?;
            },
            Instruction::MoveFrom(idx) => {
                ty_pool.paranoid_check_is_address_ty(operand_stack.pop_ty()?)?;
                let ty = frame.get_struct_ty(*idx);
                ty_pool.paranoid_check_has_ability(ty, Ability::Key)?;
                operand_stack.push_ty(ty)?;
            },
            Instruction::MoveFromGeneric(idx) => {
                ty_pool.paranoid_check_is_address_ty(operand_stack.pop_ty()?)?;
                let ty = ty_cache.get_struct_type(*idx, frame)?.0;
                ty_pool.paranoid_check_has_ability(ty, Ability::Key)?;
                operand_stack.push_ty(ty)?;
            },
            Instruction::FreezeRef => {
                let mut_ref_ty = operand_stack.pop_ty()?;
                let ref_ty = ty_pool.paranoid_freeze_ref_ty(mut_ref_ty)?;
                operand_stack.push_ty(ref_ty)?;
            },
            Instruction::Nop => (),
            Instruction::Not => {
                ty_pool.paranoid_check_is_bool_ty(operand_stack.top_ty()?)?;
                // NO-OP,  same as the two lines below:
                // let bool_ty = ty_pool.bool_ty();
                // operand_stack.push_ty(bool_ty)?;
            },
            Instruction::VecPack(si, num) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, frame)?;
                let elem_tys = operand_stack.popn_tys(*num as u16)?;
                for elem_ty in elem_tys.iter() {
                    // For vector element types, use assignability
                    ty_pool.paranoid_check_assignable(*elem_ty, ty)?;
                }

                let vec_ty = ty_pool.vec_of(ty);
                operand_stack.push_ty(vec_ty)?;
            },
            Instruction::VecLen(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, frame)?;
                ty_pool.paranoid_check_is_vec_ref_ty::<false>(operand_stack.pop_ty()?, ty)?;
                operand_stack.push_ty(TypeId::U64)?;
            },
            Instruction::VecImmBorrow(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, frame)?;
                ty_pool.paranoid_check_is_u64_ty(operand_stack.pop_ty()?)?;
                let elem_ref_ty = ty_pool
                    .paranoid_check_and_get_vec_elem_ref_ty::<false>(operand_stack.pop_ty()?, ty)?;

                operand_stack.push_ty(elem_ref_ty)?;
            },
            Instruction::VecMutBorrow(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, frame)?;
                ty_pool.paranoid_check_is_u64_ty(operand_stack.pop_ty()?)?;
                let elem_ref_ty = ty_pool
                    .paranoid_check_and_get_vec_elem_ref_ty::<true>(operand_stack.pop_ty()?, ty)?;
                operand_stack.push_ty(elem_ref_ty)?;
            },
            Instruction::VecPushBack(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, frame)?;
                // For pushing an element to a vector, use assignability
                ty_pool.paranoid_check_assignable(operand_stack.pop_ty()?, ty)?;
                ty_pool.paranoid_check_is_vec_ref_ty::<true>(operand_stack.pop_ty()?, ty)?;
            },
            Instruction::VecPopBack(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, frame)?;
                let elem_ty = ty_pool
                    .paranoid_check_and_get_vec_elem_ty::<true>(operand_stack.pop_ty()?, ty)?;
                operand_stack.push_ty(elem_ty)?;
            },
            Instruction::VecUnpack(si, num) => {
                let (expected_elem_ty, _) = ty_cache.get_signature_index_type(*si, frame)?;
                let vec_ty = operand_stack.pop_ty()?;
                ty_pool.paranoid_check_is_vec_ty(vec_ty, expected_elem_ty)?;
                for _ in 0..*num {
                    operand_stack.push_ty(expected_elem_ty)?;
                }
            },
            Instruction::VecSwap(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, frame)?;
                ty_pool.paranoid_check_is_u64_ty(operand_stack.pop_ty()?)?;
                ty_pool.paranoid_check_is_u64_ty(operand_stack.pop_ty()?)?;
                ty_pool.paranoid_check_is_vec_ref_ty::<true>(operand_stack.pop_ty()?, ty)?;
            },
        }
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn should_perform_checks(_for_fun: &Function) -> bool {
        true
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn check_cross_module_regular_call_visibility(
        caller: &LoadedFunction,
        callee: &LoadedFunction,
    ) -> PartialVMResult<()> {
        if callee.is_private() {
            let msg = format!(
                "Function {}::{} cannot be called because it is private",
                callee.module_or_script_id(),
                callee.name()
            );
            return Err(
                PartialVMError::new_invariant_violation(msg).with_sub_status(EPARANOID_FAILURE)
            );
        }

        if callee.is_friend() {
            let callee_module = callee.owner_as_module().map_err(|err| err.to_partial())?;
            if !caller
                .module_id()
                .is_some_and(|id| callee_module.friends.contains(id))
            {
                let msg = format!(
                    "Function {}::{} cannot be called because it has friend visibility, but {} \
                     is not {}'s friend",
                    callee.module_or_script_id(),
                    callee.name(),
                    caller.module_or_script_id(),
                    callee.module_or_script_id()
                );
                return Err(
                    PartialVMError::new_invariant_violation(msg).with_sub_status(EPARANOID_FAILURE)
                );
            }
        }

        Ok(())
    }
}

impl RuntimeTypeCheck for UntrustedOnlyRuntimeTypeCheck {
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn pre_execution_type_stack_transition(
        frame: &Frame,
        operand_stack: &mut Stack,
        instruction: &Instruction,
        ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()> {
        if frame.untrusted_code() {
            FullRuntimeTypeCheck::pre_execution_type_stack_transition(
                frame,
                operand_stack,
                instruction,
                ty_cache,
            )
        } else {
            Ok(())
        }
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn post_execution_type_stack_transition(
        frame: &Frame,
        operand_stack: &mut Stack,
        instruction: &Instruction,
        ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()> {
        if frame.untrusted_code() {
            FullRuntimeTypeCheck::post_execution_type_stack_transition(
                frame,
                operand_stack,
                instruction,
                ty_cache,
            )
        } else {
            Ok(())
        }
    }

    fn should_perform_checks(for_fun: &Function) -> bool {
        !for_fun.is_trusted
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn check_cross_module_regular_call_visibility(
        caller: &LoadedFunction,
        callee: &LoadedFunction,
    ) -> PartialVMResult<()> {
        if !caller.function.is_trusted {
            FullRuntimeTypeCheck::check_cross_module_regular_call_visibility(caller, callee)
        } else {
            Ok(())
        }
    }
}
