// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checking a payload's BCS arguments for the values the framework's argument
//! constructors would refuse, before any argument is placed.

use crate::errors::{invariant_violation, InvalidArguments, MoveExecutionFailure};
use mono_move_core::{
    intern_type_tag,
    interner::{view_module_id, InternedIdentifier, InternedModuleId},
    types::{view_name, view_type, view_type_list, InternedType, InternedTypeList, Type},
    VMInternalError,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{read_slice, read_uleb128_len, InterpreterContext, RuntimeError};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::{StructTag, TypeTag},
};

/// The most object existence checks one argument may make.
const MAX_OBJECT_CHECKS_PER_ARG: usize = 32;

/// Checks that each argument holds a value the framework's argument
/// constructors would accept, and nothing more.
/// - A `String` is valid UTF-8.
/// - An `Option` holds at most one value.
/// - An `Object<T>` names an address holding an `ObjectCore` and a `T`.
///
/// `param_tys` are the non-signer parameter types, one per argument.
pub(super) fn check_arg_values<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    param_tys: &[InternedType],
    args: &[Vec<u8>],
) -> Result<(), MoveExecutionFailure> {
    let object_core = object_core_type(guard)?;
    for (&ty, arg) in param_tys.iter().zip(args) {
        let mut walk = ArgWalk {
            interp: &mut *interp,
            bytes: arg,
            cursor: 0,
            object_checks: 0,
            object_core,
        };
        walk.check(ty)?;
        if walk.cursor != arg.len() {
            return Err(undecodable());
        }
    }
    Ok(())
}

/// The interned `0x1::object::ObjectCore` type.
fn object_core_type(guard: &ExecutionGuard<'_>) -> Result<InternedType, MoveExecutionFailure> {
    let tag = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("object").to_owned(),
        name: ident_str!("ObjectCore").to_owned(),
        type_args: vec![],
    }));
    intern_type_tag(&tag, guard)
        .map_err(|err| MoveExecutionFailure::RuntimeError(invariant_violation(err.to_string())))
}

/// The walk over one argument's bytes.
struct ArgWalk<'w, 'g> {
    interp: &'w mut InterpreterContext<'g>,
    bytes: &'w [u8],
    cursor: usize,
    object_checks: usize,
    object_core: InternedType,
}

