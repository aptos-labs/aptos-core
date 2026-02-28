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
    loaded_data::runtime_types::{Type, TypeBuilder},
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
fn verify_pack<'a>(
    operand_stack: &mut Stack,
    field_count: u16,
    field_tys: impl Iterator<Item = &'a Type>,
    output_ty: Type,
) -> PartialVMResult<()> {
    let ability = output_ty.abilities()?;

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
        ty.paranoid_check_abilities(field_expected_abilities)?;
        // Similar, we use assignability for the value moved in the field
        ty.paranoid_check_assignable(expected_ty)?;
    }

    operand_stack.push_ty(output_ty)
}

pub(crate) fn check_function_type_count_and_depth(
    ty_builder: &TypeBuilder,
    func: &LoadedFunction,
    mask: ClosureMask,
) -> PartialVMResult<()> {
    // Creates function type which implicitly performs count and depth checks via the TypeBuilder,
    // and then throws away the created type. The TypeBuilder enforces size and depth limits during
    // type construction, so any violations will be caught and returned as errors.
    // Abilities are not important for the depth / size checks.
    create_function_type(ty_builder, func, mask, AbilitySet::PUBLIC_FUNCTIONS)?;
    Ok(())
}

pub(crate) fn create_function_type(
    ty_builder: &TypeBuilder,
    func: &LoadedFunction,
    mask: ClosureMask,
    abilities: AbilitySet,
) -> PartialVMResult<Type> {
    let args = mask
        .extract(func.param_tys(), false)
        .into_iter()
        .map(|curried| with_owned_instantiation(ty_builder, func, curried, Ok))
        .collect::<PartialVMResult<Vec<_>>>()?;
    let results = func
        .return_tys()
        .iter()
        .map(|ret| with_owned_instantiation(ty_builder, func, ret, Ok))
        .collect::<PartialVMResult<Vec<_>>>()?;
    Ok(Type::Function {
        args,
        results,
        abilities,
    })
}

pub fn verify_pack_closure(
    ty_builder: &TypeBuilder,
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
    for (expected, given) in expected_capture_tys
        .into_iter()
        .zip(given_capture_tys.into_iter())
    {
        expected.paranoid_check_is_no_ref("Captured argument type")?;
        with_instantiation(ty_builder, func, expected, |expected| {
            // Intersect the captured type with the accumulated abilities
            abilities = abilities.intersect(given.abilities()?);
            given.paranoid_check_assignable(expected)
        })?
    }
    // Push result type onto stack
    operand_stack.push_ty(create_function_type(ty_builder, func, mask, abilities)?)?;

    Ok(())
}

fn with_instantiation<R>(
    ty_builder: &TypeBuilder,
    func: &LoadedFunction,
    ty: &Type,
    action: impl FnOnce(&Type) -> PartialVMResult<R>,
) -> PartialVMResult<R> {
    if func.ty_args().is_empty() {
        action(ty)
    } else {
        action(&ty_builder.create_ty_with_subst(ty, func.ty_args())?)
    }
}

