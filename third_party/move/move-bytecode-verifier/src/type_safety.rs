// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module defines the transfer functions for verifying type safety of a procedure body.
//! It does not utilize control flow, but does check each block independently

use crate::meter::{Meter, Scope};
use move_binary_format::{
    binary_views::{BinaryIndexedView, FunctionView},
    control_flow_graph::ControlFlowGraph,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, CodeOffset, FunctionAttribute, FunctionDefinitionIndex, FunctionHandle,
        FunctionHandleIndex, LocalIndex, Signature, SignatureToken, SignatureToken as ST,
        StructDefinition, StructDefinitionIndex, StructFieldInformation, StructHandleIndex,
        VariantIndex,
    },
    safe_assert, safe_unwrap,
    views::FieldOrVariantIndex,
};
use move_core_types::{ability::AbilitySet, function::ClosureMask, vm_status::StatusCode};

struct Locals<'a> {
    param_count: usize,
    parameters: &'a Signature,
    locals: &'a Signature,
}

const TYPE_NODE_COST: u128 = 30;

impl<'a> Locals<'a> {
    fn new(parameters: &'a Signature, locals: &'a Signature) -> Self {
        Self {
            param_count: parameters.len(),
            parameters,
            locals,
        }
    }

    fn local_at(&self, i: LocalIndex) -> &SignatureToken {
        let idx = i as usize;
        if idx < self.param_count {
            &self.parameters.0[idx]
        } else {
            &self.locals.0[idx - self.param_count]
        }
    }
}

struct TypeSafetyChecker<'a> {
    resolver: &'a BinaryIndexedView<'a>,
    function_view: &'a FunctionView<'a>,
    locals: Locals<'a>,
    stack: Vec<SignatureToken>,
}

impl<'a> TypeSafetyChecker<'a> {
    fn new(resolver: &'a BinaryIndexedView<'a>, function_view: &'a FunctionView<'a>) -> Self {
        let locals = Locals::new(function_view.parameters(), function_view.locals());
        Self {
            resolver,
            function_view,
            locals,
            stack: vec![],
        }
    }

    fn local_at(&self, i: LocalIndex) -> &SignatureToken {
        self.locals.local_at(i)
    }

    fn abilities(&self, t: &SignatureToken) -> PartialVMResult<AbilitySet> {
        self.resolver
            .abilities(t, self.function_view.type_parameters())
    }

    fn error(&self, status: StatusCode, offset: CodeOffset) -> PartialVMError {
        PartialVMError::new(status).at_code_offset(
            self.function_view
                .index()
                .unwrap_or(FunctionDefinitionIndex(0)),
            offset,
        )
    }

    fn push(&mut self, meter: &mut impl Meter, ty: SignatureToken) -> PartialVMResult<()> {
        self.charge_ty(meter, &ty)?;
        self.stack.push(ty);
        Ok(())
    }

    fn charge_ty(&mut self, meter: &mut impl Meter, ty: &SignatureToken) -> PartialVMResult<()> {
        meter.add_items(
            Scope::Function,
            TYPE_NODE_COST,
            ty.preorder_traversal().count(),
        )
    }

    fn charge_tys(
        &mut self,
        meter: &mut impl Meter,
        tys: &[SignatureToken],
    ) -> PartialVMResult<()> {
        for ty in tys {
            self.charge_ty(meter, ty)?
        }
        Ok(())
    }
}

pub(crate) fn verify<'a>(
    resolver: &'a BinaryIndexedView<'a>,
    function_view: &'a FunctionView<'a>,
    meter: &mut impl Meter, // currently unused
) -> PartialVMResult<()> {
    let verifier = &mut TypeSafetyChecker::new(resolver, function_view);

    for block_id in function_view.cfg().blocks() {
        for offset in function_view.cfg().instr_indexes(block_id) {
            let instr = &verifier.function_view.code().code[offset as usize];
            verify_instr(verifier, instr, offset, meter)?
        }
    }

    Ok(())
}

// helper for both `ImmBorrowField` and `MutBorrowField`
fn borrow_field(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    mut_: bool,
    field_handle_index: FieldOrVariantIndex,
    type_args: &Signature,
) -> PartialVMResult<()> {
    // load operand and check mutability constraints
    let operand = safe_unwrap!(verifier.stack.pop());
    if mut_ && !operand.is_mutable_reference() {
        return Err(verifier.error(StatusCode::BORROWFIELD_TYPE_MISMATCH_ERROR, offset));
    }

    // check the reference on the stack is the expected type.
    // Load the type that owns the field according to the instruction.
    // For generic fields access, this step materializes that type
    let (struct_def_index, variants, field_idx) = match field_handle_index {
        FieldOrVariantIndex::FieldIndex(idx) => {
            let field_handle = verifier.resolver.field_handle_at(idx)?;
            (field_handle.owner, None, field_handle.field as usize)
        },
        FieldOrVariantIndex::VariantFieldIndex(idx) => {
            let field_handle = verifier.resolver.variant_field_handle_at(idx)?;
            (
                field_handle.struct_index,
                Some(field_handle.variants.clone()),
                field_handle.field as usize,
            )
        },
    };
    let struct_def = verifier.resolver.struct_def_at(struct_def_index)?;
    let expected_type = materialize_type(struct_def.struct_handle, type_args);
    match operand {
        // For inner types use equality
        ST::Reference(inner) | ST::MutableReference(inner) if expected_type == *inner => (),
        _ => return Err(verifier.error(StatusCode::BORROWFIELD_TYPE_MISMATCH_ERROR, offset)),
    }

    // Check and determine the type loaded onto the stack
    let field_ty = if let Some(variants) = variants {
        if variants.is_empty() {
            // It is not allowed to have no variants provided here, otherwise we cannot
            // determine the type.
            return Err(verifier.error(StatusCode::ZERO_VARIANTS_ERROR, offset));
        }
        // For all provided variants, the field type must be the same.
        let mut field_ty = None;
        for variant in variants {
            if let Some(field_def) = struct_def
                .field_information
                .fields(Some(variant))
                .get(field_idx)
            {
                let ty = instantiate(&field_def.signature.0, type_args);
                if let Some(field_ty) = &field_ty {
                    // More than one field possible, compare types. Notice these types
                    // must be equal, not just assignable.
                    if &ty != field_ty {
                        return Err(
                            verifier.error(StatusCode::BORROWFIELD_TYPE_MISMATCH_ERROR, offset)
                        );
                    }
                } else {
                    field_ty = Some(ty)
                }
            } else {
                // If the struct variant has no field at this idx, this is an error
                return Err(verifier.error(StatusCode::BORROWFIELD_BAD_FIELD_ERROR, offset));
            }
        }
        field_ty
    } else {
        struct_def
            .field_information
            .fields(None)
            .get(field_idx)
            .map(|field_def| instantiate(&field_def.signature.0, type_args))
    };
    if let Some(field_ty) = field_ty {
        verifier.push(
            meter,
            if mut_ {
                ST::MutableReference(Box::new(field_ty))
            } else {
                ST::Reference(Box::new(field_ty))
            },
        )?;
    } else {
        // If the field is not defined, we are reporting an error in `instruction_consistency`.
        // Here push a dummy type to keep the abstract stack happy
        verifier.push(meter, ST::Bool)?;
    }
    Ok(())
}

