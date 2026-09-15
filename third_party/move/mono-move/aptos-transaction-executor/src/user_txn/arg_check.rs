// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checking a payload's parameter types and BCS arguments against what a
//! transaction may construct, before any argument is placed.

use super::args::check_arg_counts;
use crate::errors::{invariant_violation, InvalidArguments, MoveExecutionFailure};
use aptos_types::account_config::ObjectCoreResource;
use mono_move_core::{
    intern_type_tag,
    interner::{view_module_id, InternedIdentifier, InternedModuleId},
    types::{view_name, view_type, view_type_list, InternedType, InternedTypeList, Type},
    FieldTypes, Interner, PreparedModule, VMInternalError,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{read_slice, read_uleb128_len, InterpreterContext, RuntimeError};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{FunctionAttribute, StructFieldInformation, VariantIndex, Visibility},
};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{TypeTag, DOLLAR_SIGN_DELIMITER, PACK},
    move_resource::MoveStructType,
};

/// The most object existence checks one argument may make.
const MAX_OBJECT_CHECKS_PER_ARG: usize = 32;

/// Checks that a transaction may fill `param_tys` with its signers and `args`.
/// - Every non-signer parameter type is one an argument can fill: primitives,
///   vectors and `Option`s of such, `String`, `Object<T>`, `FixedPoint32`,
///   `FixedPoint64`, and public structs and enums whose fields are.
/// - The signer and argument counts match the parameters.
/// - Each argument holds a value the framework's constructors would accept: a
///   `String` is valid UTF-8, an `Option` holds at most one value, an
///   `Object<T>` names an address holding an `ObjectCore` and a `T`, and an
///   enum's tag names one of its variants.
pub(super) fn check_args<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    param_tys: &[InternedType],
    signer_params: usize,
    secondary_signers: &[AccountAddress],
    args: &[Vec<u8>],
) -> Result<(), MoveExecutionFailure> {
    let mut checker = ArgChecker {
        guard,
        interp,
        object_core: None,
    };
    let arg_tys = &param_tys[signer_params..];
    for &ty in arg_tys {
        if !checker.is_allowed(ty)? {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::DisallowedParameterType,
            ));
        }
    }
    check_arg_counts(param_tys, signer_params, secondary_signers, args)
        .map_err(MoveExecutionFailure::InvalidArguments)?;
    for (&ty, arg) in arg_tys.iter().zip(args) {
        let mut walk = Walk {
            bytes: arg,
            cursor: 0,
            object_checks: 0,
        };
        checker.walk(&mut walk, ty)?;
    }
    Ok(())
}

/// The framework types a transaction may construct.
enum Framework {
    String,
    Option,
    Object,
    FixedPoint32,
    FixedPoint64,
}

/// The framework type `module_id::name` is, if any.
fn framework_type(module_id: InternedModuleId, name: InternedIdentifier) -> Option<Framework> {
    let module_id = view_module_id(module_id);
    if *module_id.address() != AccountAddress::ONE {
        return None;
    }
    match (view_name(module_id.name()), view_name(name)) {
        ("string", "String") => Some(Framework::String),
        ("option", "Option") => Some(Framework::Option),
        ("object", "Object") => Some(Framework::Object),
        ("fixed_point32", "FixedPoint32") => Some(Framework::FixedPoint32),
        ("fixed_point64", "FixedPoint64") => Some(Framework::FixedPoint64),
        _ => None,
    }
}

/// Checks types and values against the transaction's own interpreter, which
/// loads the modules they name.
struct ArgChecker<'w, 'g> {
    guard: &'w ExecutionGuard<'g>,
    interp: &'w mut InterpreterContext<'g>,
    /// The `ObjectCore` type, interned on first use.
    object_core: Option<InternedType>,
}

