// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{frame_type_cache::FrameTypeCache, interpreter::Stack, loader::Resolver};
use move_binary_format::{
    errors::*,
    file_format::{Ability, AbilitySet, Bytecode},
};
use move_core_types::vm_status::StatusCode;
use move_vm_types::{loaded_data::runtime_types::Type, values::Locals};

pub(crate) trait RuntimeTypeCheck {
    /// Paranoid type checks to perform before instruction execution.
    fn pre_execution_type_stack_transition(
        local_tys: &[Type],
        locals: &Locals,
        ty_args: &[Type],
        resolver: &Resolver,
        operand_stack: &mut Stack,
        instruction: &Bytecode,
    ) -> PartialVMResult<()>;

    /// Paranoid type checks to perform after instruction execution.
    fn post_execution_type_stack_transition(
        local_tys: &[Type],
        ty_args: &[Type],
        resolver: &Resolver,
        operand_stack: &mut Stack,
        ty_cache: &mut FrameTypeCache,
        instruction: &Bytecode,
    ) -> PartialVMResult<()>;

    /// Paranoid check that operand and type stacks have the same size
    fn check_operand_stack_balance(operand_stack: &Stack) -> PartialVMResult<()>;

    /// For any other checks are performed externally
    fn should_perform_checks() -> bool;
}

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
        ty.paranoid_check_eq(expected_ty)?;
    }

    operand_stack.push_ty(output_ty)
}

pub(crate) struct NoRuntimeTypeCheck;
pub(crate) struct FullRuntimeTypeCheck;