// helper for both `ImmBorrowLoc` and `MutBorrowLoc`
fn borrow_loc(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    mut_: bool,
    idx: LocalIndex,
) -> PartialVMResult<()> {
    let loc_signature = verifier.local_at(idx).clone();

    if loc_signature.is_reference() {
        return Err(verifier.error(StatusCode::BORROWLOC_REFERENCE_ERROR, offset));
    }

    verifier.push(
        meter,
        if mut_ {
            ST::MutableReference(Box::new(loc_signature))
        } else {
            ST::Reference(Box::new(loc_signature))
        },
    )?;
    Ok(())
}

fn borrow_global(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    mut_: bool,
    idx: StructDefinitionIndex,
    type_args: &Signature,
) -> PartialVMResult<()> {
    // check and consume top of stack
    let operand = safe_unwrap!(verifier.stack.pop());
    if operand != ST::Address {
        return Err(verifier.error(StatusCode::BORROWGLOBAL_TYPE_MISMATCH_ERROR, offset));
    }

    let struct_def = verifier.resolver.struct_def_at(idx)?;
    let struct_type = materialize_type(struct_def.struct_handle, type_args);
    if !verifier.abilities(&struct_type)?.has_key() {
        return Err(verifier.error(StatusCode::BORROWGLOBAL_WITHOUT_KEY_ABILITY, offset));
    }

    let struct_type = materialize_type(struct_def.struct_handle, type_args);
    verifier.push(
        meter,
        if mut_ {
            ST::MutableReference(Box::new(struct_type))
        } else {
            ST::Reference(Box::new(struct_type))
        },
    )?;
    Ok(())
}

fn call(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    function_handle: &FunctionHandle,
    type_actuals: &Signature,
) -> PartialVMResult<()> {
    let parameters = verifier.resolver.signature_at(function_handle.parameters);
    for parameter in parameters.0.iter().rev() {
        let arg = safe_unwrap!(verifier.stack.pop());
        // For parameter to argument, use assignability
        if (type_actuals.is_empty() && !parameter.is_assignable_from(&arg))
            || (!type_actuals.is_empty()
                && !instantiate(parameter, type_actuals).is_assignable_from(&arg))
        {
            return Err(verifier.error(StatusCode::CALL_TYPE_MISMATCH_ERROR, offset));
        }
    }
    for return_type in &verifier.resolver.signature_at(function_handle.return_).0 {
        verifier.push(meter, instantiate(return_type, type_actuals))?
    }
    Ok(())
}

fn call_closure(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    expected_ty: &SignatureToken,
) -> PartialVMResult<()> {
    let SignatureToken::Function(param_tys, ret_tys, _) = expected_ty else {
        // The signature checker has ensured this is a function
        safe_assert!(false);
        unreachable!()
    };
    // On top of the stack is the closure, pop it.
    let closure_ty = safe_unwrap!(verifier.stack.pop());
    // Verify that the closure type is assignable to the expected type
    if !expected_ty.is_assignable_from(&closure_ty) {
        return Err(verifier
            .error(StatusCode::CALL_TYPE_MISMATCH_ERROR, offset)
            .with_message("closure type mismatch".to_owned()));
    }
    // Pop and verify arguments
    for param_ty in param_tys.iter().rev() {
        let arg_ty = safe_unwrap!(verifier.stack.pop());
        // For parameter to argument, use assignability
        if !param_ty.is_assignable_from(&arg_ty) {
            return Err(verifier.error(StatusCode::CALL_TYPE_MISMATCH_ERROR, offset));
        }
    }
    for ret_ty in ret_tys {
        verifier.push(meter, ret_ty.clone())?
    }
    Ok(())
}