impl<'w> ArgWalk<'w, '_> {
    /// Walks one value of type `ty`, checking what it holds.
    fn check(&mut self, ty: InternedType) -> Result<(), MoveExecutionFailure> {
        match view_type(ty) {
            Type::Bool | Type::U8 | Type::I8 => self.skip(1),
            Type::U16 | Type::I16 => self.skip(2),
            Type::U32 | Type::I32 => self.skip(4),
            Type::U64 | Type::I64 => self.skip(8),
            Type::U128 | Type::I128 => self.skip(16),
            Type::U256 | Type::I256 | Type::Address => self.skip(32),
            Type::Vector { elem } => self.check_vector(*elem),
            Type::Nominal {
                module_id,
                name,
                ty_args,
            } => self.check_framework_struct(*module_id, *name, *ty_args),
            Type::Signer
            | Type::ImmutRef { .. }
            | Type::MutRef { .. }
            | Type::Function { .. }
            | Type::TypeParam { .. } => Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::DisallowedParameterType,
            )),
        }
    }

    fn check_vector(&mut self, elem: InternedType) -> Result<(), MoveExecutionFailure> {
        let len = self.read_len()?;
        if let Some(size) = primitive_size(elem) {
            let bytes = len.checked_mul(size).ok_or_else(undecodable)?;
            return self.skip(bytes);
        }
        for _ in 0..len {
            self.check(elem)?;
        }
        Ok(())
    }

    fn check_framework_struct(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> Result<(), MoveExecutionFailure> {
        let module_id = view_module_id(module_id);
        let names = (*module_id.address() == AccountAddress::ONE)
            .then(|| (view_name(module_id.name()), view_name(name)));
        match names {
            Some(("string", "String")) => {
                let len = self.read_len()?;
                if std::str::from_utf8(self.read(len)?).is_err() {
                    return Err(MoveExecutionFailure::InvalidArguments(
                        InvalidArguments::MalformedString,
                    ));
                }
                Ok(())
            },
            Some(("option", "Option")) => match self.read_len()? {
                0 => Ok(()),
                1 => self.check(type_arg(ty_args)?),
                _ => Err(undecodable()),
            },
            Some(("object", "Object")) => {
                let address = AccountAddress::from_bytes(self.read(AccountAddress::LENGTH)?)
                    .map_err(|err| {
                        MoveExecutionFailure::RuntimeError(invariant_violation(err.to_string()))
                    })?;
                self.check_object(address, type_arg(ty_args)?)
            },
            Some(("fixed_point32", "FixedPoint32")) => self.skip(8),
            Some(("fixed_point64", "FixedPoint64")) => self.skip(16),
            Some(_) | None => Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::DisallowedParameterType,
            )),
        }
    }

    /// Checks that `address` holds an object and a `resource`, like
    /// `object::address_to_object` does.
    fn check_object(
        &mut self,
        address: AccountAddress,
        resource: InternedType,
    ) -> Result<(), MoveExecutionFailure> {
        if self.object_checks == MAX_OBJECT_CHECKS_PER_ARG {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::TooManyObjectChecks,
            ));
        }
        self.object_checks += 1;
        // TODO(metering): charge for the two reads.
        if !self
            .interp
            .resource_exists(address, self.object_core)
            .map_err(MoveExecutionFailure::RuntimeError)?
        {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::ObjectDoesNotExist,
            ));
        }
        if !self
            .interp
            .resource_exists(address, resource)
            .map_err(MoveExecutionFailure::RuntimeError)?
        {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::ObjectLacksResource,
            ));
        }
        Ok(())
    }

    /// Reads a BCS length or enum tag.
    fn read_len(&mut self) -> Result<usize, MoveExecutionFailure> {
        let len = read_uleb128_len(self.bytes, &mut self.cursor).map_err(decode_error)?;
        if len > bcs::MAX_SEQUENCE_LENGTH as u64 {
            return Err(undecodable());
        }
        Ok(len as usize)
    }

    fn read(&mut self, n: usize) -> Result<&'w [u8], MoveExecutionFailure> {
        read_slice(self.bytes, &mut self.cursor, n).map_err(decode_error)
    }

    fn skip(&mut self, n: usize) -> Result<(), MoveExecutionFailure> {
        self.read(n).map(|_| ())
    }
}

/// The one type argument of a framework type that takes one.
fn type_arg(ty_args: InternedTypeList) -> Result<InternedType, MoveExecutionFailure> {
    view_type_list(ty_args).first().copied().ok_or_else(|| {
        MoveExecutionFailure::RuntimeError(invariant_violation(
            "framework argument type without its type argument",
        ))
    })
}

/// The BCS size of a primitive type, or `None` for any other type.
fn primitive_size(ty: InternedType) -> Option<usize> {
    match view_type(ty) {
        Type::Bool | Type::U8 | Type::I8 => Some(1),
        Type::U16 | Type::I16 => Some(2),
        Type::U32 | Type::I32 => Some(4),
        Type::U64 | Type::I64 => Some(8),
        Type::U128 | Type::I128 => Some(16),
        Type::U256 | Type::I256 | Type::Address => Some(32),
        Type::Vector { .. }
        | Type::Nominal { .. }
        | Type::Signer
        | Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Function { .. }
        | Type::TypeParam { .. } => None,
    }
}

fn undecodable() -> MoveExecutionFailure {
    MoveExecutionFailure::InvalidArguments(InvalidArguments::UndecodableArgument)
}

/// Only a decode failure faults the argument's bytes; anything else is the VM's.
fn decode_error(err: RuntimeError) -> MoveExecutionFailure {
    if err.is_bcs_decode_error() {
        undecodable()
    } else {
        MoveExecutionFailure::RuntimeError(VMInternalError::new(err))
    }
}