impl RuntimeTypeCheck for NoRuntimeTypeCheck {
    fn pre_execution_type_stack_transition(
        _local_tys: &[Type],
        _locals: &Locals,
        _ty_args: &[Type],
        _resolver: &Resolver,
        _operand_stack: &mut Stack,
        _instruction: &Bytecode,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn post_execution_type_stack_transition(
        _local_tys: &[Type],
        _ty_args: &[Type],
        _resolver: &Resolver,
        _operand_stack: &mut Stack,
        _ty_cache: &mut FrameTypeCache,
        _instruction: &Bytecode,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn check_operand_stack_balance(_operand_stack: &Stack) -> PartialVMResult<()> {
        Ok(())
    }

    #[inline(always)]
    fn should_perform_checks() -> bool {
        false
    }
}

impl RuntimeTypeCheck for FullRuntimeTypeCheck {
    /// Note that most of the checks should happen after instruction execution, because gas charging will happen during
    /// instruction execution and we want to avoid running code without charging proper gas as much as possible.
    fn pre_execution_type_stack_transition(
        local_tys: &[Type],
        locals: &Locals,
        _ty_args: &[Type],
        _resolver: &Resolver,
        operand_stack: &mut Stack,
        instruction: &Bytecode,
    ) -> PartialVMResult<()> {
        match instruction {
            // Call instruction will be checked at execute_main.
            Bytecode::Call(_) | Bytecode::CallGeneric(_) => (),
            Bytecode::BrFalse(_) | Bytecode::BrTrue(_) => {
                operand_stack.pop_ty()?;
            },
            Bytecode::Branch(_) => (),
            Bytecode::Ret => {
                for (idx, ty) in local_tys.iter().enumerate() {
                    if !locals.is_invalid(idx)? {
                        ty.paranoid_check_has_ability(Ability::Drop)?;
                    }
                }
            },
            Bytecode::Abort => {
                operand_stack.pop_ty()?;
            },
            // StLoc needs to check before execution as we need to check the drop ability of values.
            Bytecode::StLoc(idx) => {
                let ty = local_tys[*idx as usize].clone();
                let val_ty = operand_stack.pop_ty()?;
                ty.paranoid_check_eq(&val_ty)?;
                if !locals.is_invalid(*idx as usize)? {
                    ty.paranoid_check_has_ability(Ability::Drop)?;
                }
            },
            // We will check the rest of the instructions after execution phase.
            Bytecode::Pop
            | Bytecode::LdU8(_)
            | Bytecode::LdU16(_)
            | Bytecode::LdU32(_)
            | Bytecode::LdU64(_)
            | Bytecode::LdU128(_)
            | Bytecode::LdU256(_)
            | Bytecode::LdTrue
            | Bytecode::LdFalse
            | Bytecode::LdConst(_)
            | Bytecode::CopyLoc(_)
            | Bytecode::MoveLoc(_)
            | Bytecode::MutBorrowLoc(_)
            | Bytecode::ImmBorrowLoc(_)
            | Bytecode::ImmBorrowField(_)
            | Bytecode::MutBorrowField(_)
            | Bytecode::ImmBorrowFieldGeneric(_)
            | Bytecode::MutBorrowFieldGeneric(_)
            | Bytecode::Pack(_)
            | Bytecode::PackGeneric(_)
            | Bytecode::Unpack(_)
            | Bytecode::UnpackGeneric(_)
            | Bytecode::ReadRef
            | Bytecode::WriteRef
            | Bytecode::CastU8
            | Bytecode::CastU16
            | Bytecode::CastU32
            | Bytecode::CastU64
            | Bytecode::CastU128
            | Bytecode::CastU256
            | Bytecode::Add
            | Bytecode::Sub
            | Bytecode::Mul
            | Bytecode::Mod
            | Bytecode::Div
            | Bytecode::BitOr
            | Bytecode::BitAnd
            | Bytecode::Xor
            | Bytecode::Or
            | Bytecode::And
            | Bytecode::Shl
            | Bytecode::Shr
            | Bytecode::Lt
            | Bytecode::Le
            | Bytecode::Gt
            | Bytecode::Ge
            | Bytecode::Eq
            | Bytecode::Neq
            | Bytecode::MutBorrowGlobal(_)
            | Bytecode::ImmBorrowGlobal(_)
            | Bytecode::MutBorrowGlobalGeneric(_)
            | Bytecode::ImmBorrowGlobalGeneric(_)
            | Bytecode::Exists(_)
            | Bytecode::ExistsGeneric(_)
            | Bytecode::MoveTo(_)
            | Bytecode::MoveToGeneric(_)
            | Bytecode::MoveFrom(_)
            | Bytecode::MoveFromGeneric(_)
            | Bytecode::FreezeRef
            | Bytecode::Nop
            | Bytecode::Not
            | Bytecode::VecPack(_, _)
            | Bytecode::VecLen(_)
            | Bytecode::VecImmBorrow(_)
            | Bytecode::VecMutBorrow(_)
            | Bytecode::VecPushBack(_)
            | Bytecode::VecPopBack(_)
            | Bytecode::VecUnpack(_, _)
            | Bytecode::VecSwap(_) => (),

            // Since bytecode version 7
            Bytecode::PackVariant(_)
            | Bytecode::PackVariantGeneric(_)
            | Bytecode::UnpackVariant(_)
            | Bytecode::UnpackVariantGeneric(_)
            | Bytecode::TestVariant(_)
            | Bytecode::TestVariantGeneric(_)
            | Bytecode::MutBorrowVariantField(_)
            | Bytecode::MutBorrowVariantFieldGeneric(_)
            | Bytecode::ImmBorrowVariantField(_)
            | Bytecode::ImmBorrowVariantFieldGeneric(_) => (),
        };
        Ok(())
    }

    /// Paranoid type checks to perform after instruction execution.
    ///
    /// This function and `pre_execution_type_stack_transition` should
    /// constitute the full type stack transition for the paranoid
    /// mode.
    fn post_execution_type_stack_transition(
        local_tys: &[Type],
        ty_args: &[Type],
        resolver: &Resolver,
        operand_stack: &mut Stack,
        ty_cache: &mut FrameTypeCache,
        instruction: &Bytecode,
    ) -> PartialVMResult<()> {
        let ty_builder = resolver.loader().ty_builder();

        match instruction {
            Bytecode::BrTrue(_) | Bytecode::BrFalse(_) => (),
            Bytecode::Branch(_)
            | Bytecode::Ret
            | Bytecode::Call(_)
            | Bytecode::CallGeneric(_)
            | Bytecode::Abort => {
                // Invariants hold because all of the instructions
                // above will force VM to break from the interpreter
                // loop and thus not hit this code path.
                unreachable!("control flow instruction encountered during type check")
            },
            Bytecode::Pop => {
                let ty = operand_stack.pop_ty()?;
                ty.paranoid_check_has_ability(Ability::Drop)?;
            },
            Bytecode::LdU8(_) => {
                let u8_ty = ty_builder.create_u8_ty();
                operand_stack.push_ty(u8_ty)?
            },
            Bytecode::LdU16(_) => {
                let u16_ty = ty_builder.create_u16_ty();
                operand_stack.push_ty(u16_ty)?
            },
            Bytecode::LdU32(_) => {
                let u32_ty = ty_builder.create_u32_ty();
                operand_stack.push_ty(u32_ty)?
            },
            Bytecode::LdU64(_) => {
                let u64_ty = ty_builder.create_u64_ty();
                operand_stack.push_ty(u64_ty)?
            },
            Bytecode::LdU128(_) => {
                let u128_ty = ty_builder.create_u128_ty();
                operand_stack.push_ty(u128_ty)?
            },
            Bytecode::LdU256(_) => {
                let u256_ty = ty_builder.create_u256_ty();
                operand_stack.push_ty(u256_ty)?
            },
            Bytecode::LdTrue | Bytecode::LdFalse => {
                let bool_ty = ty_builder.create_bool_ty();
                operand_stack.push_ty(bool_ty)?
            },
            Bytecode::LdConst(i) => {
                let constant = resolver.constant_at(*i);
                let ty = ty_builder.create_constant_ty(&constant.type_)?;
                operand_stack.push_ty(ty)?;
            },
            Bytecode::CopyLoc(idx) => {
                let ty = local_tys[*idx as usize].clone();
                ty.paranoid_check_has_ability(Ability::Copy)?;
                operand_stack.push_ty(ty)?;
            },
            Bytecode::MoveLoc(idx) => {
                let ty = local_tys[*idx as usize].clone();
                operand_stack.push_ty(ty)?;
            },
            Bytecode::StLoc(_) => (),
            Bytecode::MutBorrowLoc(idx) => {
                let ty = &local_tys[*idx as usize];
                let mut_ref_ty = ty_builder.create_ref_ty(ty, true)?;
                operand_stack.push_ty(mut_ref_ty)?;
            },
            Bytecode::ImmBorrowLoc(idx) => {
                let ty = &local_tys[*idx as usize];
                let ref_ty = ty_builder.create_ref_ty(ty, false)?;
                operand_stack.push_ty(ref_ty)?;
            },
            Bytecode::ImmBorrowField(fh_idx) => {
                let ty = operand_stack.pop_ty()?;
                let expected_ty = resolver.field_handle_to_struct(*fh_idx);
                ty.paranoid_check_ref_eq(&expected_ty, false)?;

                let field_ty = resolver.get_field_ty(*fh_idx)?;
                let field_ref_ty = ty_builder.create_ref_ty(field_ty, false)?;
                operand_stack.push_ty(field_ref_ty)?;
            },
            Bytecode::MutBorrowField(fh_idx) => {
                let ref_ty = operand_stack.pop_ty()?;
                let expected_inner_ty = resolver.field_handle_to_struct(*fh_idx);
                ref_ty.paranoid_check_ref_eq(&expected_inner_ty, true)?;

                let field_ty = resolver.get_field_ty(*fh_idx)?;
                let field_mut_ref_ty = ty_builder.create_ref_ty(field_ty, true)?;
                operand_stack.push_ty(field_mut_ref_ty)?;
            },
            Bytecode::ImmBorrowFieldGeneric(idx) => {
                let struct_ty = operand_stack.pop_ty()?;
                let ((field_ty, _), (expected_struct_ty, _)) =
                    ty_cache.get_field_type_and_struct_type(*idx, resolver, ty_args)?;
                struct_ty.paranoid_check_ref_eq(expected_struct_ty, false)?;

                let field_ref_ty = ty_builder.create_ref_ty(field_ty, false)?;
                operand_stack.push_ty(field_ref_ty)?;
            },
            Bytecode::MutBorrowFieldGeneric(idx) => {
                let struct_ty = operand_stack.pop_ty()?;
                let ((field_ty, _), (expected_struct_ty, _)) =
                    ty_cache.get_field_type_and_struct_type(*idx, resolver, ty_args)?;
                struct_ty.paranoid_check_ref_eq(expected_struct_ty, true)?;

                let field_mut_ref_ty = ty_builder.create_ref_ty(field_ty, true)?;
                operand_stack.push_ty(field_mut_ref_ty)?;
            },
            Bytecode::ImmBorrowVariantField(fh_idx) | Bytecode::MutBorrowVariantField(fh_idx) => {
                let is_mut = matches!(instruction, Bytecode::MutBorrowVariantField(..));
                let field_info = resolver.variant_field_info_at(*fh_idx);
                let ty = operand_stack.pop_ty()?;
                let expected_ty = resolver.create_struct_ty(&field_info.definition_struct_type);
                ty.paranoid_check_ref_eq(&expected_ty, is_mut)?;
                let field_ty = &field_info.uninstantiated_field_ty;
                let field_ref_ty = ty_builder.create_ref_ty(field_ty, is_mut)?;
                operand_stack.push_ty(field_ref_ty)?;
            },
            Bytecode::ImmBorrowVariantFieldGeneric(idx)
            | Bytecode::MutBorrowVariantFieldGeneric(idx) => {
                let is_mut = matches!(instruction, Bytecode::MutBorrowVariantFieldGeneric(..));
                let struct_ty = operand_stack.pop_ty()?;
                let ((field_ty, _), (expected_struct_ty, _)) =
                    ty_cache.get_variant_field_type_and_struct_type(*idx, resolver, ty_args)?;
                struct_ty.paranoid_check_ref_eq(expected_struct_ty, is_mut)?;
                let field_ref_ty = ty_builder.create_ref_ty(field_ty, is_mut)?;
                operand_stack.push_ty(field_ref_ty)?;
            },
            Bytecode::Pack(idx) => {
                let field_count = resolver.field_count(*idx);
                let args_ty = resolver.get_struct(*idx)?;
                let field_tys = args_ty.fields(None)?.iter().map(|(_, ty)| ty);
                let output_ty = resolver.get_struct_ty(*idx);
                verify_pack(operand_stack, field_count, field_tys, output_ty)?;
            },
            Bytecode::PackGeneric(idx) => {
                let field_count = resolver.field_instantiation_count(*idx);
                let output_ty = ty_cache.get_struct_type(*idx, resolver, ty_args)?.0.clone();
                let args_ty = ty_cache.get_struct_fields_types(*idx, resolver, ty_args)?;

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
            Bytecode::Unpack(idx) => {
                let struct_ty = operand_stack.pop_ty()?;
                struct_ty.paranoid_check_eq(&resolver.get_struct_ty(*idx))?;
                let struct_decl = resolver.get_struct(*idx)?;
                for (_name, ty) in struct_decl.fields(None)?.iter() {
                    operand_stack.push_ty(ty.clone())?;
                }
            },
            Bytecode::UnpackGeneric(idx) => {
                let struct_ty = operand_stack.pop_ty()?;

                struct_ty
                    .paranoid_check_eq(ty_cache.get_struct_type(*idx, resolver, ty_args)?.0)?;

                let struct_fields_types =
                    ty_cache.get_struct_fields_types(*idx, resolver, ty_args)?;
                for (ty, _) in struct_fields_types {
                    operand_stack.push_ty(ty.clone())?;
                }
            },
            Bytecode::PackVariant(idx) => {
                let info = resolver.get_struct_variant_at(*idx);
                let field_tys = info
                    .definition_struct_type
                    .fields(Some(info.variant))?
                    .iter()
                    .map(|(_, ty)| ty);
                let output_ty = resolver.create_struct_ty(&info.definition_struct_type);
                verify_pack(operand_stack, info.field_count, field_tys, output_ty)?;
            },
            Bytecode::PackVariantGeneric(idx) => {
                let info = resolver.get_struct_variant_instantiation_at(*idx);
                let output_ty = ty_cache
                    .get_struct_variant_type(*idx, resolver, ty_args)?
                    .0
                    .clone();
                let args_ty = ty_cache.get_struct_variant_fields_types(*idx, resolver, ty_args)?;
                verify_pack(
                    operand_stack,
                    info.field_count,
                    args_ty.iter().map(|(ty, _)| ty),
                    output_ty,
                )?;
            },
            Bytecode::UnpackVariant(idx) => {
                let info = resolver.get_struct_variant_at(*idx);
                let expected_struct_ty = resolver.create_struct_ty(&info.definition_struct_type);
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
            Bytecode::UnpackVariantGeneric(idx) => {
                let expected_struct_type =
                    ty_cache.get_struct_variant_type(*idx, resolver, ty_args)?.0;
                let actual_struct_type = operand_stack.pop_ty()?;
                actual_struct_type.paranoid_check_eq(expected_struct_type)?;
                let struct_fields_types =
                    ty_cache.get_struct_variant_fields_types(*idx, resolver, ty_args)?;
                for (ty, _) in struct_fields_types {
                    operand_stack.push_ty(ty.clone())?;
                }
            },
            Bytecode::TestVariant(idx) => {
                let info = resolver.get_struct_variant_at(*idx);
                let expected_struct_ty = resolver.create_struct_ty(&info.definition_struct_type);
                let actual_struct_ty = operand_stack.pop_ty()?;
                actual_struct_ty.paranoid_check_ref_eq(&expected_struct_ty, false)?;
                operand_stack.push_ty(ty_builder.create_bool_ty())?;
            },
            Bytecode::TestVariantGeneric(idx) => {
                let expected_struct_ty =
                    ty_cache.get_struct_variant_type(*idx, resolver, ty_args)?.0;
                let actual_struct_ty = operand_stack.pop_ty()?;
                actual_struct_ty.paranoid_check_ref_eq(expected_struct_ty, false)?;
                operand_stack.push_ty(ty_builder.create_bool_ty())?;
            },
            Bytecode::ReadRef => {
                let ref_ty = operand_stack.pop_ty()?;
                let inner_ty = ref_ty.paranoid_read_ref()?;
                operand_stack.push_ty(inner_ty)?;
            },
            Bytecode::WriteRef => {
                let mut_ref_ty = operand_stack.pop_ty()?;
                let val_ty = operand_stack.pop_ty()?;
                mut_ref_ty.paranoid_write_ref(&val_ty)?;
            },
            Bytecode::CastU8 => {
                operand_stack.pop_ty()?;
                let u8_ty = ty_builder.create_u8_ty();
                operand_stack.push_ty(u8_ty)?;
            },
            Bytecode::CastU16 => {
                operand_stack.pop_ty()?;
                let u16_ty = ty_builder.create_u16_ty();
                operand_stack.push_ty(u16_ty)?;
            },
            Bytecode::CastU32 => {
                operand_stack.pop_ty()?;
                let u32_ty = ty_builder.create_u32_ty();
                operand_stack.push_ty(u32_ty)?;
            },
            Bytecode::CastU64 => {
                operand_stack.pop_ty()?;
                let u64_ty = ty_builder.create_u64_ty();
                operand_stack.push_ty(u64_ty)?;
            },
            Bytecode::CastU128 => {
                operand_stack.pop_ty()?;
                let u128_ty = ty_builder.create_u128_ty();
                operand_stack.push_ty(u128_ty)?;
            },
            Bytecode::CastU256 => {
                operand_stack.pop_ty()?;
                let u256_ty = ty_builder.create_u256_ty();
                operand_stack.push_ty(u256_ty)?;
            },
            Bytecode::Add
            | Bytecode::Sub
            | Bytecode::Mul
            | Bytecode::Mod
            | Bytecode::Div
            | Bytecode::BitOr
            | Bytecode::BitAnd
            | Bytecode::Xor
            | Bytecode::Or
            | Bytecode::And => {
                let rhs_ty = operand_stack.pop_ty()?;
                let lhs_ty = operand_stack.pop_ty()?;
                rhs_ty.paranoid_check_eq(&lhs_ty)?;
                operand_stack.push_ty(rhs_ty)?;
            },
            Bytecode::Shl | Bytecode::Shr => {
                let _rhs = operand_stack.pop_ty()?;
                let lhs = operand_stack.pop_ty()?;
                operand_stack.push_ty(lhs)?;
            },
            Bytecode::Lt | Bytecode::Le | Bytecode::Gt | Bytecode::Ge => {
                let rhs_ty = operand_stack.pop_ty()?;
                let lhs_ty = operand_stack.pop_ty()?;
                rhs_ty.paranoid_check_eq(&lhs_ty)?;

                let bool_ty = ty_builder.create_bool_ty();
                operand_stack.push_ty(bool_ty)?;
            },
            Bytecode::Eq | Bytecode::Neq => {
                let rhs_ty = operand_stack.pop_ty()?;
                let lhs_ty = operand_stack.pop_ty()?;
                rhs_ty.paranoid_check_eq(&lhs_ty)?;
                rhs_ty.paranoid_check_has_ability(Ability::Drop)?;

                let bool_ty = ty_builder.create_bool_ty();
                operand_stack.push_ty(bool_ty)?;
            },
            Bytecode::MutBorrowGlobal(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let struct_ty = resolver.get_struct_ty(*idx);
                struct_ty.paranoid_check_has_ability(Ability::Key)?;

                let struct_mut_ref_ty = ty_builder.create_ref_ty(&struct_ty, true)?;
                operand_stack.push_ty(struct_mut_ref_ty)?;
            },
            Bytecode::ImmBorrowGlobal(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let struct_ty = resolver.get_struct_ty(*idx);
                struct_ty.paranoid_check_has_ability(Ability::Key)?;

                let struct_ref_ty = ty_builder.create_ref_ty(&struct_ty, false)?;
                operand_stack.push_ty(struct_ref_ty)?;
            },
            Bytecode::MutBorrowGlobalGeneric(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let struct_ty = ty_cache.get_struct_type(*idx, resolver, ty_args)?.0;
                struct_ty.paranoid_check_has_ability(Ability::Key)?;

                let struct_mut_ref_ty = ty_builder.create_ref_ty(struct_ty, true)?;
                operand_stack.push_ty(struct_mut_ref_ty)?;
            },
            Bytecode::ImmBorrowGlobalGeneric(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let struct_ty = ty_cache.get_struct_type(*idx, resolver, ty_args)?.0;
                struct_ty.paranoid_check_has_ability(Ability::Key)?;

                let struct_ref_ty = ty_builder.create_ref_ty(struct_ty, false)?;
                operand_stack.push_ty(struct_ref_ty)?;
            },
            Bytecode::Exists(_) | Bytecode::ExistsGeneric(_) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;

                let bool_ty = ty_builder.create_bool_ty();
                operand_stack.push_ty(bool_ty)?;
            },
            Bytecode::MoveTo(idx) => {
                let ty = operand_stack.pop_ty()?;
                operand_stack.pop_ty()?.paranoid_check_is_signer_ref_ty()?;
                ty.paranoid_check_eq(&resolver.get_struct_ty(*idx))?;
                ty.paranoid_check_has_ability(Ability::Key)?;
            },
            Bytecode::MoveToGeneric(idx) => {
                let ty = operand_stack.pop_ty()?;
                operand_stack.pop_ty()?.paranoid_check_is_signer_ref_ty()?;
                ty.paranoid_check_eq(ty_cache.get_struct_type(*idx, resolver, ty_args)?.0)?;
                ty.paranoid_check_has_ability(Ability::Key)?;
            },
            Bytecode::MoveFrom(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let ty = resolver.get_struct_ty(*idx);
                ty.paranoid_check_has_ability(Ability::Key)?;
                operand_stack.push_ty(ty)?;
            },
            Bytecode::MoveFromGeneric(idx) => {
                operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                let ty = ty_cache.get_struct_type(*idx, resolver, ty_args)?.0.clone();
                ty.paranoid_check_has_ability(Ability::Key)?;
                operand_stack.push_ty(ty)?;
            },
            Bytecode::FreezeRef => {
                let mut_ref_ty = operand_stack.pop_ty()?;
                let ref_ty = mut_ref_ty.paranoid_freeze_ref_ty()?;
                operand_stack.push_ty(ref_ty)?;
            },
            Bytecode::Nop => (),
            Bytecode::Not => {
                operand_stack.pop_ty()?.paranoid_check_is_bool_ty()?;
                let bool_ty = ty_builder.create_bool_ty();
                operand_stack.push_ty(bool_ty)?;
            },
            Bytecode::VecPack(si, num) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, resolver, ty_args)?;
                let elem_tys = operand_stack.popn_tys(*num as u16)?;
                for elem_ty in elem_tys.iter() {
                    elem_ty.paranoid_check_eq(ty)?;
                }

                let vec_ty = ty_builder.create_vec_ty(ty)?;
                operand_stack.push_ty(vec_ty)?;
            },
            Bytecode::VecLen(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, resolver, ty_args)?;
                operand_stack
                    .pop_ty()?
                    .paranoid_check_is_vec_ref_ty(ty, false)?;

                let u64_ty = ty_builder.create_u64_ty();
                operand_stack.push_ty(u64_ty)?;
            },
            Bytecode::VecImmBorrow(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, resolver, ty_args)?;
                operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                let elem_ref_ty = operand_stack
                    .pop_ty()?
                    .paranoid_check_and_get_vec_elem_ref_ty(ty, false)?;

                operand_stack.push_ty(elem_ref_ty)?;
            },
            Bytecode::VecMutBorrow(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, resolver, ty_args)?;
                operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                let elem_ref_ty = operand_stack
                    .pop_ty()?
                    .paranoid_check_and_get_vec_elem_ref_ty(ty, true)?;
                operand_stack.push_ty(elem_ref_ty)?;
            },
            Bytecode::VecPushBack(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, resolver, ty_args)?;
                operand_stack.pop_ty()?.paranoid_check_eq(ty)?;
                operand_stack
                    .pop_ty()?
                    .paranoid_check_is_vec_ref_ty(ty, true)?;
            },
            Bytecode::VecPopBack(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, resolver, ty_args)?;
                let elem_ty = operand_stack
                    .pop_ty()?
                    .paranoid_check_and_get_vec_elem_ty(ty, true)?;
                operand_stack.push_ty(elem_ty)?;
            },
            Bytecode::VecUnpack(si, num) => {
                let (expected_elem_ty, _) =
                    ty_cache.get_signature_index_type(*si, resolver, ty_args)?;
                let vec_ty = operand_stack.pop_ty()?;
                vec_ty.paranoid_check_is_vec_ty(expected_elem_ty)?;
                for _ in 0..*num {
                    operand_stack.push_ty(expected_elem_ty.clone())?;
                }
            },
            Bytecode::VecSwap(si) => {
                let (ty, _) = ty_cache.get_signature_index_type(*si, resolver, ty_args)?;
                operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                operand_stack
                    .pop_ty()?
                    .paranoid_check_is_vec_ref_ty(ty, true)?;
            },
        }
        Ok(())
    }

    fn check_operand_stack_balance(operand_stack: &Stack) -> PartialVMResult<()> {
        operand_stack.check_balance()
    }

    #[inline(always)]
    fn should_perform_checks() -> bool {
        true
    }
}