fn clos_pack(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    func_handle_idx: FunctionHandleIndex,
    type_actuals: &Signature,
    mask: ClosureMask,
) -> PartialVMResult<()> {
    let func_handle = verifier.resolver.function_handle_at(func_handle_idx);
    // In order to determine whether this closure is storable, we need to figure whether
    // this function is marked as Persistent. This is case for
    // functions which are defined as public or which have this attribute explicit in the
    // source.
    let mut abilities = if func_handle
        .attributes
        .contains(&FunctionAttribute::Persistent)
    {
        AbilitySet::PUBLIC_FUNCTIONS
    } else {
        AbilitySet::PRIVATE_FUNCTIONS
    };
    // Check the captured arguments on the stack
    let param_sgn = verifier.resolver.signature_at(func_handle.parameters);
    // Instruction consistency check has verified that the number of captured arguments
    // is less than or equal to the number of parameters of the function.
    let captured_param_tys = mask.extract(&param_sgn.0, true);
    for ty in captured_param_tys.into_iter().rev() {
        let arg = safe_unwrap!(verifier.stack.pop());
        abilities = abilities.intersect(verifier.abilities(&arg)?);
        // For captured param type to argument, use assignability
        if (type_actuals.is_empty() && !ty.is_assignable_from(&arg))
            || (!type_actuals.is_empty() && !instantiate(ty, type_actuals).is_assignable_from(&arg))
        {
            return Err(verifier
                .error(StatusCode::PACK_TYPE_MISMATCH_ERROR, offset)
                .with_message("captured argument type mismatch".to_owned()));
        }
        // A captured argument must not be a reference
        if ty.is_reference() {
            return Err(verifier
                .error(StatusCode::PACK_TYPE_MISMATCH_ERROR, offset)
                .with_message("captured argument must not be a reference".to_owned()));
        }
    }

    // Construct the resulting function type
    let not_captured_param_tys = mask
        .extract(&param_sgn.0, false)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let ret_sign = verifier.resolver.signature_at(func_handle.return_);
    verifier.push(
        meter,
        instantiate(
            &SignatureToken::Function(not_captured_param_tys, ret_sign.0.to_vec(), abilities),
            type_actuals,
        ),
    )
}

fn type_fields_signature(
    verifier: &mut TypeSafetyChecker,
    _meter: &mut impl Meter, // TODO: metering
    offset: CodeOffset,
    struct_def: &StructDefinition,
    variant: Option<VariantIndex>,
    type_args: &Signature,
) -> PartialVMResult<Signature> {
    match (&struct_def.field_information, variant) {
        (StructFieldInformation::Declared(fields), None) => Ok(Signature(
            fields
                .iter()
                .map(|field_def| instantiate(&field_def.signature.0, type_args))
                .collect(),
        )),
        (StructFieldInformation::DeclaredVariants(variants), Some(variant))
            if (variant as usize) < variants.len() =>
        {
            Ok(Signature(
                variants[variant as usize]
                    .fields
                    .iter()
                    .map(|field_def| instantiate(&field_def.signature.0, type_args))
                    .collect(),
            ))
        },
        _ => {
            // TODO: this is more of "unreachable"
            Err(verifier.error(StatusCode::PACK_TYPE_MISMATCH_ERROR, offset))
        },
    }
}

fn pack(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    struct_def: &StructDefinition,
    variant: Option<VariantIndex>,
    type_args: &Signature,
) -> PartialVMResult<()> {
    let struct_type = materialize_type(struct_def.struct_handle, type_args);
    let field_sig = type_fields_signature(verifier, meter, offset, struct_def, variant, type_args)?;
    for sig in field_sig.0.iter().rev() {
        let arg = safe_unwrap!(verifier.stack.pop());
        // For field signature to argument, use assignability
        if !sig.is_assignable_from(&arg) {
            return Err(verifier.error(StatusCode::PACK_TYPE_MISMATCH_ERROR, offset));
        }
    }

    verifier.push(meter, struct_type)?;
    Ok(())
}

fn unpack(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    struct_def: &StructDefinition,
    variant: Option<VariantIndex>,
    type_args: &Signature,
) -> PartialVMResult<()> {
    let struct_type = materialize_type(struct_def.struct_handle, type_args);

    // Pop an abstract value from the stack and check if its type is equal to the one
    // declared.
    let arg = safe_unwrap!(verifier.stack.pop());
    if arg != struct_type {
        return Err(verifier.error(StatusCode::UNPACK_TYPE_MISMATCH_ERROR, offset));
    }

    let field_sig = type_fields_signature(verifier, meter, offset, struct_def, variant, type_args)?;
    for sig in field_sig.0 {
        verifier.push(meter, sig)?
    }
    Ok(())
}

fn test_variant(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    struct_def: &StructDefinition,
    type_args: &Signature,
) -> PartialVMResult<()> {
    let struct_type = materialize_type(struct_def.struct_handle, type_args);
    let arg = safe_unwrap!(verifier.stack.pop());
    match arg {
        // For inner type, use equality
        ST::Reference(inner) | ST::MutableReference(inner) if struct_type == *inner => (),
        _ => return Err(verifier.error(StatusCode::TEST_VARIANT_TYPE_MISMATCH_ERROR, offset)),
    }
    verifier.push(meter, ST::Bool)
}

fn exists(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    struct_def: &StructDefinition,
    type_args: &Signature,
) -> PartialVMResult<()> {
    let struct_type = materialize_type(struct_def.struct_handle, type_args);
    if !verifier.abilities(&struct_type)?.has_key() {
        return Err(verifier.error(
            StatusCode::EXISTS_WITHOUT_KEY_ABILITY_OR_BAD_ARGUMENT,
            offset,
        ));
    }

    let operand = safe_unwrap!(verifier.stack.pop());
    if operand != ST::Address {
        // TODO better error here
        return Err(verifier.error(
            StatusCode::EXISTS_WITHOUT_KEY_ABILITY_OR_BAD_ARGUMENT,
            offset,
        ));
    }

    verifier.push(meter, ST::Bool)?;
    Ok(())
}