fn with_owned_instantiation<R>(
    ty_builder: &TypeBuilder,
    func: &LoadedFunction,
    ty: &Type,
    action: impl FnOnce(Type) -> PartialVMResult<R>,
) -> PartialVMResult<R> {
    if func.ty_args().is_empty() {
        action(ty.clone())
    } else {
        action(ty_builder.create_ty_with_subst(ty, func.ty_args())?)
    }
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
                let (expected_ty, _, _) = ty_cache.get_signature_index_type(*sig_idx, frame)?;
                let given_ty = operand_stack.pop_ty()?;
                given_ty.paranoid_check_assignable(expected_ty)?;
            },
            Instruction::Branch(_) => (),
            Instruction::Ret => {
                frame.check_local_tys_have_drop_ability()?;
            },
            Instruction::Abort => {
                let ty = operand_stack.pop_ty()?;
                ty.paranoid_check_is_u64_ty()?;
            },
            Instruction::AbortMsg => {
                let ty1 = operand_stack.pop_ty()?;
                ty1.paranoid_check_is_vec_ty(&Type::U8)?;
                let ty2 = operand_stack.pop_ty()?;
                ty2.paranoid_check_is_u64_ty()?;
            },
            // StLoc needs to check before execution as we need to check the drop ability of values.
            Instruction::StLoc(idx) => {
                let expected_ty = frame.local_ty_at(*idx as usize);
                let val_ty = operand_stack.pop_ty()?;
                // For store, use assignability
                val_ty.paranoid_check_assignable(expected_ty)?;
                if !frame.locals.is_invalid(*idx as usize)? {
                    expected_ty.paranoid_check_has_ability(Ability::Drop)?;
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
        let ty_builder = frame.ty_builder();
        match instruction {
            Instruction::BrTrue(_) | Instruction::BrFalse(_) => (),
            Instruction::Branch(_)
            | Instruction::Ret
            | Instruction::Call(_)
            | Instruction::CallGeneric(_)
            | Instruction::CallClosure(_)
            | Instruction::Abort
            | Instruction::AbortMsg => {
                // Invariants hold because all of the instructions
                // above will force VM to break from the interpreter
                // loop and thus not hit this code path.
                unreachable!("control flow instruction encountered during type check")
            },
            Instruction::Pop => {
                let ty = operand_stack.pop_ty()?;
                ty.paranoid_check_has_ability(Ability::Drop)?;
            },
            Instruction::LdU8(_) => {
                let u8_ty = ty_builder.create_u8_ty();
                operand_stack.push_ty(u8_ty)?
            },
            Instruction::LdU16(_) => {
                let u16_ty = ty_builder.create_u16_ty();
                operand_stack.push_ty(u16_ty)?
            },
            Instruction::LdU32(_) => {
                let u32_ty = ty_builder.create_u32_ty();
                operand_stack.push_ty(u32_ty)?
            },
            Instruction::LdU64(_) => {
                let u64_ty = ty_builder.create_u64_ty();
                operand_stack.push_ty(u64_ty)?
            },
            Instruction::LdU128(_) => {
                let u128_ty = ty_builder.create_u128_ty();
                operand_stack.push_ty(u128_ty)?
            },
            Instruction::LdU256(_) => {
                let u256_ty = ty_builder.create_u256_ty();
                operand_stack.push_ty(u256_ty)?
            },
            Instruction::LdI8(_) => {
                let i8_ty = ty_builder.create_i8_ty();
                operand_stack.push_ty(i8_ty)?
            },
            Instruction::LdI16(_) => {
                let i16_ty = ty_builder.create_i16_ty();
                operand_stack.push_ty(i16_ty)?
            },
            Instruction::LdI32(_) => {
                let i32_ty = ty_builder.create_i32_ty();
                operand_stack.push_ty(i32_ty)?
            },
            Instruction::LdI64(_) => {
                let i64_ty = ty_builder.create_i64_ty();
                operand_stack.push_ty(i64_ty)?
            },
            Instruction::LdI128(_) => {
                let i128_ty = ty_builder.create_i128_ty();
                operand_stack.push_ty(i128_ty)?
            },
            Instruction::LdI256(_) => {
                let i256_ty = ty_builder.create_i256_ty();
                operand_stack.push_ty(i256_ty)?
            },
            Instruction::LdTrue | Instruction::LdFalse => {
                let bool_ty = ty_builder.create_bool_ty();
                operand_stack.push_ty(bool_ty)?
            },
            Instruction::LdConst(i) => {
                let constant = frame.constant_at(*i);
                let ty = ty_builder.create_constant_ty(&constant.type_)?;
                operand_stack.push_ty(ty)?;
            },
            Instruction::CopyLoc(idx) => {
                let ty = frame.local_ty_at(*idx as usize).clone();
                ty.paranoid_check_has_ability(Ability::Copy)?;
                operand_stack.push_ty(ty)?;
            },
            Instruction::MoveLoc(idx) => {
                let ty = frame.local_ty_at(*idx as usize).clone();
                operand_stack.push_ty(ty)?;
            },
            Instruction::StLoc(_) => (),
            Instruction::MutBorrowLoc(idx) => {
                let ty = frame.local_ty_at(*idx as usize);
                let mut_ref_ty = ty_builder.create_ref_ty(ty, true)?;
                operand_stack.push_ty(mut_ref_ty)?;
            },
            Instruction::ImmBorrowLoc(idx) => {
                let ty = frame.local_ty_at(*idx as usize);
                let ref_ty = ty_builder.create_ref_ty(ty, false)?;
                operand_stack.push_ty(ref_ty)?;
            },
            Instruction::ImmBorrowField(fh_idx) => {
                let ty = operand_stack.pop_ty()?;
                let expected_ty = frame.field_handle_to_struct(*fh_idx);
                ty.paranoid_check_ref_eq(&expected_ty, false)?;

                let field_ty = frame.get_field_ty(*fh_idx)?;
                let field_ref_ty = ty_builder.create_ref_ty(field_ty, false)?;
                operand_stack.push_ty(field_ref_ty)?;
            },
            Instruction::MutBorrowField(fh_idx) => {
                let ref_ty = operand_stack.pop_ty()?;
                let expected_inner_ty = frame.field_handle_to_struct(*fh_idx);
                ref_ty.paranoid_check_ref_eq(&expected_inner_ty, true)?;

                let field_ty = frame.get_field_ty(*fh_idx)?;
                let field_mut_ref_ty = ty_builder.create_ref_ty(field_ty, true)?;
                operand_stack.push_ty(field_mut_ref_ty)?;
            },
            Instruction::ImmBorrowFieldGeneric(idx) => {
                let struct_ty = operand_stack.pop_ty()?;
                let ((field_ty, _), (expected_struct_ty, _)) =
                    ty_cache.get_field_type_and_struct_type(*idx, frame)?;
                struct_ty.paranoid_check_ref_eq(expected_struct_ty, false)?;

                let field_ref_ty = ty_builder.create_ref_ty(field_ty, false)?;
                operand_stack.push_ty(field_ref_ty)?;
            },
            Instruction::MutBorrowFieldGeneric(idx) => {
                let struct_ty = operand_stack.pop_ty()?;
                let ((field_ty, _), (expected_struct_ty, _)) =
                    ty_cache.get_field_type_and_struct_type(*idx, frame)?;
                struct_ty.paranoid_check_ref_eq(expected_struct_ty, true)?;

                let field_mut_ref_ty = ty_builder.create_ref_ty(field_ty, true)?;
                operand_stack.push_ty(field_mut_ref_ty)?;
            },
            Instruction::ImmBorrowVariantField(fh_idx)
            | Instruction::MutBorrowVariantField(fh_idx) => {
                let is_mut = matches!(instruction, Instruction::MutBorrowVariantField(..));
                let field_info = frame.variant_field_info_at(*fh_idx);
                let ty = operand_stack.pop_ty()?;
                let expected_ty = frame.create_struct_ty(&field_info.definition_struct_type);
                ty.paranoid_check_ref_eq(&expected_ty, is_mut)?;
                let field_ty = &field_info.uninstantiated_field_ty;
                let field_ref_ty = ty_builder.create_ref_ty(field_ty, is_mut)?;
                operand_stack.push_ty(field_ref_ty)?;
            },
            Instruction::ImmBorrowVariantFieldGeneric(idx)
            | Instruction::MutBorrowVariantFieldGeneric(idx) => {
                let is_mut = matches!(instruction, Instruction::MutBorrowVariantFieldGeneric(..));
                let struct_ty = operand_stack.pop_ty()?;
                let ((field_ty, _), (expected_struct_ty, _)) =
                    ty_cache.get_variant_field_type_and_struct_type(*idx, frame)?;
                struct_ty.paranoid_check_ref_eq(expected_struct_ty, is_mut)?;
                let field_ref_ty = ty_builder.create_ref_ty(field_ty, is_mut)?;
                operand_stack.push_ty(field_ref_ty)?;
            },
            Instruction::PackClosure(..) | Instruction::PackClosureGeneric(..) => {
                // Skip: runtime checks are implemented in interpreter loop!
            },

            Instruction::Pack(idx) => {
                let field_count = frame.field_count(*idx);
                let args_ty = frame.get_struct(*idx);
                let field_tys = args_ty.fields(None)?.iter().map(|(_, ty)| ty);
                let output_ty = frame.get_struct_ty(*idx);
                verify_pack(operand_stack, field_count, field_tys, output_ty)?;
            },
            Instruction::PackGeneric(idx) => {
                let field_count = frame.field_instantiation_count(*idx);
                let output_ty = ty_cache.get_struct_type(*idx, frame)?.0.clone();
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
                    operand_stack,
                    field_count,
                    args_ty.iter().map(|(ty, _)| ty),
                    output_ty,
                )?;
            },
            Instruction::Unpack(idx) => {
                let struct_ty = operand_stack.pop_ty()?;
                struct_ty.paranoid_check_eq(&frame.get_struct_ty(*idx))?;
                let struct_decl = frame.get_struct(*idx);
                for (_name, ty) in struct_decl.fields(None)?.iter() {
                    operand_stack.push_ty(ty.clone())?;
                }
            },
            Instruction::UnpackGeneric(idx) => {
                let struct_ty = operand_stack.pop_ty()?;

                struct_ty.paranoid_check_eq(ty_cache.get_struct_type(*idx, frame)?.0)?;

                let struct_fields_types = ty_cache.get_struct_fields_types(*idx, frame)?;
                for (ty, _) in struct_fields_types {
                    operand_stack.push_ty(ty.clone())?;
                }
            },
            Instruction::PackVariant(idx) => {
                let info = frame.get_struct_variant_at(*idx);
                let field_tys = info
                    .definition_struct_type
                    .fields(Some(info.variant))?
                    .iter()
                    .map(|(_, ty)| ty);
                let output_ty = frame.create_struct_ty(&info.definition_struct_type);
                verify_pack(operand_stack, info.field_count, field_tys, output_ty)?;
            },
            Instruction::PackVariantGeneric(idx) => {
                let info = frame.get_struct_variant_instantiation_at(*idx);
                let output_ty = ty_cache.get_struct_variant_type(*idx, frame)?.0.clone();
                let args_ty = ty_cache.get_struct_variant_fields_types(*idx, frame)?;
                verify_pack(
                    operand_stack,
                    info.field_count,
                    args_ty.iter().map(|(ty, _)| ty),
                    output_ty,
                )?;
            },
            Instruction::UnpackVariant(idx) => {
                let info = frame.get_struct_variant_at(*idx);
                let expected_struct_ty = frame.create_struct_ty(&info.definition_struct_type);
                let actual_struct_ty = operand_stack.pop_ty()?;
                actual_struct_ty.paranoid_check_eq(&expected_struct_ty)?;
                for (_name, ty) in info
                    .definition_struct_type
                    .fields(Some(info.variant))?
                    .iter()
                {
                    operand_stack.push_ty(ty.clone())?;
                }
            },
            Instruction::UnpackVariantGeneric(idx) => {
                let expected_struct_type = ty_cache.get_struct_variant_type(*idx, frame)?.0;
                let actual_struct_type = operand_stack.pop_ty()?;
                actual_struct_type.paranoid_check_eq(expected_struct_type)?;
                let struct_fields_types = ty_cache.get_struct_variant_fields_types(*idx, frame)?;
                for (ty, _) in struct_fields_types {
                    operand_stack.push_ty(ty.clone())?;
                }
            },
            Instruction::TestVariant(idx) => {
                let info = frame.get_struct_variant_at(*idx);
                let expected_struct_ty = frame.create_struct_ty(&info.definition_struct_type);
                let actual_struct_ty = operand_stack.pop_ty()?;
                actual_struct_ty.paranoid_check_ref_eq(&expected_struct_ty, false)?;
                operand_stack.push_ty(ty_builder.create_bool_ty())?;
            },
            Instruction::TestVariantGeneric(idx) => {
                let expected_struct_ty = ty_cache.get_struct_variant_type(*idx, frame)?.0;
                let actual_struct_ty = operand_stack.pop_ty()?;
                actual_struct_ty.paranoid_check_ref_eq(expected_struct_ty, false)?;
                operand_stack.push_ty(ty_builder.create_bool_ty())?;
            },
            Instruction::ReadRef => {
                let ref_ty = operand_stack.pop_ty()?;
                let inner_ty = ref_ty.paranoid_read_ref()?;
                operand_stack.push_ty(inner_ty)?;
            },
            Instruction::WriteRef => {
                let mut_ref_ty = operand_stack.pop_ty()?;
                let val_ty = operand_stack.pop_ty()?;
                mut_ref_ty.paranoid_write_ref(&val_ty)?;
            },
            Instruction::CastU8 => {
                operand_stack.pop_ty()?;
                let u8_ty = ty_builder.create_u8_ty();
                operand_stack.push_ty(u8_ty)?;
            },
            Instruction::CastU16 => {
                operand_stack.pop_ty()?;
                let u16_ty = ty_builder.create_u16_ty();
                operand_stack.push_ty(u16_ty)?;
            },
            Instruction::CastU32 => {
                operand_stack.pop_ty()?;
                let u32_ty = ty_builder.create_u32_ty();
                operand_stack.push_ty(u32_ty)?;
            },
            Instruction::CastU64 => {
                operand_stack.pop_ty()?;
                let u64_ty = ty_builder.create_u64_ty();
                operand_stack.push_ty(u64_ty)?;
            },
            Instruction::CastU128 => {
                operand_stack.pop_ty()?;
                let u128_ty = ty_builder.create_u128_ty();
                operand_stack.push_ty(u128_ty)?;
            },
            Instruction::CastU256 => {
                operand_stack.pop_ty()?;
                let u256_ty = ty_builder.create_u256_ty();
                operand_stack.push_ty(u256_ty)?;
            },
            Instruction::CastI8 => {
                operand_stack.pop_ty()?;
                let i8_ty = ty_builder.create_i8_ty();
                operand_stack.push_ty(i8_ty)?;
            },
            Instruction::CastI16 => {
                operand_stack.pop_ty()?;
                let i16_ty = ty_builder.create_i16_ty();
                operand_stack.push_ty(i16_ty)?;
            },
            Instruction::CastI32 => {
                operand_stack.pop_ty()?;
                let i32_ty = ty_builder.create_i32_ty();
                operand_stack.push_ty(i32_ty)?;
            },
            Instruction::CastI64 => {
                operand_stack.pop_ty()?;
                let i64_ty = ty_builder.create_i64_ty();
                operand_stack.push_ty(i64_ty)?;
            },
            Instruction::CastI128 => {
                operand_stack.pop_ty()?;
                let i128_ty = ty_builder.create_i128_ty();
                operand_stack.push_ty(i128_ty)?;
            },
            Instruction::CastI256 => {
                operand_stack.pop_ty()?;
                let i256_ty = ty_builder.create_i256_ty();
                operand_stack.push_ty(i256_ty)?;
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
                rhs_ty.paranoid_check_eq(operand_stack.top_ty()?)?;
                // NO-OP, same as the two lines below when the types are indeed the same:
                // let lhs_ty = operand_stack.pop_ty()?;
                // operand_stack.push_ty(rhs_ty)?;
            },
            Instruction::Negate => {
                operand_stack.top_ty()?.paranoid_check_is_sint_ty()?;
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
                rhs_ty.paranoid_check_eq(&lhs_ty)?;

                let bool_ty = ty_builder.create_bool_ty();
                operand_stack.push_ty(bool_ty)?;
            },
            Instruction::Eq | Instruction::Neq => {
                let rhs_ty = operand_stack.pop_ty()?;
                let lhs_ty = operand_stack.pop_ty()?;
                rhs_ty.paranoid_check_eq(&lhs_ty)?;
                rhs_ty.paranoid_check_has_ability(Ability::Drop)?;

                let bool_ty = ty_builder.create_bool_ty();
                operand_stack.push_ty(bool_ty)?;
            },
            Instruction::MutBorrowGlobal(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let struct_ty = frame.get_struct_ty(*idx);
                struct_ty.paranoid_check_has_ability(Ability::Key)?;

                let struct_mut_ref_ty = ty_builder.create_ref_ty(&struct_ty, true)?;
                operand_stack.push_ty(struct_mut_ref_ty)?;
            },
            Instruction::ImmBorrowGlobal(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let struct_ty = frame.get_struct_ty(*idx);
                struct_ty.paranoid_check_has_ability(Ability::Key)?;

                let struct_ref_ty = ty_builder.create_ref_ty(&struct_ty, false)?;
                operand_stack.push_ty(struct_ref_ty)?;
            },
            Instruction::MutBorrowGlobalGeneric(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let struct_ty = ty_cache.get_struct_type(*idx, frame)?.0;
                struct_ty.paranoid_check_has_ability(Ability::Key)?;

                let struct_mut_ref_ty = ty_builder.create_ref_ty(struct_ty, true)?;
                operand_stack.push_ty(struct_mut_ref_ty)?;
            },
            Instruction::ImmBorrowGlobalGeneric(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let struct_ty = ty_cache.get_struct_type(*idx, frame)?.0;
                struct_ty.paranoid_check_has_ability(Ability::Key)?;

                let struct_ref_ty = ty_builder.create_ref_ty(struct_ty, false)?;
                operand_stack.push_ty(struct_ref_ty)?;
            },
            Instruction::Exists(_) | Instruction::ExistsGeneric(_) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;

                let bool_ty = ty_builder.create_bool_ty();
                operand_stack.push_ty(bool_ty)?;
            },
            Instruction::MoveTo(idx) => {
                let ty = operand_stack.pop_ty()?;
                operand_stack.pop_ty()?.paranoid_check_is_signer_ref_ty()?;
                ty.paranoid_check_eq(&frame.get_struct_ty(*idx))?;
                ty.paranoid_check_has_ability(Ability::Key)?;
            },
            Instruction::MoveToGeneric(idx) => {
                let ty = operand_stack.pop_ty()?;
                operand_stack.pop_ty()?.paranoid_check_is_signer_ref_ty()?;
                ty.paranoid_check_eq(ty_cache.get_struct_type(*idx, frame)?.0)?;
                ty.paranoid_check_has_ability(Ability::Key)?;
            },
            Instruction::MoveFrom(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let ty = frame.get_struct_ty(*idx);
                ty.paranoid_check_has_ability(Ability::Key)?;
                operand_stack.push_ty(ty)?;
            },
            Instruction::MoveFromGeneric(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let ty = ty_cache.get_struct_type(*idx, frame)?.0.clone();
                ty.paranoid_check_has_ability(Ability::Key)?;
                operand_stack.push_ty(ty)?;
            },
            Instruction::FreezeRef => {
                let mut_ref_ty = operand_stack.pop_ty()?;
                let ref_ty = mut_ref_ty.paranoid_freeze_ref_ty()?;
                operand_stack.push_ty(ref_ty)?;
            },
            Instruction::Nop => (),
            Instruction::Not => {
                operand_stack.top_ty()?.paranoid_check_is_bool_ty()?;
                // NO-OP,  same as the two lines below:
                // let bool_ty = ty_builder.create_bool_ty();
                // operand_stack.push_ty(bool_ty)?;
            },
            Instruction::VecPack(si, num) => {
                let (ty, _, _) = ty_cache.get_signature_index_type(*si, frame)?;
                let elem_tys = operand_stack.popn_tys(*num as u16)?;
                for elem_ty in elem_tys.iter() {
                    // For vector element types, use assignability
                    elem_ty.paranoid_check_assignable(ty)?;
                }

                let vec_ty = ty_builder.create_vec_ty(ty)?;
                operand_stack.push_ty(vec_ty)?;
            },
            Instruction::VecLen(si) => {
                let (ty, _, _) = ty_cache.get_signature_index_type(*si, frame)?;
                operand_stack
                    .pop_ty()?
                    .paranoid_check_is_vec_ref_ty::<false>(ty)?;

                let u64_ty = ty_builder.create_u64_ty();
                operand_stack.push_ty(u64_ty)?;
            },
            Instruction::VecImmBorrow(si) => {
                let (ty, _, _) = ty_cache.get_signature_index_type(*si, frame)?;
                operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                let elem_ref_ty = operand_stack
                    .pop_ty()?
                    .paranoid_check_and_get_vec_elem_ref_ty::<false>(ty)?;

                operand_stack.push_ty(elem_ref_ty)?;
            },
            Instruction::VecMutBorrow(si) => {
                let (ty, _, _) = ty_cache.get_signature_index_type(*si, frame)?;
                operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                let elem_ref_ty = operand_stack
                    .pop_ty()?
                    .paranoid_check_and_get_vec_elem_ref_ty::<true>(ty)?;
                operand_stack.push_ty(elem_ref_ty)?;
            },
            Instruction::VecPushBack(si) => {
                let (ty, _, _) = ty_cache.get_signature_index_type(*si, frame)?;
                // For pushing an element to a vector, use assignability
                operand_stack.pop_ty()?.paranoid_check_assignable(ty)?;
                operand_stack
                    .pop_ty()?
                    .paranoid_check_is_vec_ref_ty::<true>(ty)?;
            },
            Instruction::VecPopBack(si) => {
                let (ty, _, _) = ty_cache.get_signature_index_type(*si, frame)?;
                let elem_ty = operand_stack
                    .pop_ty()?
                    .paranoid_check_and_get_vec_elem_ty::<true>(ty)?;
                operand_stack.push_ty(elem_ty)?;
            },
            Instruction::VecUnpack(si, num) => {
                let (expected_elem_ty, _, _) = ty_cache.get_signature_index_type(*si, frame)?;
                let vec_ty = operand_stack.pop_ty()?;
                vec_ty.paranoid_check_is_vec_ty(expected_elem_ty)?;
                for _ in 0..*num {
                    operand_stack.push_ty(expected_elem_ty.clone())?;
                }
            },
            Instruction::VecSwap(si) => {
                let (ty, _, _) = ty_cache.get_signature_index_type(*si, frame)?;
                operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                operand_stack
                    .pop_ty()?
                    .paranoid_check_is_vec_ref_ty::<true>(ty)?;
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