impl ArgChecker<'_, '_> {
    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    /// Whether a transaction argument can fill a parameter of type `ty`.
    fn is_allowed(&mut self, ty: InternedType) -> Result<bool, MoveExecutionFailure> {
        match view_type(ty) {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address => Ok(true),
            Type::Vector { elem } => self.is_allowed(*elem),
            Type::Nominal {
                module_id,
                name,
                ty_args,
            } => match framework_type(*module_id, *name) {
                Some(Framework::Option) => self.is_allowed(type_arg(*ty_args)?),
                // An `Object<T>` argument is only an address, so `T` is unrestricted.
                Some(
                    Framework::String
                    | Framework::Object
                    | Framework::FixedPoint32
                    | Framework::FixedPoint64,
                ) => Ok(true),
                None => self.is_public_struct(*module_id, *name, *ty_args),
            },
            Type::Signer
            | Type::ImmutRef { .. }
            | Type::MutRef { .. }
            | Type::Function { .. }
            | Type::TypeParam { .. } => Ok(false),
        }
    }

    /// Whether `S<A..>` is a public struct or enum a transaction may construct.
    /// - Its definition declares `copy` and not `key`.
    /// - Its module has a public pack function for it, or one for every
    ///   variant.
    /// - Its field types, instantiated with `A..`, are themselves allowed.
    fn is_public_struct(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> Result<bool, MoveExecutionFailure> {
        let module = self
            .interp
            .load_module(module_id)
            .map_err(MoveExecutionFailure::RuntimeError)?;
        let Some(handle) = module.nominal_handle(module_id, name) else {
            return Ok(false);
        };
        if !handle.abilities.has_copy() || handle.abilities.has_key() {
            return Ok(false);
        }
        let Some(def_idx) = module.interned_nominal_type_def_idx(name) else {
            return Ok(false);
        };
        let struct_name = view_name(name);
        let has_pack_fns = match &module.struct_def_at(def_idx).field_information {
            StructFieldInformation::Declared(_) => has_public_pack_fn(
                module,
                &format!("{PACK}{DOLLAR_SIGN_DELIMITER}{struct_name}"),
                FunctionAttribute::Pack,
            ),
            StructFieldInformation::DeclaredVariants(variants) => {
                variants.iter().enumerate().all(|(tag, variant)| {
                    let variant_name = module.identifier_at(variant.name);
                    has_public_pack_fn(
                        module,
                        &format!(
                            "{PACK}{DOLLAR_SIGN_DELIMITER}{struct_name}{DOLLAR_SIGN_DELIMITER}{variant_name}"
                        ),
                        FunctionAttribute::PackVariant(tag as VariantIndex),
                    )
                })
            },
            StructFieldInformation::Native => false,
        };
        if !has_pack_fns {
            return Ok(false);
        }
        let fields: Vec<InternedType> = match declared_fields(module, name)? {
            FieldTypes::Struct(fields) => fields.clone(),
            FieldTypes::Enum(variants) => variants.iter().flatten().copied().collect(),
        };
        for ty in fields {
            if !self.is_allowed(self.subst(ty, ty_args)?)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `ty` with the type parameters replaced by `ty_args`.
    fn subst(
        &self,
        ty: InternedType,
        ty_args: InternedTypeList,
    ) -> Result<InternedType, MoveExecutionFailure> {
        self.guard
            .subst_type(ty, ty_args)
            .map_err(|err| MoveExecutionFailure::RuntimeError(invariant_violation(err.to_string())))
    }

    // -----------------------------------------------------------------------
    // Values
    // -----------------------------------------------------------------------

    /// Walks one value of type `ty`, checking what it holds.
    fn walk(&mut self, w: &mut Walk<'_>, ty: InternedType) -> Result<(), MoveExecutionFailure> {
        if let Some(n) = primitive_size(ty) {
            return w.skip(n);
        }
        match view_type(ty) {
            Type::Vector { elem } => {
                let len = w.read_len()?;
                if let Some(size) = primitive_size(*elem) {
                    let bytes = len.checked_mul(size).ok_or_else(undecodable)?;
                    return w.skip(bytes);
                }
                for _ in 0..len {
                    self.walk(w, *elem)?;
                }
                Ok(())
            },
            Type::Nominal {
                module_id,
                name,
                ty_args,
            } => match framework_type(*module_id, *name) {
                Some(Framework::String) => {
                    let len = w.read_len()?;
                    if std::str::from_utf8(w.read(len)?).is_err() {
                        return Err(MoveExecutionFailure::InvalidArguments(
                            InvalidArguments::MalformedString,
                        ));
                    }
                    Ok(())
                },
                Some(Framework::Option) => match w.read_len()? {
                    0 => Ok(()),
                    1 => self.walk(w, type_arg(*ty_args)?),
                    _ => Err(undecodable()),
                },
                Some(Framework::Object) => {
                    let address = AccountAddress::from_bytes(w.read(AccountAddress::LENGTH)?)
                        .map_err(|err| {
                            MoveExecutionFailure::RuntimeError(invariant_violation(err.to_string()))
                        })?;
                    self.check_object(w, address, type_arg(*ty_args)?)
                },
                Some(Framework::FixedPoint32) => w.skip(8),
                Some(Framework::FixedPoint64) => w.skip(16),
                None => {
                    let module = self
                        .interp
                        .load_module(*module_id)
                        .map_err(MoveExecutionFailure::RuntimeError)?;
                    let fields = match declared_fields(module, *name)? {
                        FieldTypes::Struct(fields) => fields,
                        FieldTypes::Enum(variants) => {
                            variants.get(w.read_len()?).ok_or_else(undecodable)?
                        },
                    };
                    for &ty in fields {
                        let ty = self.subst(ty, *ty_args)?;
                        self.walk(w, ty)?;
                    }
                    Ok(())
                },
            },
            // Primitives were skipped above; the type check refuses the rest.
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address
            | Type::Signer
            | Type::ImmutRef { .. }
            | Type::MutRef { .. }
            | Type::Function { .. }
            | Type::TypeParam { .. } => Err(MoveExecutionFailure::RuntimeError(
                invariant_violation("argument of a type a transaction cannot fill"),
            )),
        }
    }

    /// Checks that `address` holds an object and a `resource`, like
    /// `object::address_to_object` does.
    fn check_object(
        &mut self,
        w: &mut Walk<'_>,
        address: AccountAddress,
        resource: InternedType,
    ) -> Result<(), MoveExecutionFailure> {
        if w.object_checks == MAX_OBJECT_CHECKS_PER_ARG {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::TooManyObjectChecks,
            ));
        }
        w.object_checks += 1;
        let object_core = self.object_core()?;
        // TODO(metering): charge for the two reads.
        if !self
            .interp
            .resource_exists(address, object_core)
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

    fn object_core(&mut self) -> Result<InternedType, MoveExecutionFailure> {
        if let Some(ty) = self.object_core {
            return Ok(ty);
        }
        let tag = TypeTag::Struct(Box::new(ObjectCoreResource::struct_tag()));
        let ty = intern_type_tag(&tag, self.guard).map_err(|err| {
            MoveExecutionFailure::RuntimeError(invariant_violation(err.to_string()))
        })?;
        self.object_core = Some(ty);
        Ok(ty)
    }
}

/// The declared field types of `name`, defined in `module`.
fn declared_fields(
    module: &PreparedModule,
    name: InternedIdentifier,
) -> Result<&FieldTypes, MoveExecutionFailure> {
    module.interned_field_types(name).ok_or_else(|| {
        MoveExecutionFailure::RuntimeError(invariant_violation(
            "nominal type without a definition in its module",
        ))
    })
}

/// Whether `module` defines a public function `name` carrying `attribute`.
fn has_public_pack_fn(module: &PreparedModule, name: &str, attribute: FunctionAttribute) -> bool {
    module.function_defs().iter().any(|def| {
        let handle = module.function_handle_at(def.function);
        def.visibility == Visibility::Public
            && module.identifier_at(handle.name).as_str() == name
            && handle.attributes.contains(&attribute)
    })
}

/// The cursor over one argument's bytes.
struct Walk<'b> {
    bytes: &'b [u8],
    cursor: usize,
    object_checks: usize,
}

impl<'b> Walk<'b> {
    /// Reads a BCS length or enum tag.
    fn read_len(&mut self) -> Result<usize, MoveExecutionFailure> {
        let len = read_uleb128_len(self.bytes, &mut self.cursor).map_err(decode_error)?;
        if len > bcs::MAX_SEQUENCE_LENGTH as u64 {
            return Err(undecodable());
        }
        Ok(len as usize)
    }

    fn read(&mut self, n: usize) -> Result<&'b [u8], MoveExecutionFailure> {
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