fn move_from(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    offset: CodeOffset,
    struct_def: &StructDefinition,
    type_args: &Signature,
) -> PartialVMResult<()> {
    let struct_type = materialize_type(struct_def.struct_handle, type_args);
    if !verifier.abilities(&struct_type)?.has_key() {
        return Err(verifier.error(StatusCode::MOVEFROM_WITHOUT_KEY_ABILITY, offset));
    }

    let struct_type = materialize_type(struct_def.struct_handle, type_args);
    let operand = safe_unwrap!(verifier.stack.pop());
    if operand != ST::Address {
        return Err(verifier.error(StatusCode::MOVEFROM_TYPE_MISMATCH_ERROR, offset));
    }

    verifier.push(meter, struct_type)?;
    Ok(())
}

fn move_to(
    verifier: &mut TypeSafetyChecker,
    offset: CodeOffset,
    struct_def: &StructDefinition,
    type_args: &Signature,
) -> PartialVMResult<()> {
    let struct_type = materialize_type(struct_def.struct_handle, type_args);
    if !verifier.abilities(&struct_type)?.has_key() {
        return Err(verifier.error(StatusCode::MOVETO_WITHOUT_KEY_ABILITY, offset));
    }

    let struct_type = materialize_type(struct_def.struct_handle, type_args);
    let key_struct_operand = safe_unwrap!(verifier.stack.pop());
    let signer_reference_operand = safe_unwrap!(verifier.stack.pop());
    if key_struct_operand != struct_type {
        return Err(verifier.error(StatusCode::MOVETO_TYPE_MISMATCH_ERROR, offset));
    }
    match signer_reference_operand {
        ST::Reference(inner) => match *inner {
            ST::Signer => Ok(()),
            _ => Err(verifier.error(StatusCode::MOVETO_TYPE_MISMATCH_ERROR, offset)),
        },
        _ => Err(verifier.error(StatusCode::MOVETO_TYPE_MISMATCH_ERROR, offset)),
    }
}

fn borrow_vector_element(
    verifier: &mut TypeSafetyChecker,
    meter: &mut impl Meter,
    declared_element_type: &SignatureToken,
    offset: CodeOffset,
    mut_ref_only: bool,
) -> PartialVMResult<()> {
    let operand_idx = safe_unwrap!(verifier.stack.pop());
    let operand_vec = safe_unwrap!(verifier.stack.pop());

    // check index
    if operand_idx != ST::U64 {
        panic!("type_mismatch 10"); // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset));
    }

    // check vector and update stack
    // The declared element type must be exactly the same as the element type of the vector
    // operand. (No co-variance.)
    let element_type = match get_vector_element_type(operand_vec, mut_ref_only) {
        Some(ty) if declared_element_type == &ty => ty,
        _ => panic!("type_mismatch 8"), // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset)),
    };
    let element_ref_type = if mut_ref_only {
        ST::MutableReference(Box::new(element_type))
    } else {
        ST::Reference(Box::new(element_type))
    };
    verifier.push(meter, element_ref_type)?;

    Ok(())
}

fn verify_instr(
    verifier: &mut TypeSafetyChecker,
    bytecode: &Bytecode,
    offset: CodeOffset,
    meter: &mut impl Meter,
) -> PartialVMResult<()> {
    match bytecode {
        Bytecode::Pop => {
            let operand = safe_unwrap!(verifier.stack.pop());
            let abilities = verifier
                .resolver
                .abilities(&operand, verifier.function_view.type_parameters());
            if !abilities?.has_drop() {
                return Err(verifier.error(StatusCode::POP_WITHOUT_DROP_ABILITY, offset));
            }
        },

        Bytecode::BrTrue(_) | Bytecode::BrFalse(_) => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if operand != ST::Bool {
                return Err(verifier.error(StatusCode::BR_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::StLoc(idx) => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if !verifier.local_at(*idx).is_assignable_from(&operand) {
                return Err(verifier.error(StatusCode::STLOC_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::Abort => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if operand != ST::U64 {
                return Err(verifier.error(StatusCode::ABORT_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::Ret => {
            let return_ = &verifier.function_view.return_().0;
            for return_type in return_.iter().rev() {
                let operand = safe_unwrap!(verifier.stack.pop());
                // The return type must be assignable from the returned value.
                if !return_type.is_assignable_from(&operand) {
                    return Err(verifier.error(StatusCode::RET_TYPE_MISMATCH_ERROR, offset));
                }
            }
        },

        Bytecode::Branch(_) | Bytecode::Nop => (),

        Bytecode::FreezeRef => {
            let operand = safe_unwrap!(verifier.stack.pop());
            match operand {
                ST::MutableReference(inner) => verifier.push(meter, ST::Reference(inner))?,
                _ => return Err(verifier.error(StatusCode::FREEZEREF_TYPE_MISMATCH_ERROR, offset)),
            }
        },

        Bytecode::MutBorrowField(field_handle_index) => borrow_field(
            verifier,
            meter,
            offset,
            true,
            FieldOrVariantIndex::FieldIndex(*field_handle_index),
            &Signature(vec![]),
        )?,

        Bytecode::MutBorrowFieldGeneric(field_inst_index) => {
            let field_inst = verifier
                .resolver
                .field_instantiation_at(*field_inst_index)?;
            let type_inst = verifier.resolver.signature_at(field_inst.type_parameters);
            verifier.charge_tys(meter, &type_inst.0)?;
            borrow_field(
                verifier,
                meter,
                offset,
                true,
                FieldOrVariantIndex::FieldIndex(field_inst.handle),
                type_inst,
            )?
        },

        Bytecode::ImmBorrowField(field_handle_index) => borrow_field(
            verifier,
            meter,
            offset,
            false,
            FieldOrVariantIndex::FieldIndex(*field_handle_index),
            &Signature(vec![]),
        )?,

        Bytecode::ImmBorrowFieldGeneric(field_inst_index) => {
            let field_inst = verifier
                .resolver
                .field_instantiation_at(*field_inst_index)?;
            let type_inst = verifier.resolver.signature_at(field_inst.type_parameters);
            verifier.charge_tys(meter, &type_inst.0)?;
            borrow_field(
                verifier,
                meter,
                offset,
                false,
                FieldOrVariantIndex::FieldIndex(field_inst.handle),
                type_inst,
            )?
        },

        Bytecode::MutBorrowVariantField(field_handle_index) => borrow_field(
            verifier,
            meter,
            offset,
            true,
            FieldOrVariantIndex::VariantFieldIndex(*field_handle_index),
            &Signature(vec![]),
        )?,

        Bytecode::MutBorrowVariantFieldGeneric(field_inst_index) => {
            let field_inst = verifier
                .resolver
                .variant_field_instantiation_at(*field_inst_index)?;
            let type_inst = verifier.resolver.signature_at(field_inst.type_parameters);
            verifier.charge_tys(meter, &type_inst.0)?;
            borrow_field(
                verifier,
                meter,
                offset,
                true,
                FieldOrVariantIndex::VariantFieldIndex(field_inst.handle),
                type_inst,
            )?
        },

        Bytecode::ImmBorrowVariantField(field_handle_index) => borrow_field(
            verifier,
            meter,
            offset,
            false,
            FieldOrVariantIndex::VariantFieldIndex(*field_handle_index),
            &Signature(vec![]),
        )?,

        Bytecode::ImmBorrowVariantFieldGeneric(field_inst_index) => {
            let field_inst = verifier
                .resolver
                .variant_field_instantiation_at(*field_inst_index)?;
            let type_inst = verifier.resolver.signature_at(field_inst.type_parameters);
            verifier.charge_tys(meter, &type_inst.0)?;
            borrow_field(
                verifier,
                meter,
                offset,
                false,
                FieldOrVariantIndex::VariantFieldIndex(field_inst.handle),
                type_inst,
            )?
        },

        Bytecode::LdU8(_) => {
            verifier.push(meter, ST::U8)?;
        },

        Bytecode::LdU16(_) => {
            verifier.push(meter, ST::U16)?;
        },

        Bytecode::LdU32(_) => {
            verifier.push(meter, ST::U32)?;
        },

        Bytecode::LdU64(_) => {
            verifier.push(meter, ST::U64)?;
        },

        Bytecode::LdU128(_) => {
            verifier.push(meter, ST::U128)?;
        },

        Bytecode::LdU256(_) => {
            verifier.push(meter, ST::U256)?;
        },

        Bytecode::LdConst(idx) => {
            let signature = verifier.resolver.constant_at(*idx).type_.clone();
            verifier.push(meter, signature)?;
        },

        Bytecode::LdTrue | Bytecode::LdFalse => {
            verifier.push(meter, ST::Bool)?;
        },

        Bytecode::CopyLoc(idx) => {
            let local_signature = verifier.local_at(*idx).clone();
            if !verifier
                .resolver
                .abilities(&local_signature, verifier.function_view.type_parameters())?
                .has_copy()
            {
                return Err(verifier.error(StatusCode::COPYLOC_WITHOUT_COPY_ABILITY, offset));
            }
            verifier.push(meter, local_signature)?
        },

        Bytecode::MoveLoc(idx) => {
            let local_signature = verifier.local_at(*idx).clone();
            verifier.push(meter, local_signature)?
        },

        Bytecode::MutBorrowLoc(idx) => borrow_loc(verifier, meter, offset, true, *idx)?,

        Bytecode::ImmBorrowLoc(idx) => borrow_loc(verifier, meter, offset, false, *idx)?,

        Bytecode::Call(idx) => {
            let function_handle = verifier.resolver.function_handle_at(*idx);
            call(verifier, meter, offset, function_handle, &Signature(vec![]))?
        },

        Bytecode::CallGeneric(idx) => {
            let func_inst = verifier.resolver.function_instantiation_at(*idx);
            let func_handle = verifier.resolver.function_handle_at(func_inst.handle);
            let type_args = &verifier.resolver.signature_at(func_inst.type_parameters);
            verifier.charge_tys(meter, &type_args.0)?;
            call(verifier, meter, offset, func_handle, type_args)?
        },

        Bytecode::PackClosure(idx, mask) => {
            clos_pack(verifier, meter, offset, *idx, &Signature(vec![]), *mask)?
        },
        Bytecode::PackClosureGeneric(idx, mask) => {
            let func_inst = verifier.resolver.function_instantiation_at(*idx);
            let type_args = &verifier.resolver.signature_at(func_inst.type_parameters);
            verifier.charge_tys(meter, &type_args.0)?;
            clos_pack(verifier, meter, offset, func_inst.handle, type_args, *mask)?
        },
        Bytecode::CallClosure(idx) => {
            // The signature checker has verified this is a function type.
            let expected_ty = safe_unwrap!(verifier.resolver.signature_at(*idx).0.first());
            call_closure(verifier, meter, offset, expected_ty)?
        },

        Bytecode::Pack(idx) => {
            let struct_definition = verifier.resolver.struct_def_at(*idx)?;
            pack(
                verifier,
                meter,
                offset,
                struct_definition,
                None,
                &Signature(vec![]),
            )?
        },
        Bytecode::PackGeneric(idx) => {
            let struct_inst = verifier.resolver.struct_instantiation_at(*idx)?;
            let struct_def = verifier.resolver.struct_def_at(struct_inst.def)?;
            let type_args = verifier.resolver.signature_at(struct_inst.type_parameters);
            verifier.charge_tys(meter, &type_args.0)?;
            pack(verifier, meter, offset, struct_def, None, type_args)?
        },
        Bytecode::Unpack(idx) => {
            let struct_definition = verifier.resolver.struct_def_at(*idx)?;
            unpack(
                verifier,
                meter,
                offset,
                struct_definition,
                None,
                &Signature(vec![]),
            )?
        },
        Bytecode::UnpackGeneric(idx) => {
            let struct_inst = verifier.resolver.struct_instantiation_at(*idx)?;
            let struct_def = verifier.resolver.struct_def_at(struct_inst.def)?;
            let type_args = verifier.resolver.signature_at(struct_inst.type_parameters);
            verifier.charge_tys(meter, &type_args.0)?;
            unpack(verifier, meter, offset, struct_def, None, type_args)?
        },

        Bytecode::PackVariant(idx) => {
            let handle = verifier.resolver.struct_variant_handle_at(*idx)?;
            let struct_definition = verifier.resolver.struct_def_at(handle.struct_index)?;
            pack(
                verifier,
                meter,
                offset,
                struct_definition,
                Some(handle.variant),
                &Signature(vec![]),
            )?
        },
        Bytecode::PackVariantGeneric(idx) => {
            let inst = verifier.resolver.struct_variant_instantiation_at(*idx)?;
            let handle = verifier.resolver.struct_variant_handle_at(inst.handle)?;
            let struct_def = verifier.resolver.struct_def_at(handle.struct_index)?;
            let type_args = verifier.resolver.signature_at(inst.type_parameters);
            verifier.charge_tys(meter, &type_args.0)?;
            pack(
                verifier,
                meter,
                offset,
                struct_def,
                Some(handle.variant),
                type_args,
            )?
        },
        Bytecode::UnpackVariant(idx) => {
            let handle = verifier.resolver.struct_variant_handle_at(*idx)?;
            let struct_definition = verifier.resolver.struct_def_at(handle.struct_index)?;
            unpack(
                verifier,
                meter,
                offset,
                struct_definition,
                Some(handle.variant),
                &Signature(vec![]),
            )?
        },
        Bytecode::UnpackVariantGeneric(idx) => {
            let inst = verifier.resolver.struct_variant_instantiation_at(*idx)?;
            let handle = verifier.resolver.struct_variant_handle_at(inst.handle)?;
            let struct_def = verifier.resolver.struct_def_at(handle.struct_index)?;
            let type_args = verifier.resolver.signature_at(inst.type_parameters);
            verifier.charge_tys(meter, &type_args.0)?;
            unpack(
                verifier,
                meter,
                offset,
                struct_def,
                Some(handle.variant),
                type_args,
            )?
        },

        Bytecode::TestVariant(idx) => {
            let handle = verifier.resolver.struct_variant_handle_at(*idx)?;
            let struct_def = verifier.resolver.struct_def_at(handle.struct_index)?;
            test_variant(verifier, meter, offset, struct_def, &Signature(vec![]))?
        },
        Bytecode::TestVariantGeneric(idx) => {
            let inst = verifier.resolver.struct_variant_instantiation_at(*idx)?;
            let handle = verifier.resolver.struct_variant_handle_at(inst.handle)?;
            let struct_def = verifier.resolver.struct_def_at(handle.struct_index)?;
            let type_args = verifier.resolver.signature_at(inst.type_parameters);
            test_variant(verifier, meter, offset, struct_def, type_args)?
        },

        Bytecode::ReadRef => {
            let operand = safe_unwrap!(verifier.stack.pop());
            match operand {
                ST::Reference(inner) | ST::MutableReference(inner) => {
                    if !verifier.abilities(&inner)?.has_copy() {
                        return Err(
                            verifier.error(StatusCode::READREF_WITHOUT_COPY_ABILITY, offset)
                        );
                    }
                    verifier.push(meter, *inner)?;
                },
                _ => return Err(verifier.error(StatusCode::READREF_TYPE_MISMATCH_ERROR, offset)),
            }
        },

        Bytecode::WriteRef => {
            let ref_operand = safe_unwrap!(verifier.stack.pop());
            let val_operand = safe_unwrap!(verifier.stack.pop());
            let ref_inner_signature = match ref_operand {
                ST::MutableReference(inner) => *inner,
                _ => {
                    return Err(
                        verifier.error(StatusCode::WRITEREF_NO_MUTABLE_REFERENCE_ERROR, offset)
                    )
                },
            };
            if !verifier.abilities(&ref_inner_signature)?.has_drop() {
                return Err(verifier.error(StatusCode::WRITEREF_WITHOUT_DROP_ABILITY, offset));
            }

            // The inner type of the reference must be assignable from the operand
            if !ref_inner_signature.is_assignable_from(&val_operand) {
                return Err(verifier.error(StatusCode::WRITEREF_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::CastU8 => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if !operand.is_integer() {
                return Err(verifier.error(StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR, offset));
            }
            verifier.push(meter, ST::U8)?;
        },
        Bytecode::CastU64 => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if !operand.is_integer() {
                return Err(verifier.error(StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR, offset));
            }
            verifier.push(meter, ST::U64)?;
        },
        Bytecode::CastU128 => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if !operand.is_integer() {
                return Err(verifier.error(StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR, offset));
            }
            verifier.push(meter, ST::U128)?;
        },

        Bytecode::Add
        | Bytecode::Sub
        | Bytecode::Mul
        | Bytecode::Mod
        | Bytecode::Div
        | Bytecode::BitOr
        | Bytecode::BitAnd
        | Bytecode::Xor => {
            let operand1 = safe_unwrap!(verifier.stack.pop());
            let operand2 = safe_unwrap!(verifier.stack.pop());
            if operand1.is_integer() && operand1 == operand2 {
                verifier.push(meter, operand1)?;
            } else {
                return Err(verifier.error(StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::Shl | Bytecode::Shr => {
            let operand1 = safe_unwrap!(verifier.stack.pop());
            let operand2 = safe_unwrap!(verifier.stack.pop());
            if operand2.is_integer() && operand1 == ST::U8 {
                verifier.push(meter, operand2)?;
            } else {
                return Err(verifier.error(StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::Or | Bytecode::And => {
            let operand1 = safe_unwrap!(verifier.stack.pop());
            let operand2 = safe_unwrap!(verifier.stack.pop());
            if operand1 == ST::Bool && operand2 == ST::Bool {
                verifier.push(meter, ST::Bool)?;
            } else {
                return Err(verifier.error(StatusCode::BOOLEAN_OP_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::Not => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if operand == ST::Bool {
                verifier.push(meter, ST::Bool)?;
            } else {
                return Err(verifier.error(StatusCode::BOOLEAN_OP_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::Eq | Bytecode::Neq => {
            let operand1 = safe_unwrap!(verifier.stack.pop());
            let operand2 = safe_unwrap!(verifier.stack.pop());
            if verifier.abilities(&operand1)?.has_drop() && operand1 == operand2 {
                verifier.push(meter, ST::Bool)?;
            } else {
                return Err(verifier.error(StatusCode::EQUALITY_OP_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::Lt | Bytecode::Gt | Bytecode::Le | Bytecode::Ge => {
            let operand1 = safe_unwrap!(verifier.stack.pop());
            let operand2 = safe_unwrap!(verifier.stack.pop());
            if operand1.is_integer() && operand1 == operand2 {
                verifier.push(meter, ST::Bool)?
            } else {
                return Err(verifier.error(StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR, offset));
            }
        },

        Bytecode::MutBorrowGlobal(idx) => {
            borrow_global(verifier, meter, offset, true, *idx, &Signature(vec![]))?
        },

        Bytecode::MutBorrowGlobalGeneric(idx) => {
            let struct_inst = verifier.resolver.struct_instantiation_at(*idx)?;
            let type_inst = verifier.resolver.signature_at(struct_inst.type_parameters);
            verifier.charge_tys(meter, &type_inst.0)?;
            borrow_global(verifier, meter, offset, true, struct_inst.def, type_inst)?
        },

        Bytecode::ImmBorrowGlobal(idx) => {
            borrow_global(verifier, meter, offset, false, *idx, &Signature(vec![]))?
        },

        Bytecode::ImmBorrowGlobalGeneric(idx) => {
            let struct_inst = verifier.resolver.struct_instantiation_at(*idx)?;
            let type_inst = verifier.resolver.signature_at(struct_inst.type_parameters);
            verifier.charge_tys(meter, &type_inst.0)?;
            borrow_global(verifier, meter, offset, false, struct_inst.def, type_inst)?
        },

        Bytecode::Exists(idx) => {
            let struct_def = verifier.resolver.struct_def_at(*idx)?;
            exists(verifier, meter, offset, struct_def, &Signature(vec![]))?
        },

        Bytecode::ExistsGeneric(idx) => {
            let struct_inst = verifier.resolver.struct_instantiation_at(*idx)?;
            let struct_def = verifier.resolver.struct_def_at(struct_inst.def)?;
            let type_args = verifier.resolver.signature_at(struct_inst.type_parameters);
            verifier.charge_tys(meter, &type_args.0)?;
            exists(verifier, meter, offset, struct_def, type_args)?
        },

        Bytecode::MoveFrom(idx) => {
            let struct_def = verifier.resolver.struct_def_at(*idx)?;
            move_from(verifier, meter, offset, struct_def, &Signature(vec![]))?
        },

        Bytecode::MoveFromGeneric(idx) => {
            let struct_inst = verifier.resolver.struct_instantiation_at(*idx)?;
            let struct_def = verifier.resolver.struct_def_at(struct_inst.def)?;
            let type_args = verifier.resolver.signature_at(struct_inst.type_parameters);
            verifier.charge_tys(meter, &type_args.0)?;
            move_from(verifier, meter, offset, struct_def, type_args)?
        },

        Bytecode::MoveTo(idx) => {
            let struct_def = verifier.resolver.struct_def_at(*idx)?;
            move_to(verifier, offset, struct_def, &Signature(vec![]))?
        },

        Bytecode::MoveToGeneric(idx) => {
            let struct_inst = verifier.resolver.struct_instantiation_at(*idx)?;
            let struct_def = verifier.resolver.struct_def_at(struct_inst.def)?;
            let type_args = verifier.resolver.signature_at(struct_inst.type_parameters);
            verifier.charge_tys(meter, &type_args.0)?;
            move_to(verifier, offset, struct_def, type_args)?
        },

        Bytecode::VecPack(idx, num) => {
            let element_type = &verifier.resolver.signature_at(*idx).0[0];
            for _ in 0..*num {
                let operand_type = safe_unwrap!(verifier.stack.pop());
                // The operand type must be assignable to the element type.
                if !element_type.is_assignable_from(&operand_type) {
                    panic!("type_mismatch 9"); // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset));
                }
            }
            verifier
                .stack
                .push(ST::Vector(Box::new(element_type.clone())));
        },

        Bytecode::VecLen(idx) => {
            let operand = safe_unwrap!(verifier.stack.pop());
            let declared_element_type = &verifier.resolver.signature_at(*idx).0[0];
            match get_vector_element_type(operand, false) {
                // The derived and declared element types must be equal (no co-variance)
                Some(derived_element_type) if &derived_element_type == declared_element_type => {
                    verifier.push(meter, ST::U64)?;
                },
                _ => panic!("type_mismatch 7"), // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset)),
            };
        },

        Bytecode::VecImmBorrow(idx) => {
            let declared_element_type = &verifier.resolver.signature_at(*idx).0[0];
            borrow_vector_element(verifier, meter, declared_element_type, offset, false)?
        },
        Bytecode::VecMutBorrow(idx) => {
            let declared_element_type = &verifier.resolver.signature_at(*idx).0[0];
            borrow_vector_element(verifier, meter, declared_element_type, offset, true)?
        },

        Bytecode::VecPushBack(idx) => {
            let operand_elem = safe_unwrap!(verifier.stack.pop());
            let operand_vec = safe_unwrap!(verifier.stack.pop());
            let declared_element_type = &verifier.resolver.signature_at(*idx).0[0];
            // The operand type must be assignable to the declared element type.
            if !declared_element_type.is_assignable_from(&operand_elem) {
                panic!("type_mismatch 6"); // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset));
            }
            match get_vector_element_type(operand_vec, true) {
                // Derived and declared element types must be equal.
                Some(derived_element_type) if &derived_element_type == declared_element_type => {},
                _ => panic!("type_mismatch 5"), // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset)),
            };
        },

        Bytecode::VecPopBack(idx) => {
            let operand_vec = safe_unwrap!(verifier.stack.pop());
            let declared_element_type = &verifier.resolver.signature_at(*idx).0[0];
            match get_vector_element_type(operand_vec, true) {
                // Derived and declared element types must be equal.
                Some(derived_element_type) if &derived_element_type == declared_element_type => {
                    verifier.push(meter, derived_element_type)?;
                },
                _ => panic!("type_mismatch 4"), // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset)),
            };
        },

        Bytecode::VecUnpack(idx, num) => {
            let operand_vec = safe_unwrap!(verifier.stack.pop());
            let declared_element_type = &verifier.resolver.signature_at(*idx).0[0];
            if operand_vec != ST::Vector(Box::new(declared_element_type.clone())) {
                panic!("type_mismatch 3"); // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset));
            }
            for _ in 0..*num {
                verifier.push(meter, declared_element_type.clone())?;
            }
        },

        Bytecode::VecSwap(idx) => {
            let operand_idx2 = safe_unwrap!(verifier.stack.pop());
            let operand_idx1 = safe_unwrap!(verifier.stack.pop());
            let operand_vec = safe_unwrap!(verifier.stack.pop());
            if operand_idx1 != ST::U64 || operand_idx2 != ST::U64 {
                panic!("type_mismatch 2"); // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset));
            }
            let declared_element_type = &verifier.resolver.signature_at(*idx).0[0];
            match get_vector_element_type(operand_vec, true) {
                // Derived and declared element types must be equal
                Some(derived_element_type) if &derived_element_type == declared_element_type => {},
                _ => panic!("type_mismatch 1"), // return Err(verifier.error(StatusCode::TYPE_MISMATCH, offset)),
            };
        },
        Bytecode::CastU16 => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if !operand.is_integer() {
                return Err(verifier.error(StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR, offset));
            }
            verifier.push(meter, ST::U16)?;
        },
        Bytecode::CastU32 => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if !operand.is_integer() {
                return Err(verifier.error(StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR, offset));
            }
            verifier.push(meter, ST::U32)?;
        },
        Bytecode::CastU256 => {
            let operand = safe_unwrap!(verifier.stack.pop());
            if !operand.is_integer() {
                return Err(verifier.error(StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR, offset));
            }
            verifier.push(meter, ST::U256)?;
        },
    };
    Ok(())
}

//
// Helpers functions for types
//

fn materialize_type(struct_handle: StructHandleIndex, type_args: &Signature) -> SignatureToken {
    if type_args.is_empty() {
        ST::Struct(struct_handle)
    } else {
        ST::StructInstantiation(struct_handle, type_args.0.clone())
    }
}

fn instantiate(token: &SignatureToken, subst: &Signature) -> SignatureToken {
    use SignatureToken::*;

    if subst.0.is_empty() {
        return token.clone();
    }

    let inst_vec = |v: &[SignatureToken]| -> Vec<SignatureToken> {
        v.iter().map(|ty| instantiate(ty, subst)).collect()
    };
    match token {
        Bool => Bool,
        U8 => U8,
        U16 => U16,
        U32 => U32,
        U64 => U64,
        U128 => U128,
        U256 => U256,
        Address => Address,
        Signer => Signer,
        Vector(ty) => Vector(Box::new(instantiate(ty, subst))),
        Function(args, result, abilities) => Function(inst_vec(args), inst_vec(result), *abilities),
        Struct(idx) => Struct(*idx),
        StructInstantiation(idx, struct_type_args) => {
            StructInstantiation(*idx, inst_vec(struct_type_args))
        },
        Reference(ty) => Reference(Box::new(instantiate(ty, subst))),
        MutableReference(ty) => MutableReference(Box::new(instantiate(ty, subst))),
        TypeParameter(idx) => {
            // Assume that the caller has previously parsed and verified the structure of the
            // file and that this guarantees that type parameter indices are always in bounds.
            debug_assert!((*idx as usize) < subst.len());
            subst.0[*idx as usize].clone()
        },
    }
}

fn get_vector_element_type(
    vector_ref_ty: SignatureToken,
    mut_ref_only: bool,
) -> Option<SignatureToken> {
    use SignatureToken::*;
    match vector_ref_ty {
        Reference(referred_type) => {
            if mut_ref_only {
                None
            } else if let ST::Vector(element_type) = *referred_type {
                Some(*element_type)
            } else {
                None
            }
        },
        MutableReference(referred_type) => {
            if let ST::Vector(element_type) = *referred_type {
                Some(*element_type)
            } else {
                None
            }
        },
        _ => None,
    }
}
